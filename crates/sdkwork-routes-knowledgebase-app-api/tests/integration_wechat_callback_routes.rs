use async_trait::async_trait;
use aes::Aes256;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use cbc::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use sdkwork_intelligence_knowledgebase_service::wechat::{
    KnowledgeWechatService, WechatApiClient, WechatCallbackReceipt, WechatCallbackReceiptRequest,
    WechatCallbackService, WechatCallbackVerificationRequest, callback_signature,
};
use sdkwork_knowledgebase_contract::wechat::KnowledgeWechatOfficialAccount;
use sdkwork_knowledgebase_test_support::fake_drive::FakeKnowledgeDriveStorage;
use sdkwork_routes_knowledgebase_app_api::{build_router_with_app_api, paths, ApiError, ApiResult, KnowledgeAppApi};
use std::sync::Arc;
use tower::util::ServiceExt;

const TEST_TENANT_ID: u64 = 1;
const TEST_ACCOUNT_ID: &str = "acct-callback";
const TEST_TOKEN: &str = "know-callback-token";
const TEST_AES_KEY_43: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ";
const TEST_APP_ID: &str = "wx-callback-appid";
const MAX_TEST_RESPONSE_BYTES: usize = 1024 * 1024;

type Aes256CbcEnc = cbc::Encryptor<Aes256>;

#[derive(Default)]
struct TestCallbackApi {
    drive: FakeKnowledgeDriveStorage,
}

impl TestCallbackApi {
    fn callback_service(&self) -> WechatCallbackService<'_> {
        WechatCallbackService::new(&self.drive, "1")
    }
}

#[async_trait]
impl KnowledgeAppApi for TestCallbackApi {
    async fn verify_wechat_callback(
        &self,
        request: WechatCallbackVerificationRequest,
    ) -> ApiResult<String> {
        self.callback_service()
            .verify_url(&request)
            .await
            .map_err(ApiError::from)
    }

    async fn receive_wechat_callback(
        &self,
        request: WechatCallbackReceiptRequest,
    ) -> ApiResult<WechatCallbackReceipt> {
        self.callback_service()
            .receive_message(&request)
            .await
            .map_err(ApiError::from)
    }
}

fn safe_official_account() -> KnowledgeWechatOfficialAccount {
    KnowledgeWechatOfficialAccount {
        id: TEST_ACCOUNT_ID.to_string(),
        name: "Callback Account".to_string(),
        account_type: "service".to_string(),
        avatar: "CA".to_string(),
        description: None,
        app_id: TEST_APP_ID.to_string(),
        app_secret: None,
        server_url: Some("https://knowledgebase.example.com/app/v3/api/knowledge/wechat/callback".to_string()),
        token: Some(TEST_TOKEN.to_string()),
        encoding_aes_key: Some(TEST_AES_KEY_43.to_string()),
        encrypt_mode: Some("safe".to_string()),
        domain_verify_file_name: None,
        domain_verify_file_content: None,
        js_secure_domains: None,
        web_auth_domains: None,
        business_domains: None,
        group: None,
    }
}

fn test_aes_key32() -> [u8; 32] {
    // 43 chars + the standard "=" pad decodes to the 32-byte WeChat key.
    let mut key = TEST_AES_KEY_43.to_string();
    key.push('=');
    BASE64_STANDARD
        .decode(key.as_bytes())
        .expect("test aes key")
        .try_into()
        .expect("32 byte key")
}

fn encrypt_wechat_blob(message: &str, receive_id: &str) -> String {
    let key = test_aes_key32();
    let mut plain = vec![0u8; 16];
    plain.extend_from_slice(&(message.len() as u32).to_be_bytes());
    plain.extend_from_slice(message.as_bytes());
    plain.extend_from_slice(receive_id.as_bytes());
    let mut padded = vec![0u8; plain.len() + 16];
    padded[..plain.len()].copy_from_slice(&plain);
    let ciphertext = Aes256CbcEnc::new_from_slices(&key, &key[..16])
        .expect("test cipher key")
        .encrypt_padded_mut::<Pkcs7>(&mut padded, plain.len())
        .expect("test padding")
        .to_vec();
    BASE64_STANDARD.encode(ciphertext)
}

fn query_encode(value: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("v", value)
        .finish()
        .trim_start_matches("v=")
        .to_string()
}

fn signed_callback_params(payload: &str, nonce: &str) -> (String, String, String) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("test clock")
        .as_secs()
        .to_string();
    let signature = callback_signature(TEST_TOKEN, &timestamp, nonce, payload);
    (signature, timestamp, nonce.to_string())
}

fn callback_uri(extra: &str) -> String {
    format!(
        "{}?tenant={TEST_TENANT_ID}&account_id={TEST_ACCOUNT_ID}{extra}",
        paths::WECHAT_CALLBACK
    )
}

#[tokio::test]
async fn callback_get_echo_returns_decrypted_echo_string() {
    let _env_guard = WechatCallbackEnvGuard::with_test_secret_key();
    let mut api = TestCallbackApi::default();
    seed_account(&mut api).await;
    let app = build_router_with_app_api(api);

    let echo = "random-echo-7381";
    let echostr = encrypt_wechat_blob(echo, TEST_APP_ID);
    let (signature, timestamp, nonce) = signed_callback_params(&echostr, "verify-nonce");
    let uri = callback_uri(&format!(
        "&signature={}&timestamp={timestamp}&nonce={nonce}&echostr={}",
        query_encode(&signature),
        query_encode(&echostr)
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(response_body_string(response).await, echo);
}

#[tokio::test]
async fn callback_get_echo_rejects_bad_signature_with_problem_json() {
    let _env_guard = WechatCallbackEnvGuard::with_test_secret_key();
    let mut api = TestCallbackApi::default();
    seed_account(&mut api).await;
    let app = build_router_with_app_api(api);

    let echostr = encrypt_wechat_blob("rejected-echo", TEST_APP_ID);
    let (_, timestamp, nonce) = signed_callback_params("wrong-payload", "verify-nonce");
    let uri = callback_uri(&format!(
        "&signature={}&timestamp={timestamp}&nonce={nonce}&echostr={}",
        query_encode(&"0".repeat(40)),
        query_encode(&echostr)
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_problem(response, StatusCode::UNAUTHORIZED, 40101).await;
}

#[tokio::test]
async fn callback_post_receipt_acks_verified_message_and_persists_artifact() {
    let _env_guard = WechatCallbackEnvGuard::with_test_secret_key();
    let mut api = TestCallbackApi::default();
    seed_account(&mut api).await;
    let app = build_router_with_app_api(api);

    let message_xml = format!(
        "<xml><ToUserName><![CDATA[{TEST_APP_ID}]]></ToUserName>\
<FromUserName><![CDATA[fan-openid]]></FromUserName>\
<CreateTime>1700000000</CreateTime>\
<MsgType><![CDATA[text]]></MsgType>\
<MsgId>9007199254740993</MsgId></xml>"
    );
    let encrypt = encrypt_wechat_blob(&message_xml, TEST_APP_ID);
    let body = format!("<xml><Encrypt><![CDATA[{encrypt}]]></Encrypt></xml>");
    let (signature, timestamp, nonce) = signed_callback_params(&encrypt, "receipt-nonce");
    let uri = callback_uri(&format!(
        "&msg_signature={}&timestamp={timestamp}&nonce={nonce}",
        query_encode(&signature)
    ));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::CONTENT_TYPE, "text/xml")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(response_body_string(response).await, "success");

    // WeChat retransmissions stay acknowledged instead of accumulating artifacts.
    let encrypt_again = encrypt_wechat_blob(&message_xml, TEST_APP_ID);
    let body_again = format!("<xml><Encrypt><![CDATA[{encrypt_again}]]></Encrypt></xml>");
    let (signature_again, timestamp_again, nonce_again) =
        signed_callback_params(&encrypt_again, "receipt-nonce-2");
    let uri_again = callback_uri(&format!(
        "&msg_signature={}&timestamp={timestamp_again}&nonce={nonce_again}",
        query_encode(&signature_again)
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri_again)
                .header(header::CONTENT_TYPE, "text/xml")
                .body(Body::from(body_again))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn callback_post_receipt_rejects_unverified_payload_and_unknown_tenant() {
    let _env_guard = WechatCallbackEnvGuard::with_test_secret_key();
    let mut api = TestCallbackApi::default();
    seed_account(&mut api).await;
    let app = build_router_with_app_api(api);

    // A payload signed with the wrong material must not be acked.
    let message_xml = "<xml><MsgType>text</MsgType></xml>";
    let encrypt = encrypt_wechat_blob(message_xml, TEST_APP_ID);
    let (_, timestamp, nonce) = signed_callback_params("tampered", "receipt-nonce");
    let uri = callback_uri(&format!(
        "&msg_signature={}&timestamp={timestamp}&nonce={nonce}",
        query_encode(&"0".repeat(40))
    ));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::CONTENT_TYPE, "text/xml")
                .body(Body::from(format!(
                    "<xml><Encrypt><![CDATA[{encrypt}]]></Encrypt></xml>"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_problem(response, StatusCode::UNAUTHORIZED, 40101).await;

    // A tenant query param outside this deployment is rejected before any lookup.
    let (signature, timestamp, nonce) = signed_callback_params(&encrypt, "receipt-nonce");
    let uri = format!(
        "{}?tenant=999&account_id={TEST_ACCOUNT_ID}&msg_signature={}&timestamp={timestamp}&nonce={nonce}",
        paths::WECHAT_CALLBACK,
        query_encode(&signature)
    );
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::CONTENT_TYPE, "text/xml")
                .body(Body::from(format!(
                    "<xml><Encrypt><![CDATA[{encrypt}]]></Encrypt></xml>"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

async fn seed_account(api: &mut TestCallbackApi) {
    let service = KnowledgeWechatService::new(&api.drive, "1", Arc::new(WechatApiClient::new()));
    service
        .replace_official_accounts(vec![safe_official_account()])
        .await
        .expect("seed callback account");
}

static WECHAT_CALLBACK_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct WechatCallbackEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    previous_secret_key: Option<String>,
    previous_secret_key_file: Option<String>,
}

impl WechatCallbackEnvGuard {
    fn with_test_secret_key() -> Self {
        let lock = WECHAT_CALLBACK_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_secret_key =
            std::env::var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY").ok();
        let previous_secret_key_file =
            std::env::var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE").ok();
        std::env::set_var(
            "SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY",
            "sdkwork-knowledgebase-wechat-callback-integration-test-key",
        );
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE");
        Self {
            _lock: lock,
            previous_secret_key,
            previous_secret_key_file,
        }
    }
}

impl Drop for WechatCallbackEnvGuard {
    fn drop(&mut self) {
        match self.previous_secret_key.as_deref() {
            Some(value) => std::env::set_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY", value),
            None => std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY"),
        }
        match self.previous_secret_key_file.as_deref() {
            Some(value) => {
                std::env::set_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE", value)
            }
            None => std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE"),
        }
    }
}

async fn assert_problem(
    response: axum::response::Response,
    expected_status: StatusCode,
    expected_code: i64,
) {
    assert_eq!(response.status(), expected_status);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/problem+json")
    );
    let problem: serde_json::Value = serde_json::from_str(&response_body_string(response).await)
        .expect("parse problem response json");
    assert_eq!(
        problem["status"].as_u64(),
        Some(expected_status.as_u16().into())
    );
    assert_eq!(problem["code"].as_i64(), Some(expected_code));
    uuid::Uuid::parse_str(problem["traceId"].as_str().expect("problem traceId"))
        .expect("problem traceId UUID");
}

async fn response_body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), MAX_TEST_RESPONSE_BYTES)
        .await
        .expect("read response body");
    String::from_utf8(bytes.to_vec()).expect("utf8 response body")
}
