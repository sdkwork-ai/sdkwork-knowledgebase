use async_trait::async_trait;

use crate::ports::{
    knowledge_memory_context::{
        KnowledgeMemoryContextProvider, KnowledgeMemoryContextProviderError,
        KnowledgeMemoryContextRequest,
    },
    knowledge_retrieval_backend::{
        KnowledgeChunkSearchHit, KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend,
        KnowledgeRetrievalBackendError,
    },
    knowledge_retrieval_trace_store::{
        CreateKnowledgeRetrievalHitRecord, CreateKnowledgeRetrievalTraceRecord,
        KnowledgeRetrievalTraceHitRecord, KnowledgeRetrievalTraceStore,
        KnowledgeRetrievalTraceStoreError,
    },
};
use crate::public_web_search::{
    metadata_public_web_top_k, metadata_requests_public_web, search_public_web, stable_web_hit_ids,
    PublicWebSearchError, PublicWebSearchHit,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeCitation, KnowledgeContextFragment, KnowledgeContextPack, KnowledgeContextPackRequest,
    KnowledgeFilter, KnowledgeMemoryContextFragment, KnowledgeRetrievalBinding,
    KnowledgeRetrievalMethod, KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
    KnowledgeRetrievalTrace,
};
use sdkwork_knowledgebase_observability::{
    record_context_pack_completed, record_retrieval_completed,
};
use sdkwork_utils_rust::{is_blank, sha256_hash};
use std::{cmp::Ordering, time::Instant};
use thiserror::Error;

const DEFAULT_TOP_K: u32 = 8;
const MAX_TOP_K: u32 = 64;

pub struct KnowledgeRetrievalService<'a> {
    backend: &'a dyn KnowledgeRetrievalBackend,
    traces: &'a dyn KnowledgeRetrievalTraceStore,
    memory: Option<&'a dyn KnowledgeMemoryContextProvider>,
}

#[async_trait]
pub trait KnowledgeRetrievalExecutor: Send + Sync {
    async fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeRetrievalServiceError>;
}

#[async_trait]
impl KnowledgeRetrievalExecutor for KnowledgeRetrievalService<'_> {
    async fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeRetrievalServiceError> {
        KnowledgeRetrievalService::retrieve(self, request).await
    }
}

impl<'a> KnowledgeRetrievalService<'a> {
    pub fn new(
        backend: &'a dyn KnowledgeRetrievalBackend,
        traces: &'a dyn KnowledgeRetrievalTraceStore,
    ) -> Self {
        Self {
            backend,
            traces,
            memory: None,
        }
    }

    pub fn with_memory(
        backend: &'a dyn KnowledgeRetrievalBackend,
        traces: &'a dyn KnowledgeRetrievalTraceStore,
        memory: &'a dyn KnowledgeMemoryContextProvider,
    ) -> Self {
        Self {
            backend,
            traces,
            memory: Some(memory),
        }
    }

    pub async fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeRetrievalServiceError> {
        validate_request(&request)?;

        let started_at = Instant::now();
        let normalized_query = request.query.trim().to_string();
        let methods = normalize_methods(&request.methods);
        let mut hits = Vec::new();

        for binding in sorted_bindings(&request.bindings) {
            let binding_top_k = normalize_top_k(binding.top_k.or(request.top_k));
            for method in &methods {
                let mut binding_hits = self
                    .backend
                    .search_chunks(KnowledgeChunkSearchRequest {
                        tenant_id: request.tenant_id,
                        query: normalized_query.clone(),
                        binding: binding.clone(),
                        method: *method,
                        top_k: binding_top_k,
                        query_embedding: None,
                    })
                    .await?;
                hits.append(&mut binding_hits);
            }
        }

        sort_hits(&mut hits);
        let limit = normalize_top_k(request.top_k);
        hits.truncate(limit as usize);

        let mut fragments = build_fragments(&hits, request.include_citations);
        if metadata_requests_public_web(&request.metadata) {
            let web_top_k = metadata_public_web_top_k(&request.metadata);
            match search_public_web(&normalized_query, web_top_k).await {
                Ok(web_hits) => {
                    append_public_web_fragments(
                        &mut fragments,
                        web_hits,
                        request.include_citations,
                    );
                }
                Err(PublicWebSearchError::Disabled) => {}
                Err(error) => {
                    tracing::warn!(error = %error, "public web search skipped");
                }
            }
        }

        let latency_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        let trace_id = self
            .persist_trace(&request, &normalized_query, &fragments, Some(latency_ms))
            .await?;

        record_retrieval_completed(request.tenant_id, fragments.len() as u32, latency_ms);

        Ok(KnowledgeRetrievalResult {
            retrieval_id: trace_id,
            trace: request.include_trace.then(|| KnowledgeRetrievalTrace {
                retrieval_trace_id: trace_id,
                status: "succeeded".to_string(),
                latency_ms: Some(latency_ms),
                result_count: fragments.len() as u32,
            }),
            hits: fragments,
        })
    }

    pub async fn create_context_pack(
        &self,
        request: KnowledgeContextPackRequest,
    ) -> Result<KnowledgeContextPack, KnowledgeRetrievalServiceError> {
        if request.context_budget_tokens == 0 {
            return Err(KnowledgeRetrievalServiceError::InvalidRequest(
                "context_budget_tokens must be greater than zero".to_string(),
            ));
        }

        let retrieval = self
            .retrieve(KnowledgeRetrievalRequest {
                tenant_id: request.tenant_id,
                actor_id: request.actor_id,
                query: request.query.clone(),
                retrieval_profile_id: request.retrieval_profile_id,
                bindings: request.bindings.clone(),
                methods: vec![KnowledgeRetrievalMethod::Hybrid],
                top_k: None,
                include_citations: request.include_citations,
                include_trace: true,
                context_budget_tokens: Some(request.context_budget_tokens),
                metadata: vec![],
            })
            .await?;

        let mut estimated_tokens = 0_u32;
        let mut truncated = false;
        let mut fragments = Vec::new();
        for fragment in retrieval.hits {
            let token_count = estimate_fragment_tokens(&fragment);
            if estimated_tokens.saturating_add(token_count) > request.context_budget_tokens {
                truncated = true;
                break;
            }
            estimated_tokens = estimated_tokens.saturating_add(token_count);
            fragments.push(fragment);
        }

        let mut memory_fragments = Vec::new();
        if let Some(memory_policy_ref) = request
            .memory_policy_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let Some(memory) = self.memory else {
                return Err(KnowledgeRetrievalServiceError::InvalidRequest(
                    "memory_policy_ref requires a knowledge memory context provider".to_string(),
                ));
            };

            let remaining_tokens = request
                .context_budget_tokens
                .saturating_sub(estimated_tokens);
            if remaining_tokens == 0 {
                truncated = true;
            } else {
                let memory_result = memory
                    .build_memory_context(KnowledgeMemoryContextRequest {
                        tenant_id: request.tenant_id,
                        actor_id: request.actor_id,
                        query: request.query.clone(),
                        memory_policy_ref: memory_policy_ref.to_string(),
                        max_tokens: remaining_tokens,
                    })
                    .await?;
                truncated |= memory_result.truncated;

                for memory_fragment in memory_result.fragments {
                    let token_count = estimate_memory_fragment_tokens(&memory_fragment);
                    if estimated_tokens.saturating_add(token_count) > request.context_budget_tokens
                    {
                        truncated = true;
                        break;
                    }
                    estimated_tokens = estimated_tokens.saturating_add(token_count);
                    memory_fragments.push(memory_fragment);
                }
            }
        }

        let citations = fragments
            .iter()
            .filter_map(|fragment| fragment.citation.clone())
            .collect();

        record_context_pack_completed(request.tenant_id, estimated_tokens, truncated);

        Ok(KnowledgeContextPack {
            context_pack_id: retrieval.retrieval_id,
            retrieval_id: Some(retrieval.retrieval_id),
            query: request.query,
            fragments,
            memory_fragments,
            estimated_tokens,
            citations,
            truncated,
        })
    }

    pub async fn retrieve_persisted(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeRetrievalServiceError> {
        if tenant_id == 0 {
            return Err(KnowledgeRetrievalServiceError::InvalidRequest(
                "tenant_id is required".to_string(),
            ));
        }
        if retrieval_trace_id == 0 {
            return Err(KnowledgeRetrievalServiceError::InvalidRequest(
                "retrieval_trace_id is required".to_string(),
            ));
        }

        let trace = self
            .traces
            .retrieve_trace(tenant_id, retrieval_trace_id)
            .await?;
        let hits = self
            .traces
            .list_trace_hits(tenant_id, retrieval_trace_id)
            .await?;

        Ok(KnowledgeRetrievalResult {
            retrieval_id: trace.retrieval_trace_id,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: trace.retrieval_trace_id,
                status: trace.status,
                latency_ms: trace.latency_ms,
                result_count: trace.result_count,
            }),
            hits: build_fragments_from_trace_hits(&hits),
        })
    }

    async fn persist_trace(
        &self,
        request: &KnowledgeRetrievalRequest,
        normalized_query: &str,
        fragments: &[KnowledgeContextFragment],
        latency_ms: Option<u64>,
    ) -> Result<u64, KnowledgeRetrievalServiceError> {
        let trace_id = self
            .traces
            .create_trace(CreateKnowledgeRetrievalTraceRecord {
                tenant_id: request.tenant_id,
                actor_id: request.actor_id,
                retrieval_profile_id: request.retrieval_profile_id,
                query_hash_sha256_hex: sha256_hash(normalized_query.as_bytes()),
                query_text_redacted: Some(redact_query(normalized_query)),
                request_payload_json: redacted_request_payload(request, normalized_query),
                latency_ms,
                result_count: fragments.len() as u32,
                status: "succeeded".to_string(),
            })
            .await?;

        self.traces
            .create_hits(
                fragments
                    .iter()
                    .map(|fragment| CreateKnowledgeRetrievalHitRecord {
                        tenant_id: request.tenant_id,
                        retrieval_trace_id: trace_id,
                        chunk_id: fragment.chunk_id,
                        document_id: fragment.document_id,
                        document_version_id: fragment.document_version_id,
                        score: fragment.score,
                        result_rank: fragment.rank,
                        match_reason: Some(format!("{:?}", fragment.retrieval_method)),
                        citation_json: fragment
                            .citation
                            .as_ref()
                            .and_then(|citation| serde_json::to_string(citation).ok()),
                        metadata_json: None,
                    })
                    .collect(),
            )
            .await?;

        Ok(trace_id)
    }
}

fn build_fragments_from_trace_hits(
    hits: &[KnowledgeRetrievalTraceHitRecord],
) -> Vec<KnowledgeContextFragment> {
    hits.iter()
        .map(|hit| KnowledgeContextFragment {
            chunk_id: hit.chunk_id,
            document_id: hit.document_id,
            document_version_id: hit.document_version_id,
            space_id: hit.space_id,
            collection_id: hit.collection_id,
            title: hit.title.clone(),
            content: hit.content.clone(),
            score: hit.score,
            rank: hit.result_rank,
            token_count: hit.token_count,
            retrieval_method: hit.retrieval_method,
            citation: hit
                .citation_json
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok()),
        })
        .collect()
}

fn validate_request(
    request: &KnowledgeRetrievalRequest,
) -> Result<(), KnowledgeRetrievalServiceError> {
    if request.tenant_id == 0 {
        return Err(KnowledgeRetrievalServiceError::InvalidRequest(
            "tenant_id is required".to_string(),
        ));
    }
    if is_blank(Some(request.query.as_str())) {
        return Err(KnowledgeRetrievalServiceError::InvalidRequest(
            "query is required".to_string(),
        ));
    }
    if request.bindings.is_empty() {
        return Err(KnowledgeRetrievalServiceError::InvalidRequest(
            "at least one binding is required".to_string(),
        ));
    }
    if request.bindings.iter().any(|binding| binding.space_id == 0) {
        return Err(KnowledgeRetrievalServiceError::InvalidRequest(
            "binding space_id is required".to_string(),
        ));
    }
    Ok(())
}

fn sorted_bindings(bindings: &[KnowledgeRetrievalBinding]) -> Vec<KnowledgeRetrievalBinding> {
    let mut bindings = bindings.to_vec();
    bindings.sort_by_key(|binding| std::cmp::Reverse(binding.priority));
    bindings
}

fn normalize_methods(methods: &[KnowledgeRetrievalMethod]) -> Vec<KnowledgeRetrievalMethod> {
    if methods.is_empty() {
        return vec![KnowledgeRetrievalMethod::Hybrid];
    }
    methods.to_vec()
}

fn normalize_top_k(top_k: Option<u32>) -> u32 {
    top_k.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K)
}

fn sort_hits(hits: &mut [KnowledgeChunkSearchHit]) {
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.chunk_id.cmp(&right.chunk_id))
    });
}

fn build_fragments(
    hits: &[KnowledgeChunkSearchHit],
    include_citations: bool,
) -> Vec<KnowledgeContextFragment> {
    hits.iter()
        .enumerate()
        .map(|(index, hit)| KnowledgeContextFragment {
            chunk_id: hit.chunk_id,
            document_id: hit.document_id,
            document_version_id: hit.document_version_id,
            space_id: hit.space_id,
            collection_id: hit.collection_id,
            title: hit.title.clone(),
            content: hit.content.clone(),
            score: Some(hit.score),
            rank: (index + 1) as u32,
            token_count: hit.token_count,
            retrieval_method: hit.retrieval_method,
            citation: include_citations.then(|| KnowledgeCitation {
                document_id: hit.document_id,
                document_version_id: hit.document_version_id,
                chunk_id: Some(hit.chunk_id),
                title: hit.title.clone(),
                source_uri: hit.source_uri.clone(),
                locator: hit.locator.clone(),
                score: Some(hit.score),
            }),
        })
        .collect()
}

fn append_public_web_fragments(
    fragments: &mut Vec<KnowledgeContextFragment>,
    web_hits: Vec<PublicWebSearchHit>,
    include_citations: bool,
) {
    for hit in web_hits {
        let (document_id, chunk_id) = stable_web_hit_ids(&hit.url);
        let rank = fragments.len() as u32 + 1;
        fragments.push(KnowledgeContextFragment {
            chunk_id,
            document_id,
            document_version_id: None,
            space_id: 0,
            collection_id: None,
            title: hit.title.clone(),
            content: hit.snippet.clone(),
            score: None,
            rank,
            token_count: Some(hit.snippet.split_whitespace().count().max(1) as u32),
            retrieval_method: KnowledgeRetrievalMethod::External,
            citation: include_citations.then(|| KnowledgeCitation {
                document_id,
                document_version_id: None,
                chunk_id: Some(chunk_id),
                title: hit.title,
                source_uri: Some(hit.url),
                locator: Some("public_web".to_string()),
                score: None,
            }),
        });
    }
}

fn estimate_fragment_tokens(fragment: &KnowledgeContextFragment) -> u32 {
    fragment
        .token_count
        .unwrap_or_else(|| fragment.content.split_whitespace().count().max(1) as u32)
}

fn estimate_memory_fragment_tokens(fragment: &KnowledgeMemoryContextFragment) -> u32 {
    fragment
        .token_count
        .unwrap_or_else(|| fragment.content.split_whitespace().count().max(1) as u32)
}

/// Field-whitelisted snapshot of a retrieval request persisted on the trace.
///
/// The raw `query` (user content, potentially PII) and the `actor_id` are never stored;
/// the query is represented only by its SHA-256 hash and a bounded snippet. Bindings are
/// reduced to a count so scope keys cannot leak into the trace payload.
#[derive(Debug, serde::Serialize)]
struct RedactedRetrievalRequestPayload {
    tenant_id: u64,
    retrieval_profile_id: Option<u64>,
    methods: Vec<KnowledgeRetrievalMethod>,
    top_k: Option<u32>,
    include_citations: bool,
    include_trace: bool,
    context_budget_tokens: Option<u32>,
    metadata: Vec<KnowledgeFilter>,
    binding_count: usize,
    query_hash_sha256_hex: String,
}

fn redacted_request_payload(
    request: &KnowledgeRetrievalRequest,
    normalized_query: &str,
) -> Option<String> {
    serde_json::to_string(&RedactedRetrievalRequestPayload {
        tenant_id: request.tenant_id,
        retrieval_profile_id: request.retrieval_profile_id,
        methods: request.methods.clone(),
        top_k: request.top_k,
        include_citations: request.include_citations,
        include_trace: request.include_trace,
        context_budget_tokens: request.context_budget_tokens,
        metadata: request.metadata.clone(),
        binding_count: request.bindings.len(),
        query_hash_sha256_hex: sha256_hash(normalized_query.as_bytes()),
    })
    .ok()
}

/// Bounded, operator-readable query snippet persisted on the trace. This is a deliberate
/// privacy trade-off: full query text never enters the trace, and the snippet stays far
/// below the threshold of reconstructable user content.
fn redact_query(value: &str) -> String {
    const MAX_QUERY_TRACE_CHARS: usize = 128;
    value.chars().take(MAX_QUERY_TRACE_CHARS).collect()
}

#[derive(Debug, Error)]
pub enum KnowledgeRetrievalServiceError {
    #[error("invalid knowledge retrieval request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Backend(#[from] KnowledgeRetrievalBackendError),
    #[error(transparent)]
    MemoryProvider(#[from] KnowledgeMemoryContextProviderError),
    #[error(transparent)]
    TraceStore(#[from] KnowledgeRetrievalTraceStoreError),
}
