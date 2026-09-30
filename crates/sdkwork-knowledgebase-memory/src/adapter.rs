use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_memory_context::{
    KnowledgeMemoryContextProvider, KnowledgeMemoryContextProviderError,
    KnowledgeMemoryContextRequest, KnowledgeMemoryContextResult,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeMemoryContextFragment;
use sdkwork_memory_spi::{
    AssembleMemoryContextCommand, MAX_MEMORY_RETRIEVAL_CANDIDATES, MemoryContextAssemblerPort,
    MemoryContextPackDraft, MemoryRetrieverPort, MemorySensitivityReadScope,
    RetrieveMemoryCandidatesCommand,
};
use sdkwork_utils_rust::{is_blank, trim};
use std::sync::Arc;

pub struct KnowledgebaseMemoryContextProviderAdapter {
    retriever: Arc<dyn MemoryRetrieverPort>,
    assembler: Arc<dyn MemoryContextAssemblerPort>,
}

impl KnowledgebaseMemoryContextProviderAdapter {
    pub fn new<R, A>(retriever: Arc<R>, assembler: Arc<A>) -> Self
    where
        R: MemoryRetrieverPort + 'static,
        A: MemoryContextAssemblerPort + 'static,
    {
        Self {
            retriever,
            assembler,
        }
    }
}

#[async_trait]
impl KnowledgeMemoryContextProvider for KnowledgebaseMemoryContextProviderAdapter {
    async fn build_memory_context(
        &self,
        request: KnowledgeMemoryContextRequest,
    ) -> Result<KnowledgeMemoryContextResult, KnowledgeMemoryContextProviderError> {
        validate_request(&request)?;

        let query = trim(request.query.as_str());
        let policy_ref = trim(request.memory_policy_ref.as_str());
        // Fail-closed sensitivity ceiling: the knowledgebase memory context is a
        // cross-actor shared surface, so it reads only Public-sensitivity
        // memories until a caller-facing scope contract exists (upstream
        // forbids stores from choosing a scope themselves).
        let candidates = self
            .retriever
            .retrieve(RetrieveMemoryCandidatesCommand {
                query,
                read_scope: MemorySensitivityReadScope::Public,
                limit: MAX_MEMORY_RETRIEVAL_CANDIDATES,
            })
            .await
            .map_err(map_memory_spi_error)?;
        if candidates.memory_ids.is_empty() {
            return Ok(KnowledgeMemoryContextResult {
                fragments: vec![],
                truncated: false,
            });
        }

        let draft = self
            .assembler
            .assemble(AssembleMemoryContextCommand {
                memory_ids: candidates.memory_ids,
            })
            .await
            .map_err(map_memory_spi_error)?;

        Ok(memory_context_result_from_draft(
            draft,
            &policy_ref,
            request.max_tokens,
        ))
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
    if is_blank(Some(request.query.as_str())) {
        return Err(KnowledgeMemoryContextProviderError::InvalidRequest(
            "query is required".to_string(),
        ));
    }
    if is_blank(Some(request.memory_policy_ref.as_str())) {
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

fn memory_context_result_from_draft(
    draft: MemoryContextPackDraft,
    policy_ref: &str,
    max_tokens: u32,
) -> KnowledgeMemoryContextResult {
    let context_lines = draft
        .context_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let fallback_context = draft.context_text.trim().to_string();
    let mut estimated_tokens = 0_u32;
    let mut truncated = false;
    let mut fragments = Vec::new();

    for (index, memory_id) in draft.memory_ids.iter().enumerate() {
        let content = context_lines
            .get(index)
            .cloned()
            .or_else(|| {
                (draft.memory_ids.len() == 1 && !fallback_context.is_empty())
                    .then(|| fallback_context.clone())
            })
            .unwrap_or_default();
        if content.is_empty() {
            continue;
        }

        let token_count = estimate_tokens(&content);
        if estimated_tokens.saturating_add(token_count) > max_tokens {
            truncated = true;
            break;
        }
        estimated_tokens = estimated_tokens.saturating_add(token_count);

        fragments.push(KnowledgeMemoryContextFragment {
            memory_id: memory_id.clone(),
            title: None,
            content,
            score: None,
            rank: (fragments.len() + 1) as u32,
            token_count: Some(token_count),
            source_uri: Some(format!("memory://{memory_id}")),
            policy_ref: Some(policy_ref.to_string()),
        });
    }

    KnowledgeMemoryContextResult {
        fragments,
        truncated,
    }
}

fn estimate_tokens(content: &str) -> u32 {
    content.split_whitespace().count().max(1) as u32
}

fn map_memory_spi_error(
    error: sdkwork_memory_spi::MemorySpiError,
) -> KnowledgeMemoryContextProviderError {
    KnowledgeMemoryContextProviderError::Upstream(error.to_string())
}
