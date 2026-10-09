use crate::bounded_http_body::{
    read_bounded_http_body, redacted_reqwest_error_detail, BoundedHttpBodyError,
};
use reqwest::Client;
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;

const WECHAT_API_HOST: &str = "api.weixin.qq.com";
const WECHAT_API_TIMEOUT_SECS: u64 = 30;
const MAX_WECHAT_JSON_RESPONSE_BYTES: usize = 512 * 1024;
/// WeChat access tokens are valid for 7,200 s; cache them for 7,000 s so a token is
/// refreshed 200 s before expiry instead of being re-fetched per operation (the token
/// endpoint is rate-limited to roughly 2,000 calls/day).
const WECHAT_TOKEN_CACHE_TTL_SECS: u64 = 7_000;
/// Bounded retries for rate-limited or transient token requests. The WeChat token
/// endpoint is strictly rate-limited, so retries must stay small and back off.
const WECHAT_TOKEN_MAX_RETRIES: u32 = 2;
/// WeChat error codes that indicate rate limiting or transient upstream load.
const WECHAT_RETRYABLE_ERROR_CODES: &[&str] = &["-1", "45009", "45002"];
/// WeChat error codes that mean the presented access token is invalid or expired. The
/// operation is retried exactly once after a forced token refresh; anything else fails.
const WECHAT_TOKEN_ERROR_CODES: &[i64] = &[40001, 40014, 41001, 42001];

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    expires_in: Option<i64>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    tags: Option<Vec<TagEntry>>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TagEntry {
    id: Option<i64>,
    name: Option<String>,
    count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MaterialUploadResponse {
    media_id: Option<String>,
    url: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DraftAddResponse {
    media_id: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FreepublishSubmitResponse {
    publish_id: Option<String>,
    errcode: Option<i64>,
    errmsg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatUserTag {
    pub id: i64,
    pub name: String,
    pub count: u64,
}

/// A permanent image material uploaded through `cgi-bin/material/add_material`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatUploadedMaterial {
    pub media_id: String,
    pub url: Option<String>,
}

/// One article inside a WeChat draft (`cgi-bin/draft/add`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatDraftArticle {
    pub title: String,
    pub author: String,
    pub digest: Option<String>,
    pub content: String,
    /// Free-text source link rendered under the published article; the current
    /// publish contract has no field for it and passes `None`.
    pub content_source_url: Option<String>,
    /// Cover thumbnail as a media id already uploaded for the same app id. WeChat
    /// media ids are app-scoped, so callers must not reuse one across accounts.
    pub thumb_media_id: Option<String>,
}

/// Wire shape of one entry in the `cgi-bin/draft/add` `articles` array.
#[derive(Debug, Serialize)]
struct WechatDraftArticlePayload<'a> {
    title: &'a str,
    author: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<&'a str>,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_source_url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb_media_id: Option<&'a str>,
    need_open_comment: u8,
    only_fans_can_comment: u8,
}

impl<'a> From<&'a WechatDraftArticle> for WechatDraftArticlePayload<'a> {
    fn from(article: &'a WechatDraftArticle) -> Self {
        Self {
            title: article.title.as_str(),
            author: article.author.as_str(),
            digest: article.digest.as_deref(),
            content: article.content.as_str(),
            content_source_url: article.content_source_url.as_deref(),
            thumb_media_id: article.thumb_media_id.as_deref(),
            need_open_comment: 0,
            only_fans_can_comment: 0,
        }
    }
}

pub struct WechatApiClient {
    http: Result<Client, String>,
    /// `None` pins every request to the allowlisted production host; embedded harnesses
    /// and integration tests inject a base URL so the wire can be intercepted locally.
    api_base_override: Option<Url>,
    token_cache: Mutex<HashMap<String, (String, Instant)>>,
    /// Per-appid single-flight refresh locks. The std map guard is only held to clone the Arc
    /// out (never across an await); the tokio mutex is what concurrent misses serialize on.
    token_refresh_locks: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

impl Default for WechatApiClient {
    fn default() -> Self {
        Self {
            http: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(WECHAT_API_TIMEOUT_SECS))
                .build()
                .map_err(|error| redacted_reqwest_error_detail(&error)),
            api_base_override: None,
            token_cache: Mutex::new(HashMap::new()),
            token_refresh_locks: Mutex::new(HashMap::new()),
        }
    }
}

impl WechatApiClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Routes WeChat API calls at `api_base` instead of the allowlisted production host.
    /// Only embedded harnesses and integration tests should inject a base; application
    /// wiring must use [`WechatApiClient::new`].
    pub fn with_api_base_override(api_base: Url) -> Self {
        Self {
            api_base_override: Some(api_base),
            ..Self::default()
        }
    }

    fn http(&self) -> Result<&Client, WechatApiClientError> {
        self.http
            .as_ref()
            .map_err(|detail| WechatApiClientError::Configuration(detail.clone()))
    }

    /// Returns a cached, still-valid access token when present, otherwise fetches a fresh one
    /// with bounded retries and caches it. Concurrent misses for the same appid share a single
    /// in-flight refresh so one cold start burns one token quota unit, not one per request.
    pub async fn fetch_access_token(
        &self,
        app_id: &str,
        app_secret: &str,
    ) -> Result<String, WechatApiClientError> {
        if let Some(token) = self.cached_access_token(app_id) {
            return Ok(token);
        }

        let refresh_lock = self.token_refresh_lock(app_id);
        let _refresh_guard = refresh_lock.lock().await;
        // Double-check: another request may have completed the refresh while this one waited
        // on the single-flight lock.
        if let Some(token) = self.cached_access_token(app_id) {
            return Ok(token);
        }

        let mut last_error = WechatApiClientError::Api("wechat token request failed".to_string());
        for attempt in 0..=WECHAT_TOKEN_MAX_RETRIES {
            match self.request_access_token(app_id, app_secret).await {
                Ok((token, ttl_secs)) => {
                    self.cache_access_token(app_id, token.clone(), ttl_secs);
                    return Ok(token);
                }
                Err(error) if attempt < WECHAT_TOKEN_MAX_RETRIES && is_retryable(&error) => {
                    last_error = error;
                    tokio::time::sleep(backoff_for_attempt(attempt)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error)
    }

    fn token_refresh_lock(&self, app_id: &str) -> Arc<AsyncMutex<()>> {
        let mut locks = self
            .token_refresh_locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Arc::clone(locks.entry(app_id.to_string()).or_default())
    }

    /// Drops any cached access token for `app_id` so the next fetch hits the token endpoint
    /// again. Used after a 40001-class rejection, before the single refresh-and-retry cycle.
    fn invalidate_access_token(&self, app_id: &str) {
        if let Ok(mut cache) = self.token_cache.lock() {
            cache.remove(app_id);
        }
    }

    /// Builds a WeChat API URL against the allowlisted production host, or against the
    /// injected base when the client was created through `with_api_base_override`.
    fn wechat_url(&self, path_and_query: &str) -> Result<Url, WechatApiClientError> {
        match self.api_base_override.as_ref() {
            Some(base) => base
                .join(path_and_query.trim_start_matches('/'))
                .map_err(|_| {
                    WechatApiClientError::InvalidRequest(
                        "failed to construct wechat api url".to_string(),
                    )
                }),
            None => build_wechat_url(path_and_query),
        }
    }

    fn cached_access_token(&self, app_id: &str) -> Option<String> {
        let cache = self.token_cache.lock().ok()?;
        let (token, expires_at) = cache.get(app_id)?;
        if *expires_at > Instant::now() {
            Some(token.clone())
        } else {
            None
        }
    }

    fn cache_access_token(&self, app_id: &str, token: String, ttl_secs: u64) {
        if let Ok(mut cache) = self.token_cache.lock() {
            cache.insert(
                app_id.to_string(),
                (token, Instant::now() + Duration::from_secs(ttl_secs)),
            );
        }
    }

    async fn request_access_token(
        &self,
        app_id: &str,
        app_secret: &str,
    ) -> Result<(String, u64), WechatApiClientError> {
        let url = self.wechat_url(&format!(
            "/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",
            urlencoding::encode(app_id),
            urlencoding::encode(app_secret),
        ))?;
        let response = self
            .http()?
            .get(url)
            .send()
            .await
            .map_err(redacted_reqwest_error)?;
        let body: AccessTokenResponse = parse_wechat_json(response).await?;
        if let Some(token) = body.access_token.filter(|value| !value.is_empty()) {
            // Respect a server-declared shorter lifetime; refresh 200 s early and never
            // cache longer than the conservative TTL.
            let ttl = body
                .expires_in
                .filter(|seconds| *seconds > 0)
                .map(|seconds| (seconds as u64).saturating_sub(200).max(60))
                .unwrap_or(WECHAT_TOKEN_CACHE_TTL_SECS)
                .min(WECHAT_TOKEN_CACHE_TTL_SECS);
            return Ok((token, ttl));
        }
        Err(WechatApiClientError::Api(format!(
            "wechat token request failed with code {}: {}",
            body.errcode.unwrap_or(0),
            body.errmsg
                .unwrap_or_else(|| "no upstream detail".to_string()),
        )))
    }

    pub async fn list_user_tags(
        &self,
        access_token: &str,
    ) -> Result<Vec<WechatUserTag>, WechatApiClientError> {
        let url = self.wechat_url(&format!(
            "/cgi-bin/tags/get?access_token={}",
            urlencoding::encode(access_token),
        ))?;
        let response = self
            .http()?
            .get(url)
            .send()
            .await
            .map_err(redacted_reqwest_error)?;
        let body: TagsResponse = parse_wechat_json(response).await?;
        if let Some(errcode) = body.errcode.filter(|code| *code != 0) {
            return Err(WechatApiClientError::Api(body.errmsg.unwrap_or_else(
                || format!("wechat tag list failed with code {errcode}"),
            )));
        }
        Ok(body
            .tags
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| {
                let id = entry.id?;
                let name = entry.name.filter(|value| !value.is_empty())?;
                Some(WechatUserTag {
                    id,
                    name,
                    count: entry.count.unwrap_or(0),
                })
            })
            .collect())
    }

    /// Uploads one image as a permanent material (`cgi-bin/material/add_material`,
    /// `type=image`) and returns its `media_id`/`url`. A 40001-class token rejection
    /// triggers exactly one refresh-and-retry cycle.
    pub async fn upload_permanent_image(
        &self,
        app_id: &str,
        app_secret: &str,
        file_name: &str,
        content_type: &str,
        image: &[u8],
    ) -> Result<WechatUploadedMaterial, WechatApiClientError> {
        let access_token = self.fetch_access_token(app_id, app_secret).await?;
        match self
            .request_upload_permanent_image(&access_token, file_name, content_type, image)
            .await
        {
            Err(WechatApiClientError::AccessTokenRejected) => {
                self.invalidate_access_token(app_id);
                let access_token = self.fetch_access_token(app_id, app_secret).await?;
                self.request_upload_permanent_image(&access_token, file_name, content_type, image)
                    .await
            }
            outcome => outcome,
        }
    }

    async fn request_upload_permanent_image(
        &self,
        access_token: &str,
        file_name: &str,
        content_type: &str,
        image: &[u8],
    ) -> Result<WechatUploadedMaterial, WechatApiClientError> {
        let url = self.wechat_url(&format!(
            "/cgi-bin/material/add_material?access_token={}&type=image",
            urlencoding::encode(access_token),
        ))?;
        let media = reqwest::multipart::Part::bytes(image.to_vec())
            .file_name(file_name.to_string())
            .mime_str(content_type)
            .map_err(|_| {
                WechatApiClientError::InvalidRequest(
                    "wechat material content type is invalid".to_string(),
                )
            })?;
        let form = reqwest::multipart::Form::new().part("media", media);
        let response = self
            .http()?
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(redacted_reqwest_error)?;
        let body: MaterialUploadResponse = parse_wechat_json(response).await?;
        ensure_wechat_ok("material upload", body.errcode, body.errmsg)?;
        let media_id = required_upstream_field("material upload", "media_id", body.media_id)?;
        Ok(WechatUploadedMaterial {
            media_id,
            url: body.url,
        })
    }

    /// Creates a draft from the given articles (`cgi-bin/draft/add`) and returns the
    /// draft `media_id` that `freepublish/submit` consumes. A 40001-class token
    /// rejection triggers exactly one refresh-and-retry cycle.
    pub async fn add_draft(
        &self,
        app_id: &str,
        app_secret: &str,
        articles: Vec<WechatDraftArticle>,
    ) -> Result<String, WechatApiClientError> {
        let access_token = self.fetch_access_token(app_id, app_secret).await?;
        match self.request_add_draft(&access_token, &articles).await {
            Err(WechatApiClientError::AccessTokenRejected) => {
                self.invalidate_access_token(app_id);
                let access_token = self.fetch_access_token(app_id, app_secret).await?;
                self.request_add_draft(&access_token, &articles).await
            }
            outcome => outcome,
        }
    }

    async fn request_add_draft(
        &self,
        access_token: &str,
        articles: &[WechatDraftArticle],
    ) -> Result<String, WechatApiClientError> {
        let url = self.wechat_url(&format!(
            "/cgi-bin/draft/add?access_token={}",
            urlencoding::encode(access_token),
        ))?;
        let payload = serde_json::json!({
            "articles": articles
                .iter()
                .map(|article| WechatDraftArticlePayload::from(article))
                .collect::<Vec<_>>(),
        });
        let response = self
            .http()?
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(redacted_reqwest_error)?;
        let body: DraftAddResponse = parse_wechat_json(response).await?;
        ensure_wechat_ok("draft add", body.errcode, body.errmsg)?;
        required_upstream_field("draft add", "media_id", body.media_id)
    }

    /// Submits a draft for publication (`cgi-bin/freepublish/submit`) and returns the
    /// upstream `publish_id`. Publication itself is asynchronous on WeChat's side. A
    /// 40001-class token rejection triggers exactly one refresh-and-retry cycle.
    pub async fn submit_freepublish(
        &self,
        app_id: &str,
        app_secret: &str,
        draft_media_id: &str,
    ) -> Result<String, WechatApiClientError> {
        let access_token = self.fetch_access_token(app_id, app_secret).await?;
        match self
            .request_submit_freepublish(&access_token, draft_media_id)
            .await
        {
            Err(WechatApiClientError::AccessTokenRejected) => {
                self.invalidate_access_token(app_id);
                let access_token = self.fetch_access_token(app_id, app_secret).await?;
                self.request_submit_freepublish(&access_token, draft_media_id)
                    .await
            }
            outcome => outcome,
        }
    }

    async fn request_submit_freepublish(
        &self,
        access_token: &str,
        draft_media_id: &str,
    ) -> Result<String, WechatApiClientError> {
        let url = self.wechat_url(&format!(
            "/cgi-bin/freepublish/submit?access_token={}",
            urlencoding::encode(access_token),
        ))?;
        let payload = serde_json::json!({ "media_id": draft_media_id });
        let response = self
            .http()?
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(redacted_reqwest_error)?;
        let body: FreepublishSubmitResponse = parse_wechat_json(response).await?;
        ensure_wechat_ok("freepublish submit", body.errcode, body.errmsg)?;
        required_upstream_field("freepublish submit", "publish_id", body.publish_id)
    }
}

/// True when the token error is a rate limit or transient upstream failure that a short
/// bounded retry can recover from. All other errors fail immediately.
fn is_retryable(error: &WechatApiClientError) -> bool {
    let WechatApiClientError::Api(detail) = error else {
        return false;
    };
    WECHAT_RETRYABLE_ERROR_CODES
        .iter()
        .any(|code| detail.contains(code))
}

fn backoff_for_attempt(attempt: u32) -> Duration {
    // 500 ms doubling to 1 s, plus a small deterministic jitter fraction.
    let base_ms = 500u64 * 2u64.pow(attempt);
    Duration::from_millis(base_ms + (attempt as u64 * 137))
}

fn build_wechat_url(path_and_query: &str) -> Result<Url, WechatApiClientError> {
    let url = Url::parse(&format!("https://{WECHAT_API_HOST}{path_and_query}")).map_err(|_| {
        WechatApiClientError::InvalidRequest(
            "failed to construct allowlisted WeChat API URL".to_string(),
        )
    })?;
    if url.host_str() != Some(WECHAT_API_HOST) {
        return Err(WechatApiClientError::InvalidRequest(
            "wechat api host is not allowlisted".to_string(),
        ));
    }
    Ok(url)
}

fn redacted_reqwest_error(error: reqwest::Error) -> WechatApiClientError {
    WechatApiClientError::Http(redacted_reqwest_error_detail(&error))
}

async fn parse_wechat_json<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, WechatApiClientError> {
    // WeChat normally answers 200 with an errcode envelope; a non-2xx body is
    // an upstream/transport failure and must not be fed to the envelope codec.
    if !response.status().is_success() {
        return Err(WechatApiClientError::Http(format!(
            "wechat upstream returned HTTP {}",
            response.status()
        )));
    }
    let body = read_bounded_http_body(response, MAX_WECHAT_JSON_RESPONSE_BYTES)
        .await
        .map_err(|error| match error {
            BoundedHttpBodyError::TooLarge { max_bytes } => {
                WechatApiClientError::Api(format!("wechat response exceeds {max_bytes} bytes"))
            }
            BoundedHttpBodyError::Read { detail } => WechatApiClientError::Http(detail),
        })?;
    serde_json::from_slice(&body)
        .map_err(|_| WechatApiClientError::Http("upstream response decoding failed".to_string()))
}

#[derive(Debug, Error)]
pub enum WechatApiClientError {
    #[error("invalid wechat api request: {0}")]
    InvalidRequest(String),
    #[error("wechat api client configuration failed: {0}")]
    Configuration(String),
    /// Upstream rejected the presented access token (40001-class errcode). Callers that
    /// hold app credentials refresh the token and may retry the operation exactly once.
    #[error("wechat access token was rejected by upstream")]
    AccessTokenRejected,
    #[error("wechat api call failed: {0}")]
    Api(String),
    #[error("wechat api transport failed: {0}")]
    Http(String),
}

/// Maps a WeChat business-response envelope to `Ok` or a typed error. The raw upstream
/// message is diagnostic only: it is logged, never carried into the error surface.
fn ensure_wechat_ok(
    operation: &'static str,
    errcode: Option<i64>,
    errmsg: Option<String>,
) -> Result<(), WechatApiClientError> {
    let Some(errcode) = errcode.filter(|code| *code != 0) else {
        return Ok(());
    };
    tracing::warn!(
        errcode,
        upstream_detail = errmsg.as_deref().unwrap_or("none"),
        "wechat {operation} rejected by upstream"
    );
    if WECHAT_TOKEN_ERROR_CODES.contains(&errcode) {
        return Err(WechatApiClientError::AccessTokenRejected);
    }
    Err(WechatApiClientError::Api(format!(
        "wechat {operation} failed with upstream code {errcode}"
    )))
}

/// Requires a non-empty string field from an upstream success envelope. Upstream
/// success responses that omit their identifier are transport contract violations.
fn required_upstream_field(
    operation: &'static str,
    field: &'static str,
    value: Option<String>,
) -> Result<String, WechatApiClientError> {
    value.filter(|value| !value.is_empty()).ok_or_else(|| {
        WechatApiClientError::Api(format!("wechat {operation} response is missing {field}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    #[test]
    fn reqwest_errors_are_rendered_without_request_urls_or_credentials() {
        let error = Client::new()
            .get("http://[::1?access_token=super-secret")
            .build()
            .expect_err("invalid URL must fail request construction");

        let rendered = redacted_reqwest_error(error).to_string();

        assert!(!rendered.contains("super-secret"));
        assert!(!rendered.contains("access_token"));
        assert!(!rendered.contains("http://"));
    }

    #[test]
    fn retryable_errors_match_rate_limit_and_transient_codes() {
        assert!(is_retryable(&WechatApiClientError::Api(
            "wechat token request failed with code -1: system error rid: 123".to_string()
        )));
        assert!(is_retryable(&WechatApiClientError::Api(
            "wechat token request failed with code 45009: api freq out of limit".to_string()
        )));
        assert!(is_retryable(&WechatApiClientError::Api(
            "wechat token request failed with code 45002: concurrent limited".to_string()
        )));
        assert!(!is_retryable(&WechatApiClientError::Api(
            "wechat token request failed with code 40013: invalid appid".to_string()
        )));
        assert!(!is_retryable(&WechatApiClientError::Http(
            "transport".to_string()
        )));
        assert!(!is_retryable(&WechatApiClientError::Configuration(
            "config".to_string()
        )));
    }

    #[test]
    fn backoff_is_bounded_and_increases() {
        let first = backoff_for_attempt(0);
        let second = backoff_for_attempt(1);
        assert_eq!(first, Duration::from_millis(500));
        assert_eq!(second, Duration::from_millis(1_137));
        assert!(second > first);
        assert!(second <= Duration::from_secs(2));
    }

    #[test]
    fn cached_access_token_is_returned_until_ttl() {
        let client = WechatApiClient::new();
        assert_eq!(client.cached_access_token("app-1"), None);
        client.cache_access_token("app-1", "token-1".to_string(), 60);
        assert_eq!(
            client.cached_access_token("app-1").as_deref(),
            Some("token-1")
        );
    }

    #[test]
    fn cached_access_token_expires() {
        let client = WechatApiClient::new();
        {
            let mut cache = client.token_cache.lock().expect("cache lock");
            cache.insert(
                "app-expired".to_string(),
                (
                    "stale-token".to_string(),
                    Instant::now() - Duration::from_secs(1),
                ),
            );
        }
        assert_eq!(client.cached_access_token("app-expired"), None);
    }

    fn mock_client(server_uri: &str) -> WechatApiClient {
        WechatApiClient::with_api_base_override(server_uri.parse().expect("mock api base url"))
    }

    fn draft_article() -> WechatDraftArticle {
        WechatDraftArticle {
            title: "Title".to_string(),
            author: "Author".to_string(),
            digest: Some("Digest".to_string()),
            content: "<p>Body</p>".to_string(),
            content_source_url: None,
            thumb_media_id: Some("THUMB_1".to_string()),
        }
    }

    async fn mount_token_endpoint(server: &MockServer, expected_calls: u64) {
        Mock::given(method("GET"))
            .and(path("/cgi-bin/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "token-1",
                "expires_in": 7200
            })))
            .expect(expected_calls)
            .mount(server)
            .await;
    }

    /// Captures `path?query` plus the raw request body of every matched request so the
    /// assertions can inspect the exact wire format the client produced.
    type CapturedRequests = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

    fn capture_requests() -> CapturedRequests {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn captured(captures: &CapturedRequests) -> Vec<(String, String)> {
        captures
            .lock()
            .expect("capture lock")
            .iter()
            .map(|(path_and_query, body)| {
                (
                    path_and_query.clone(),
                    String::from_utf8_lossy(body).to_string(),
                )
            })
            .collect()
    }

    fn capture_multipart(responder_target: CapturedRequests) -> impl Fn(&Request) -> ResponseTemplate {
        move |request: &Request| {
            if let Ok(mut captures) = responder_target.lock() {
                captures.push((
                    format!(
                        "{}?{}",
                        request.url.path(),
                        request.url.query().unwrap_or_default()
                    ),
                    request.body.clone(),
                ));
            }
            ResponseTemplate::new(200).set_body_json(json!({
                "media_id": "MEDIA_ID_1",
                "url": "https://mmbiz.example.com/cover.png"
            }))
        }
    }

    #[tokio::test]
    async fn upload_permanent_image_posts_multipart_media_field_and_parses_media_id() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;
        let captures = capture_requests();
        Mock::given(method("POST"))
            .and(path("/cgi-bin/material/add_material"))
            .respond_with(capture_multipart(Arc::clone(&captures)))
            .expect(1)
            .mount(&server)
            .await;
        let client = mock_client(&server.uri());

        let material = client
            .upload_permanent_image("wx-app", "app-secret", "cover.png", "image/png", b"png-bytes")
            .await
            .expect("permanent material upload");

        assert_eq!(material.media_id, "MEDIA_ID_1");
        assert_eq!(
            material.url.as_deref(),
            Some("https://mmbiz.example.com/cover.png")
        );
        let requests = captured(&captures);
        assert_eq!(requests.len(), 1);
        let (path_and_query, body) = &requests[0];
        assert_eq!(
            path_and_query.as_str(),
            "/cgi-bin/material/add_material?access_token=token-1&type=image"
        );
        assert!(body.contains("name=\"media\""), "multipart body: {body}");
        assert!(body.contains("filename=\"cover.png\""));
        assert!(body.contains("image/png"));
        assert!(body.contains("png-bytes"));
    }

    #[tokio::test]
    async fn add_draft_posts_wechat_article_payload_and_parses_draft_media_id() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;
        let captures = capture_requests();
        let responder_target = Arc::clone(&captures);
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(move |request: &Request| {
                if let Ok(mut captures) = responder_target.lock() {
                    captures.push((
                        format!(
                            "{}?{}",
                            request.url.path(),
                            request.url.query().unwrap_or_default()
                        ),
                        request.body.clone(),
                    ));
                }
                ResponseTemplate::new(200).set_body_json(json!({ "media_id": "DRAFT_ID_1" }))
            })
            .expect(1)
            .mount(&server)
            .await;
        let client = mock_client(&server.uri());

        let draft_media_id = client
            .add_draft("wx-app", "app-secret", vec![draft_article()])
            .await
            .expect("draft add");

        assert_eq!(draft_media_id, "DRAFT_ID_1");
        let requests = captured(&captures);
        assert_eq!(requests.len(), 1);
        let (path_and_query, body) = &requests[0];
        assert_eq!(
            path_and_query.as_str(),
            "/cgi-bin/draft/add?access_token=token-1"
        );
        let payload: serde_json::Value =
            serde_json::from_str(body).expect("draft add request body json");
        assert_eq!(payload["articles"].as_array().map(Vec::len), Some(1));
        assert_eq!(payload["articles"][0]["title"], "Title");
        assert_eq!(payload["articles"][0]["author"], "Author");
        assert_eq!(payload["articles"][0]["digest"], "Digest");
        assert_eq!(payload["articles"][0]["content"], "<p>Body</p>");
        assert_eq!(payload["articles"][0]["thumb_media_id"], "THUMB_1");
        assert_eq!(payload["articles"][0]["need_open_comment"], 0);
        assert_eq!(payload["articles"][0]["only_fans_can_comment"], 0);
        assert!(payload["articles"][0]["content_source_url"].is_null());
    }

    #[tokio::test]
    async fn submit_freepublish_parses_publish_id() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;
        Mock::given(method("POST"))
            .and(path("/cgi-bin/freepublish/submit"))
            .respond_with(|request: &Request| {
                let payload: serde_json::Value =
                    serde_json::from_slice(&request.body).unwrap_or(serde_json::Value::Null);
                assert_eq!(payload["media_id"], "DRAFT_ID_1");
                ResponseTemplate::new(200).set_body_json(json!({
                    "errcode": 0,
                    "errmsg": "ok",
                    "publish_id": "PUBLISH_ID_1"
                }))
            })
            .expect(1)
            .mount(&server)
            .await;
        let client = mock_client(&server.uri());

        let publish_id = client
            .submit_freepublish("wx-app", "app-secret", "DRAFT_ID_1")
            .await
            .expect("freepublish submit");

        assert_eq!(publish_id, "PUBLISH_ID_1");
    }

    #[tokio::test]
    async fn upstream_errcode_maps_to_safe_typed_error_without_upstream_text() {
        let server = MockServer::start().await;
        mount_token_endpoint(&server, 1).await;
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(|_request: &Request| {
                ResponseTemplate::new(200).set_body_json(json!({
                    "errcode": 53401,
                    "errmsg": "secret upstream detail text"
                }))
            })
            .mount(&server)
            .await;
        let client = mock_client(&server.uri());

        let error = client
            .add_draft("wx-app", "app-secret", vec![draft_article()])
            .await
            .expect_err("draft add must fail on upstream errcode");

        match error {
            WechatApiClientError::Api(detail) => {
                assert!(detail.contains("53401"), "detail: {detail}");
                assert!(
                    !detail.contains("secret upstream detail"),
                    "raw upstream text leaked: {detail}"
                );
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn token_rejection_triggers_single_refresh_and_retry() {
        let server = MockServer::start().await;
        let token_calls = Arc::new(AtomicUsize::new(0));
        let token_counter = Arc::clone(&token_calls);
        Mock::given(method("GET"))
            .and(path("/cgi-bin/token"))
            .respond_with(move |_request: &Request| {
                let call = token_counter.fetch_add(1, Ordering::SeqCst);
                let token = if call == 0 { "token-a" } else { "token-b" };
                ResponseTemplate::new(200).set_body_json(json!({
                    "access_token": token,
                    "expires_in": 7200
                }))
            })
            .expect(2)
            .mount(&server)
            .await;
        let draft_calls = Arc::new(AtomicUsize::new(0));
        let draft_counter = Arc::clone(&draft_calls);
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(move |request: &Request| {
                let call = draft_counter.fetch_add(1, Ordering::SeqCst);
                let presented = request
                    .url
                    .query_pairs()
                    .find(|(key, _)| key == "access_token")
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_default();
                if call == 0 {
                    assert_eq!(presented, "token-a", "first draft call uses the cached token");
                    ResponseTemplate::new(200).set_body_json(json!({
                        "errcode": 40001,
                        "errmsg": "invalid credential"
                    }))
                } else {
                    assert_eq!(presented, "token-b", "retry must present the refreshed token");
                    ResponseTemplate::new(200).set_body_json(json!({
                        "media_id": "DRAFT_AFTER_REFRESH"
                    }))
                }
            })
            .expect(2)
            .mount(&server)
            .await;
        let client = mock_client(&server.uri());

        let draft_media_id = client
            .add_draft("wx-app", "app-secret", vec![draft_article()])
            .await
            .expect("draft add succeeds after one refresh-and-retry");

        assert_eq!(draft_media_id, "DRAFT_AFTER_REFRESH");
        assert_eq!(token_calls.load(Ordering::SeqCst), 2);
        assert_eq!(draft_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn repeated_token_rejection_fails_after_the_single_retry() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cgi-bin/token"))
            .respond_with(|_request: &Request| {
                ResponseTemplate::new(200).set_body_json(json!({
                    "access_token": "fresh-token",
                    "expires_in": 7200
                }))
            })
            .mount(&server)
            .await;
        let draft_calls = Arc::new(AtomicUsize::new(0));
        let draft_counter = Arc::clone(&draft_calls);
        Mock::given(method("POST"))
            .and(path("/cgi-bin/draft/add"))
            .respond_with(move |_request: &Request| {
                draft_counter.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({
                    "errcode": 40001,
                    "errmsg": "invalid credential"
                }))
            })
            .mount(&server)
            .await;
        let client = mock_client(&server.uri());

        let error = client
            .add_draft("wx-app", "app-secret", vec![draft_article()])
            .await
            .expect_err("a second rejection must fail, not loop");

        assert!(matches!(
            error,
            WechatApiClientError::AccessTokenRejected
        ));
        assert_eq!(draft_calls.load(Ordering::SeqCst), 2);
    }
}
