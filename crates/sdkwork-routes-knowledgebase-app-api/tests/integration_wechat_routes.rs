use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use sdkwork_intelligence_knowledgebase_service::wechat::{KnowledgeWechatService, WechatApiClient};
use sdkwork_knowledgebase_contract::wechat::{
    KnowledgeWechatApplet, KnowledgeWechatAppletList, KnowledgeWechatArticlesPublishRequest,
    KnowledgeWechatOfficialAccount, KnowledgeWechatOfficialAccountList,
    KnowledgeWechatOperationResult, KnowledgeWechatReplaceAppletsRequest,
    KnowledgeWechatReplaceOfficialAccountsRequest,
};
use sdkwork_knowledgebase_test_support::fake_drive::FakeKnowledgeDriveStorage;
use sdkwork_routes_knowledgebase_app_api::{
    build_router_with_app_api, dev_auth, paths, ApiError, ApiResult, KnowledgeAppApi,
    KnowledgeAppRequestContext,
};
use serde_json::json;
use std::sync::{Arc, Mutex, MutexGuard};
use tower::util::ServiceExt;

const TEST_TENANT_ID: u64 = 1;
const TEST_ACTOR_ID: u64 = 42;
const MAX_TEST_RESPONSE_BYTES: usize = 1024 * 1024;

#[tokio::test]
async fn integration_wechat_official_accounts_replace_redacts_secrets_on_list() {
    let _env_guard = WechatIntegrationEnvGuard::with_test_secret_key();
    let app = test_app();

    let replace_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(paths::WECHAT_OFFICIAL_ACCOUNTS)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "accounts": [{
                            "id": "acct-1",
                            "name": "Test Account",
                            "type": "subscription",
                            "avatar": "TA",
                            "appId": "wx-test-app-id",
                            "appSecret": "super-secret-value"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        replace_response.status(),
        StatusCode::OK,
        "replace failed: {}",
        response_body_string(replace_response).await
    );
    let replace_body = response_body_json(replace_response).await;
    assert_eq!(replace_body["accounts"][0]["id"], "acct-1");
    assert!(replace_body["accounts"][0]["appSecret"].is_null());

    let list_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(paths::WECHAT_OFFICIAL_ACCOUNTS)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = response_body_json(list_response).await;
    assert_eq!(list_body["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(list_body["pageInfo"]["hasMore"], json!(false));
    assert!(list_body["items"][0]["appSecret"].is_null());
}

#[tokio::test]
async fn integration_wechat_config_rejects_invalid_input_without_overwrite() {
    let _env_guard = WechatIntegrationEnvGuard::with_test_secret_key();
    let app = test_app();

    let seed_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(paths::WECHAT_OFFICIAL_ACCOUNTS)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "accounts": [{
                            "id": "stable-account",
                            "name": "Stable Account",
                            "type": "subscription",
                            "avatar": "SA",
                            "appId": "wx-stable"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(seed_response.status(), StatusCode::OK);

    for invalid_body in [
        json!({
            "accounts": [{
                "id": "unknown-field",
                "name": "Unknown Field",
                "type": "subscription",
                "avatar": "UF",
                "appId": "wx-unknown",
                "unexpected": true
            }]
        }),
        json!({
            "accounts": [{
                "id": "media-avatar",
                "name": "Media Avatar",
                "type": "subscription",
                "avatar": "data:image/png;base64,AAAA",
                "appId": "wx-media"
            }]
        }),
        json!({
            "accounts": [{
                "id": "unsupported-enum",
                "name": "Unsupported Enum",
                "type": "enterprise",
                "avatar": "UE",
                "appId": "wx-enum"
            }]
        }),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(paths::WECHAT_OFFICIAL_ACCOUNTS)
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_problem(response, StatusCode::BAD_REQUEST, 40001).await;
    }

    let applet_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(paths::WECHAT_APPLETS)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "applets": [{
                            "id": "invalid-applet",
                            "name": "Invalid Applet",
                            "appId": "wx-invalid-applet",
                            "path": "pages/index",
                            "avatar": "IA",
                            "msgDataFormat": "yaml"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_problem(applet_response, StatusCode::BAD_REQUEST, 40001).await;

    let list_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(paths::WECHAT_OFFICIAL_ACCOUNTS)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = response_body_json(list_response).await;
    assert_eq!(list_body["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(list_body["items"][0]["id"], "stable-account");
}

#[tokio::test]
async fn integration_wechat_publish_rejects_missing_managed_cover_before_upstream_io() {
    let _env_guard = WechatIntegrationEnvGuard::with_test_secret_key();
    let app = test_app();

    let replace_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(paths::WECHAT_OFFICIAL_ACCOUNTS)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "accounts": [{
                            "id": "acct-2",
                            "name": "No Secret",
                            "type": "subscription",
                            "avatar": "NS",
                            "appId": "wx-no-secret"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replace_response.status(), StatusCode::OK);

    let publish_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(paths::WECHAT_ARTICLES_PUBLISH)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "accountIds": ["acct-2"],
                        "articles": [{
                            "id": "article-1",
                            "title": "Title",
                            "author": "Author",
                            "content": "<p>Hello</p>"
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_problem(publish_response, StatusCode::NOT_IMPLEMENTED, 50001).await;
}

#[derive(Default)]
struct TestWechatApi {
    drive: FakeKnowledgeDriveStorage,
    /// Shared client, matching the production wiring so the token cache survives per call.
    api_client: Arc<WechatApiClient>,
}

impl TestWechatApi {
    fn service(&self) -> KnowledgeWechatService<'_> {
        KnowledgeWechatService::new(&self.drive, "tenant-1", Arc::clone(&self.api_client))
    }
}

#[async_trait]
impl KnowledgeAppApi for TestWechatApi {
    async fn list_wechat_official_accounts(
        &self,
        _context: KnowledgeAppRequestContext,
    ) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeWechatOfficialAccount>> {
        let accounts = self
            .service()
            .list_official_accounts()
            .await
            .map_err(ApiError::from)?;
        Ok(sdkwork_routes_knowledgebase_app_api::pagination::fixed_list_page_data(
            accounts,
        ))
    }

    async fn replace_wechat_official_accounts(
        &self,
        _context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceOfficialAccountsRequest,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList> {
        let accounts = self
            .service()
            .replace_official_accounts(request.accounts)
            .await
            .map_err(ApiError::from)?;
        Ok(KnowledgeWechatOfficialAccountList { accounts })
    }

    async fn list_wechat_applets(
        &self,
        _context: KnowledgeAppRequestContext,
    ) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeWechatApplet>> {
        let applets = self
            .service()
            .list_applets()
            .await
            .map_err(ApiError::from)?;
        Ok(sdkwork_routes_knowledgebase_app_api::pagination::fixed_list_page_data(
            applets,
        ))
    }

    async fn replace_wechat_applets(
        &self,
        _context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceAppletsRequest,
    ) -> ApiResult<KnowledgeWechatAppletList> {
        let applets = self
            .service()
            .replace_applets(request.applets)
            .await
            .map_err(ApiError::from)?;
        Ok(KnowledgeWechatAppletList { applets })
    }

    async fn publish_wechat_articles(
        &self,
        _context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPublishRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        self.service()
            .publish_articles(request)
            .await
            .map_err(ApiError::from)
    }
}

fn test_app() -> axum::Router {
    dev_auth::with_dev_app_auth(
        build_router_with_app_api(TestWechatApi::default()),
        TEST_TENANT_ID,
        Some(TEST_ACTOR_ID),
    )
}

static WECHAT_INTEGRATION_ENV_LOCK: Mutex<()> = Mutex::new(());

struct WechatIntegrationEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous_secret_key: Option<String>,
    previous_secret_key_file: Option<String>,
}

impl WechatIntegrationEnvGuard {
    fn with_test_secret_key() -> Self {
        let lock = WECHAT_INTEGRATION_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_secret_key =
            std::env::var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY").ok();
        let previous_secret_key_file =
            std::env::var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE").ok();

        std::env::set_var(
            "SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY",
            "sdkwork-knowledgebase-wechat-integration-test-secret-key",
        );
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE");

        Self {
            _lock: lock,
            previous_secret_key,
            previous_secret_key_file,
        }
    }
}

impl Drop for WechatIntegrationEnvGuard {
    fn drop(&mut self) {
        restore_env_var(
            "SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY",
            self.previous_secret_key.as_deref(),
        );
        restore_env_var(
            "SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE",
            self.previous_secret_key_file.as_deref(),
        );
    }
}

fn restore_env_var(key: &str, value: Option<&str>) {
    match value {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
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
    let trace_id = problem["traceId"].as_str().expect("problem traceId");
    uuid::Uuid::parse_str(trace_id).expect("problem traceId UUID");
}

async fn response_body_json(response: axum::response::Response) -> serde_json::Value {
    let text = response_body_string(response).await;
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse response json");
    sdkwork_knowledgebase_test_support::api_envelope::unwrap_payload_or_envelope(&value)
}

async fn response_body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), MAX_TEST_RESPONSE_BYTES)
        .await
        .expect("read response body");
    String::from_utf8(bytes.to_vec()).expect("utf8 response body")
}
