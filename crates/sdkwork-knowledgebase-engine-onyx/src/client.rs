//! Onyx unified search HTTP client (adapter-local; handlers must not call Onyx directly).

use reqwest::Method;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineDocument, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineSearchHit, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_provider_runtime::{
    engine_provider_error, optional_bearer_token, ProviderErrorCategory, ProviderExecutionContext,
    ProviderHttpRequest, ProviderOperation, ProviderRuntime,
};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::config::OnyxConnectorConfig;
use crate::ONYX_IMPLEMENTATION_ID;

/// Maximum `url:` ids retained per space for the read allowlist; the oldest URL
/// is evicted first so long-lived engines keep a flat memory profile.
const MAX_ALLOWED_URLS_PER_SPACE: usize = 512;

#[derive(Clone)]
pub struct OnyxApiClient {
    config: OnyxConnectorConfig,
    http: ProviderRuntime,
    allowlist: SearchUrlAllowlist,
}

impl OnyxApiClient {
    pub fn new(config: OnyxConnectorConfig) -> Result<Self, KnowledgeEngineError> {
        let http = ProviderRuntime::for_base_url_with_private_targets(
            &config.base_url,
            config.allow_private_network,
        )
        .map_err(KnowledgeEngineError::from)?;
        Ok(Self {
            config,
            http,
            allowlist: SearchUrlAllowlist::default(),
        })
    }

    fn bearer_token(&self) -> Option<&str> {
        optional_bearer_token(Some(self.config.api_key.as_str()))
    }

    fn health_context(&self) -> ProviderExecutionContext {
        ProviderExecutionContext::for_system_health(ONYX_IMPLEMENTATION_ID)
    }

    pub async fn connector_health(&self) -> Result<(), KnowledgeEngineError> {
        let url = format!("{}/health", self.config.base_url.trim_end_matches('/'));
        let request = ProviderHttpRequest::new(ProviderOperation::Health, Method::GET, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        self.http
            .execute(&self.health_context(), request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        Ok(())
    }

    pub async fn search(
        &self,
        context: &ProviderExecutionContext,
        space_id: u64,
        query: &str,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let url = format!("{}/search", self.config.base_url.trim_end_matches('/'));
        let request = ProviderHttpRequest::new(ProviderOperation::Search, Method::POST, url)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "query": query,
                "skip_query_expansion": false,
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: OnyxSearchResponse = response.json().map_err(KnowledgeEngineError::from)?;

        if let Some(error) = payload.error.filter(|value| !value.is_empty()) {
            // Raw engine error text is diagnostic-only and must not reach API errors.
            tracing::warn!(implementation_id = ONYX_IMPLEMENTATION_ID, error = %error, "engine request failed");
            return Err(engine_provider_error(
                ProviderOperation::Search,
                ONYX_IMPLEMENTATION_ID,
                ProviderErrorCategory::InvalidResponse,
                "onyx rejected the request",
            ));
        }

        let mut hits = Vec::with_capacity(payload.results.len());
        for result in payload.results {
            if let Some(hit_url) = result.url.as_deref() {
                if !hit_url.is_empty() {
                    self.allowlist.record(space_id, hit_url);
                }
            }
            hits.push(map_result_to_hit(space_id, result));
        }

        Ok(KnowledgeEngineSearchResult {
            implementation_id: ONYX_IMPLEMENTATION_ID.to_string(),
            hits,
        })
    }

    pub async fn read_url_document(
        &self,
        context: &ProviderExecutionContext,
        url: &str,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        // SSRF invariant: Onyx fetches `open_urls` server-side, so only URLs that
        // THIS space's search results previously returned may be read, and only
        // over https. Caller-supplied `url:` ids that never surfaced in a search
        // hit are rejected before any upstream request is made.
        if !is_https_url(url) {
            return Err(KnowledgeEngineError::Validation(
                "onyx read_document only accepts https url: document ids from search hits"
                    .to_string(),
            ));
        }
        if !self.allowlist.contains(context.space_id, url) {
            return Err(KnowledgeEngineError::NotFound(
                "onyx document url was not returned by this space's search results".to_string(),
            ));
        }

        let endpoint = format!("{}/open_urls", self.config.base_url.trim_end_matches('/'));
        let request = ProviderHttpRequest::new(ProviderOperation::Read, Method::POST, endpoint)
            .map_err(KnowledgeEngineError::from)?
            .optional_bearer_auth(self.bearer_token())
            .map_err(KnowledgeEngineError::from)?
            .json(&serde_json::json!({
                "urls": [url],
            }))
            .map_err(KnowledgeEngineError::from)?
            .idempotent(true);
        let response = self
            .http
            .execute(context, request)
            .await
            .map_err(KnowledgeEngineError::from)?;
        let payload: OnyxOpenUrlsResponse = response.json().map_err(KnowledgeEngineError::from)?;

        if let Some(error) = payload.error.filter(|value| !value.is_empty()) {
            // Raw engine error text is diagnostic-only and must not reach API errors.
            tracing::warn!(implementation_id = ONYX_IMPLEMENTATION_ID, error = %error, "engine request failed");
            return Err(engine_provider_error(
                ProviderOperation::Read,
                ONYX_IMPLEMENTATION_ID,
                ProviderErrorCategory::InvalidResponse,
                "onyx rejected the request",
            ));
        }

        let Some(result) = payload.results.into_iter().next() else {
            return Err(KnowledgeEngineError::NotFound(
                "onyx document not found for the requested url".to_string(),
            ));
        };

        let title = result.title.unwrap_or_else(|| url.to_string());
        let content = result.content.unwrap_or_default();

        Ok(KnowledgeEngineDocument {
            document_id: encode_url_document_id(url),
            title,
            content,
            source_uri: Some(url.to_string()),
        })
    }
}

fn is_https_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|parsed| parsed.scheme() == "https")
}

/// Per-space bounded allowlist of `url:` document ids returned by THIS space's
/// search results. `read_url_document` may only fetch URLs recorded here, which
/// keeps Onyx server-side fetches from being redirected by forged document ids.
#[derive(Clone, Default)]
struct SearchUrlAllowlist {
    urls_per_space: Arc<Mutex<HashMap<u64, VecDeque<String>>>>,
}

impl SearchUrlAllowlist {
    fn record(&self, space_id: u64, url: &str) {
        if !is_https_url(url) {
            return;
        }
        let mut urls = self.locked();
        let queue = urls.entry(space_id).or_default();
        if queue.iter().any(|existing| existing == url) {
            return;
        }
        if queue.len() >= MAX_ALLOWED_URLS_PER_SPACE {
            queue.pop_front();
        }
        queue.push_back(url.to_string());
    }

    fn contains(&self, space_id: u64, url: &str) -> bool {
        self.locked()
            .get(&space_id)
            .is_some_and(|queue| queue.iter().any(|existing| existing == url))
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, HashMap<u64, VecDeque<String>>> {
        self.urls_per_space
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn encode_url_document_id(url: &str) -> String {
    format!("url:{url}")
}

pub fn decode_url_document_id(document_id: &str) -> Option<String> {
    document_id.strip_prefix("url:").map(str::to_string)
}

fn map_result_to_hit(space_id: u64, result: OnyxSearchResult) -> KnowledgeEngineSearchHit {
    let url = result.url.unwrap_or_default();
    let title = result.title.unwrap_or_else(|| url.clone());
    let local_document_id = if url.is_empty() {
        "unknown".to_string()
    } else {
        encode_url_document_id(&url)
    };

    KnowledgeEngineSearchHit {
        document: KnowledgeEngineDocumentRef {
            document_id: format!("{space_id}/{local_document_id}"),
            title,
            source_uri: if url.is_empty() { None } else { Some(url) },
        },
        snippet: result.content.unwrap_or_default(),
        score: None,
    }
}

#[derive(Debug, Deserialize)]
struct OnyxSearchResponse {
    #[serde(default)]
    results: Vec<OnyxSearchResult>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OnyxSearchResult {
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
    #[serde(rename = "source_type")]
    _source_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OnyxOpenUrlsResponse {
    #[serde(default)]
    results: Vec<OnyxOpenUrlResult>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OnyxOpenUrlResult {
    title: Option<String>,
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_onyx_search_payload_to_scoped_hits() {
        let payload = r#"{
            "results": [
                {
                    "title": "Reset Okta",
                    "url": "https://example.com/okta",
                    "content": "reset steps",
                    "source_type": "web"
                }
            ]
        }"#;
        let parsed: OnyxSearchResponse = serde_json::from_str(payload).expect("parse");
        let hit = map_result_to_hit(5, parsed.results.into_iter().next().expect("result"));

        assert_eq!(hit.document.document_id, "5/url:https://example.com/okta");
        assert_eq!(hit.document.title, "Reset Okta");
        assert_eq!(hit.snippet, "reset steps");
    }

    #[test]
    fn url_document_id_round_trips() {
        let url = "https://example.com/doc";
        let encoded = encode_url_document_id(url);
        assert_eq!(decode_url_document_id(&encoded).as_deref(), Some(url));
    }

    #[test]
    fn allowlist_records_only_https_and_space_scoped_lookups() {
        let allowlist = SearchUrlAllowlist::default();
        allowlist.record(1, "http://example.com/insecure");
        allowlist.record(1, "https://example.com/doc");
        allowlist.record(2, "https://example.com/other");

        assert!(allowlist.contains(1, "https://example.com/doc"));
        assert!(allowlist.contains(2, "https://example.com/other"));
        assert!(!allowlist.contains(1, "http://example.com/insecure"));
        assert!(!allowlist.contains(2, "https://example.com/doc"));
        assert!(!allowlist.contains(1, "https://example.com/never-searched"));
    }

    #[test]
    fn allowlist_evicts_oldest_url_per_space() {
        let allowlist = SearchUrlAllowlist::default();
        for index in 0..MAX_ALLOWED_URLS_PER_SPACE {
            allowlist.record(1, &format!("https://example.com/{index}"));
        }
        allowlist.record(1, "https://example.com/newest");

        assert!(!allowlist.contains(1, "https://example.com/0"));
        assert!(allowlist.contains(1, "https://example.com/1"));
        assert!(allowlist.contains(1, "https://example.com/newest"));
    }

    #[test]
    fn non_https_urls_are_rejected_before_allowlist_lookup() {
        assert!(!is_https_url("http://example.com/doc"));
        assert!(!is_https_url("file:///etc/passwd"));
        assert!(!is_https_url("not a url"));
        assert!(is_https_url("https://example.com/doc"));
    }
}
