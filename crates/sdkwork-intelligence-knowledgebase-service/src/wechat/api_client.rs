use crate::bounded_http_body::{
    read_bounded_http_body, redacted_reqwest_error_detail, BoundedHttpBodyError,
};
use reqwest::Client;
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use thiserror::Error;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatUserTag {
    pub id: i64,
    pub name: String,
    pub count: u64,
}

pub struct WechatApiClient {
    http: Result<Client, String>,
    token_cache: Mutex<HashMap<String, (String, Instant)>>,
}

impl Default for WechatApiClient {
    fn default() -> Self {
        Self {
            http: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(WECHAT_API_TIMEOUT_SECS))
                .build()
                .map_err(|error| redacted_reqwest_error_detail(&error)),
            token_cache: Mutex::new(HashMap::new()),
        }
    }
}

impl WechatApiClient {
    pub fn new() -> Self {
        Self::default()
    }

    fn http(&self) -> Result<&Client, WechatApiClientError> {
        self.http
            .as_ref()
            .map_err(|detail| WechatApiClientError::Configuration(detail.clone()))
    }

    /// Returns a cached, still-valid access token when present, otherwise fetches a fresh
    /// one with bounded retries and caches it.
    pub async fn fetch_access_token(
        &self,
        app_id: &str,
        app_secret: &str,
    ) -> Result<String, WechatApiClientError> {
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
        let url = build_wechat_url(&format!(
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
        let url = build_wechat_url(&format!(
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
    #[error("wechat api call failed: {0}")]
    Api(String),
    #[error("wechat api transport failed: {0}")]
    Http(String),
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
