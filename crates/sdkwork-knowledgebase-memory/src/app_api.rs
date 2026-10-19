//! Memory App API context provider.
//!
//! Reads durable user memory through the generator-owned Memory App API Rust
//! SDK. The caller's dual tokens are forwarded unchanged: the memory backend
//! derives actor, tenant, and owner scope from the credential, so this provider
//! fails closed when no token is available instead of reading memory as a
//! service identity.

use std::time::Duration;

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_memory_context::{
    KnowledgeMemoryContextProvider, KnowledgeMemoryContextProviderError,
    KnowledgeMemoryContextRequest, KnowledgeMemoryContextResult,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeMemoryContextFragment;
use sdkwork_memory_app_sdk_generated_rust::models::{MemoryRecord, MemoryRecordList};
use sdkwork_memory_app_sdk_generated_rust::SdkworkMemoryAppClient;

/// Default per-turn memory retrieval deadline.
pub const DEFAULT_MEMORY_TIMEOUT: Duration = Duration::from_secs(8);
/// Maximum fragments returned to the chat prompt.
pub const MAX_MEMORY_FRAGMENTS: usize = 8;
/// Default memory retrieval depth when the caller does not budget tokens.
pub const DEFAULT_MEMORY_TOP_K: i64 = 8;

/// Memory App API backed context provider.
pub struct MemoryAppApiContextProvider {
    base_url: String,
    timeout: Duration,
}

impl MemoryAppApiContextProvider {
    /// Builds the provider from the Memory App API origin.
    pub fn new(base_url: impl Into<String>) -> Result<Self, String> {
        let base_url = base_url.into();
        let trimmed = base_url.trim();
        if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
            return Err("memory base URL must be an absolute http(s) origin".to_string());
        }
        Ok(Self {
            base_url: trimmed.trim_end_matches('/').to_string(),
            timeout: DEFAULT_MEMORY_TIMEOUT,
        })
    }

    /// Overrides the per-turn retrieval deadline.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }
}

#[async_trait]
impl KnowledgeMemoryContextProvider for MemoryAppApiContextProvider {
    async fn build_memory_context(
        &self,
        request: KnowledgeMemoryContextRequest,
    ) -> Result<KnowledgeMemoryContextResult, KnowledgeMemoryContextProviderError> {
        validate_request(&request)?;
        let space_id = parse_memory_space_id(&request.memory_policy_ref).ok_or_else(|| {
            KnowledgeMemoryContextProviderError::InvalidRequest(
                "memory_policy_ref must be a numeric Memory space id or space:<id>".to_string(),
            )
        })?;
        let auth_token = request
            .auth_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                KnowledgeMemoryContextProviderError::Upstream(
                    "memory read requires the caller auth token".to_string(),
                )
            })?;

        let client = SdkworkMemoryAppClient::new_with_base_url(self.base_url.clone())
            .map_err(|error| KnowledgeMemoryContextProviderError::Internal(error.to_string()))?;
        client.set_auth_token(auth_token);
        if let Some(access_token) = request
            .access_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            client.set_access_token(access_token);
        }

        let response = tokio::time::timeout(
            self.timeout,
            client.memory().memories_list(
                &space_id,
                Some(request.query.as_str()),
                None,
                Some(DEFAULT_MEMORY_TOP_K),
                Some(false),
            ),
        )
        .await
        .map_err(|_| {
            KnowledgeMemoryContextProviderError::Upstream(format!(
                "memory query did not complete within {} ms",
                self.timeout.as_millis()
            ))
        })?
        .map_err(|error| KnowledgeMemoryContextProviderError::Upstream(error.to_string()))?;

        Ok(map_memory_context(response, &request))
    }
}

fn validate_request(
    request: &KnowledgeMemoryContextRequest,
) -> Result<(), KnowledgeMemoryContextProviderError> {
    if request.tenant_id == 0 {
        return Err(KnowledgeMemoryContextProviderError::InvalidRequest(
            "tenant_id is required".to_string(),
        ));
    }
    if request.query.trim().is_empty() {
        return Err(KnowledgeMemoryContextProviderError::InvalidRequest(
            "query is required".to_string(),
        ));
    }
    if request.memory_policy_ref.trim().is_empty() {
        return Err(KnowledgeMemoryContextProviderError::InvalidRequest(
            "memory_policy_ref is required".to_string(),
        ));
    }
    if request.max_tokens == 0 {
        return Err(KnowledgeMemoryContextProviderError::InvalidRequest(
            "max_tokens must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// Parses `memory_policy_ref` as a Memory space id (`4242` or `space:4242`).
#[must_use]
pub fn parse_memory_space_id(reference: &str) -> Option<String> {
    let trimmed = reference.trim();
    let numeric = trimmed.strip_prefix("space:").unwrap_or(trimmed).trim();
    if numeric.is_empty() || !numeric.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    Some(numeric.to_string())
}

fn map_memory_context(
    value: serde_json::Value,
    request: &KnowledgeMemoryContextRequest,
) -> KnowledgeMemoryContextResult {
    let Ok(page) = serde_json::from_value::<MemoryRecordList>(value) else {
        return KnowledgeMemoryContextResult {
            fragments: Vec::new(),
            truncated: false,
        };
    };
    let mut fragments = Vec::new();
    let mut used_tokens = 0_u32;
    let mut truncated = false;
    for record in page.items {
        if fragments.len() >= MAX_MEMORY_FRAGMENTS {
            truncated = true;
            break;
        }
        if !record.status.eq_ignore_ascii_case("active") {
            continue;
        }
        let Some(content) = memory_content(&record) else {
            continue;
        };
        let tokens = estimate_tokens(&content);
        if used_tokens.saturating_add(tokens) > request.max_tokens {
            truncated = true;
            break;
        }
        used_tokens = used_tokens.saturating_add(tokens);
        fragments.push(KnowledgeMemoryContextFragment {
            memory_id: record.memory_id,
            title: None,
            content,
            score: Some(record.confidence),
            rank: (fragments.len() + 1) as u32,
            token_count: Some(tokens),
            source_uri: None,
            policy_ref: Some(request.memory_policy_ref.trim().to_string()),
        });
    }
    KnowledgeMemoryContextResult {
        fragments,
        truncated,
    }
}

fn memory_content(record: &MemoryRecord) -> Option<String> {
    record
        .summary_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let text = record.canonical_text.trim();
            (!text.is_empty()).then(|| text.to_string())
        })
}

fn estimate_tokens(content: &str) -> u32 {
    content.split_whitespace().count().max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(policy_ref: &str) -> KnowledgeMemoryContextRequest {
        KnowledgeMemoryContextRequest {
            tenant_id: 100_001,
            actor_id: Some(7),
            query: "preferred refund window".to_string(),
            memory_policy_ref: policy_ref.to_string(),
            max_tokens: 1_024,
            auth_token: Some("auth-token".to_string()),
            access_token: Some("access-token".to_string()),
        }
    }

    #[test]
    fn space_references_accept_numeric_and_space_prefix() {
        assert_eq!(parse_memory_space_id("4242").as_deref(), Some("4242"));
        assert_eq!(
            parse_memory_space_id(" space:4242 ").as_deref(),
            Some("4242")
        );
        assert!(parse_memory_space_id("memory.space.42").is_none());
        assert!(parse_memory_space_id("space:").is_none());
    }

    #[test]
    fn active_memories_become_bounded_fragments() {
        let page = serde_json::json!({
            "items": [
                {
                    "memoryId": "7001", "spaceId": "4242", "userId": "7",
                    "scope": "user", "memoryType": "semantic",
                    "canonicalText": "Prefers refunds within 30 days.",
                    "summaryText": null, "confidence": 0.9,
                    "evidenceCount": 1, "contradictionCount": 0, "expiresAt": null,
                    "status": "active", "sensitivityLevel": "normal", "metadata": null,
                    "supersedesMemoryId": null, "supersededByMemoryId": null,
                    "createdAt": "2026-10-01T00:00:00Z", "updatedAt": "2026-10-01T00:00:00Z",
                    "version": "1"
                },
                {
                    "memoryId": "7002", "spaceId": "4242", "userId": "7",
                    "scope": "user", "memoryType": "semantic",
                    "canonicalText": "Deleted preference", "summaryText": null,
                    "confidence": 0.5, "evidenceCount": 1, "contradictionCount": 0,
                    "expiresAt": null, "status": "deleted", "sensitivityLevel": "normal",
                    "metadata": null, "supersedesMemoryId": null, "supersededByMemoryId": null,
                    "createdAt": "2026-10-01T00:00:00Z", "updatedAt": "2026-10-01T00:00:00Z",
                    "version": "1"
                }
            ],
            "pageInfo": { "mode": "cursor", "hasMore": false }
        });
        let result = map_memory_context(page, &request("space:4242"));
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].memory_id, "7001");
        assert_eq!(
            result.fragments[0].policy_ref.as_deref(),
            Some("space:4242")
        );
        assert!(!result.truncated);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn provider_forwards_dual_tokens_and_query_to_memories_list() {
        use serde_json::json;
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/app/v3/api/memory/memories"))
            .and(query_param("space_id", "4242"))
            .and(query_param("q", "preferred refund window"))
            .and(header("Authorization", "Bearer auth-token"))
            .and(header("Access-Token", "access-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "data": {
                    "items": [{
                        "memoryId": "7001", "spaceId": "4242", "userId": "7",
                        "scope": "user", "memoryType": "semantic",
                        "canonicalText": "Prefers refunds within 30 days.",
                        "summaryText": null, "confidence": 0.9,
                        "evidenceCount": 1, "contradictionCount": 0, "expiresAt": null,
                        "status": "active", "sensitivityLevel": "normal", "metadata": null,
                        "supersedesMemoryId": null, "supersededByMemoryId": null,
                        "createdAt": "2026-10-01T00:00:00Z",
                        "updatedAt": "2026-10-01T00:00:00Z", "version": "1"
                    }],
                    "pageInfo": { "mode": "cursor", "hasMore": false }
                },
                "traceId": "00000000-0000-4000-8000-000000000000"
            })))
            .mount(&server)
            .await;

        let provider = MemoryAppApiContextProvider::new(server.uri()).expect("provider");
        let result = provider
            .build_memory_context(request("space:4242"))
            .await
            .expect("memory context");
        assert_eq!(result.fragments.len(), 1);
        assert!(result.fragments[0].content.contains("30 days"));
    }
}
