//! In-process Memory context provider.
//!
//! This is the default Knowledgebase memory mode when the process is
//! co-deployed with the memory data plane. It calls the same
//! `MemoryAppApi::create_retrieval` service method as `retrievals.create`, so
//! in-process and HTTP delivery share one retrieval contract and one set of
//! scope rules.

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_memory_context::{
    KnowledgeMemoryContextProvider, KnowledgeMemoryContextProviderError,
    KnowledgeMemoryContextRequest, KnowledgeMemoryContextResult,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeMemoryContextFragment;
use sdkwork_memory_inprocess::{InProcessMemoryRuntime, DEFAULT_CONTEXT_BUDGET_TOKENS};

use crate::app_api::{estimate_tokens, parse_memory_space_id, MAX_MEMORY_FRAGMENTS};

/// In-process Memory App API provider.
pub struct InProcessMemoryContextProvider {
    runtime: InProcessMemoryRuntime,
}

impl InProcessMemoryContextProvider {
    /// Builds the provider over an initialized in-process memory runtime.
    #[must_use]
    pub fn new(runtime: InProcessMemoryRuntime) -> Self {
        Self { runtime }
    }

    /// Bootstraps the runtime from the environment.
    ///
    /// Returns `Ok(None)` when the memory database is not configured, so hosts
    /// without the memory capability keep their existing no-memory degradation.
    pub async fn from_env() -> Result<Option<Self>, String> {
        Ok(InProcessMemoryRuntime::from_env().await?.map(Self::new))
    }
}

#[async_trait]
impl KnowledgeMemoryContextProvider for InProcessMemoryContextProvider {
    async fn build_memory_context(
        &self,
        request: KnowledgeMemoryContextRequest,
    ) -> Result<KnowledgeMemoryContextResult, KnowledgeMemoryContextProviderError> {
        validate_request(&request)?;
        if request.actor_id.is_none() {
            tracing::warn!(
                tenant_id = request.tenant_id,
                "in-process memory context requires an authenticated actor; skipping retrieval"
            );
            return Ok(KnowledgeMemoryContextResult {
                fragments: Vec::new(),
                truncated: false,
            });
        }
        let space_id = parse_memory_space_id(&request.memory_policy_ref).ok_or_else(|| {
            KnowledgeMemoryContextProviderError::InvalidRequest(
                "memory_policy_ref must be a numeric Memory space id or space:<id>".to_string(),
            )
        })?;
        let space_id = space_id.parse::<u64>().map_err(|_| {
            KnowledgeMemoryContextProviderError::InvalidRequest(
                "memory space id is out of range".to_string(),
            )
        })?;
        let response = self
            .runtime
            .retrieve(
                request.tenant_id,
                request.actor_id,
                space_id,
                &request.query,
                Some(MAX_MEMORY_FRAGMENTS as u32),
                Some(request.max_tokens.min(DEFAULT_CONTEXT_BUDGET_TOKENS as u32)),
            )
            .await
            .map_err(|error| {
                KnowledgeMemoryContextProviderError::Upstream(format!(
                    "{}: {}",
                    error.code, error.detail
                ))
            })?;

        Ok(map_retrieval_result(response, &request))
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

fn record_content(record: &sdkwork_memory_contract::MemoryRecord) -> Option<String> {
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

fn map_retrieval_result(
    result: sdkwork_memory_contract::MemoryRetrievalResult,
    request: &KnowledgeMemoryContextRequest,
) -> KnowledgeMemoryContextResult {
    let mut fragments = Vec::new();
    let mut used_tokens = 0_u32;
    let mut truncated = result.degraded;
    for hit in result.hits {
        if fragments.len() >= MAX_MEMORY_FRAGMENTS {
            truncated = true;
            break;
        }
        let Some(record) = hit.memory else {
            continue;
        };
        let Some(content) = record_content(&record) else {
            continue;
        };
        let tokens = estimate_tokens(&content);
        if used_tokens.saturating_add(tokens) > request.max_tokens {
            truncated = true;
            break;
        }
        used_tokens = used_tokens.saturating_add(tokens);
        fragments.push(KnowledgeMemoryContextFragment {
            memory_id: record.memory_id.to_string(),
            title: None,
            content,
            score: hit.fused_score.or(Some(record.confidence)),
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
