use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_memory_context::{
    KnowledgeMemoryContextProvider, KnowledgeMemoryContextProviderError,
    KnowledgeMemoryContextRequest, KnowledgeMemoryContextResult,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_backend::{
    KnowledgeChunkSearchHit, KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend,
    KnowledgeRetrievalBackendError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_trace_store::{
    CreateKnowledgeRetrievalHitRecord, CreateKnowledgeRetrievalTraceRecord,
    KnowledgeRetrievalTraceHitRecord, KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStore,
    KnowledgeRetrievalTraceStoreError,
};
use sdkwork_intelligence_knowledgebase_service::public_web_search::INCLUDE_PUBLIC_WEB_METADATA_KEY;
use sdkwork_intelligence_knowledgebase_service::retrieval::KnowledgeRetrievalService;
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeContextPackRequest, KnowledgeFilter, KnowledgeMemoryContextFragment,
    KnowledgeRetrievalBinding, KnowledgeRetrievalMethod, KnowledgeRetrievalRequest,
};
use std::sync::Mutex;

#[tokio::test]
async fn retrieval_searches_multiple_bindings_and_persists_trace_hits() {
    let backend = RecordingRetrievalBackend::new(vec![
        hit(
            21,
            7,
            101,
            "Quarterly Revenue",
            "Revenue grew from enterprise renewals.",
            0.82,
            24,
        ),
        hit(
            11,
            5,
            100,
            "Support Playbook",
            "Escalate enterprise renewal issues.",
            0.94,
            18,
        ),
        hit(
            22,
            7,
            102,
            "Quarterly Churn",
            "Churn decreased after support workflow changes.",
            0.79,
            19,
        ),
    ]);
    let traces = RecordingTraceStore::new(701);
    let service = KnowledgeRetrievalService::new(&backend, &traces);

    let result = service
        .retrieve(KnowledgeRetrievalRequest {
            tenant_id: 100001,
            actor_id: Some(30001),
            query: "enterprise renewal support".to_string(),
            retrieval_profile_id: Some(31),
            bindings: vec![
                binding(7, 20, Some(2), Some(0.75)),
                binding(5, 10, Some(1), Some(0.90)),
            ],
            methods: vec![KnowledgeRetrievalMethod::Hybrid],
            top_k: Some(2),
            include_citations: true,
            include_trace: true,
            context_budget_tokens: Some(60),
            metadata: vec![],
        })
        .await
        .unwrap();

    assert_eq!(result.retrieval_id, 701);
    assert_eq!(result.trace.as_ref().unwrap().retrieval_trace_id, 701);
    assert_eq!(result.trace.as_ref().unwrap().result_count, 2);
    assert_eq!(result.hits.len(), 2);
    assert_eq!(result.hits[0].chunk_id, 11);
    assert_eq!(result.hits[0].rank, 1);
    assert_eq!(result.hits[0].citation.as_ref().unwrap().document_id, 100);
    assert_eq!(result.hits[1].chunk_id, 21);
    assert_eq!(result.hits[1].rank, 2);

    assert_eq!(
        backend.requests(),
        vec![
            BackendCall {
                tenant_id: 100001,
                query: "enterprise renewal support".to_string(),
                space_id: 7,
                priority: 20,
                top_k: 2,
                min_score: Some(0.75),
                method: KnowledgeRetrievalMethod::Hybrid,
            },
            BackendCall {
                tenant_id: 100001,
                query: "enterprise renewal support".to_string(),
                space_id: 5,
                priority: 10,
                top_k: 1,
                min_score: Some(0.90),
                method: KnowledgeRetrievalMethod::Hybrid,
            },
        ]
    );

    let trace = traces.created_trace().unwrap();
    assert_eq!(trace.tenant_id, 100001);
    assert_eq!(trace.actor_id, Some(30001));
    assert_eq!(trace.retrieval_profile_id, Some(31));
    assert_eq!(trace.result_count, 2);
    assert!(trace.query_hash_sha256_hex.len() >= 64);
    assert_eq!(
        trace.query_text_redacted.as_deref(),
        Some("enterprise renewal support")
    );

    let persisted_hits = traces.created_hits();
    assert_eq!(persisted_hits.len(), 2);
    assert_eq!(persisted_hits[0].chunk_id, 11);
    assert_eq!(persisted_hits[0].result_rank, 1);
    assert_eq!(persisted_hits[1].chunk_id, 21);
    assert_eq!(persisted_hits[1].result_rank, 2);
}

#[tokio::test]
async fn retrieval_public_web_metadata_is_no_op_when_disabled() {
    let backend = RecordingRetrievalBackend::new(vec![hit(
        41,
        7,
        301,
        "Release Notes",
        "Version 2.0 shipped with retrieval improvements.",
        0.88,
        12,
    )]);
    let traces = RecordingTraceStore::new(702);
    let service = KnowledgeRetrievalService::new(&backend, &traces);

    let result = service
        .retrieve(KnowledgeRetrievalRequest {
            tenant_id: 100001,
            actor_id: None,
            query: "retrieval improvements".to_string(),
            retrieval_profile_id: None,
            bindings: vec![binding(7, 10, None, None)],
            methods: vec![KnowledgeRetrievalMethod::Hybrid],
            top_k: Some(3),
            include_citations: true,
            include_trace: false,
            context_budget_tokens: None,
            metadata: vec![KnowledgeFilter {
                key: INCLUDE_PUBLIC_WEB_METADATA_KEY.to_string(),
                value: "true".to_string(),
            }],
        })
        .await
        .unwrap();

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].chunk_id, 41);
    assert_ne!(
        result.hits[0].retrieval_method,
        KnowledgeRetrievalMethod::External
    );
}

#[tokio::test]
async fn context_pack_uses_retrieval_budget_and_collects_citations() {
    let backend = RecordingRetrievalBackend::new(vec![
        hit(
            31,
            7,
            201,
            "Long Result",
            "first second third fourth fifth",
            0.90,
            5,
        ),
        hit(
            32,
            7,
            202,
            "Budget Overflow",
            "sixth seventh eighth",
            0.89,
            3,
        ),
    ]);
    let traces = RecordingTraceStore::new(801);
    let service = KnowledgeRetrievalService::new(&backend, &traces);

    let pack = service
        .create_context_pack(KnowledgeContextPackRequest {
            tenant_id: 100001,
            actor_id: Some(30001),
            query: "budgeted context".to_string(),
            retrieval_profile_id: None,
            bindings: vec![binding(7, 0, None, None)],
            context_budget_tokens: 5,
            include_citations: true,
            memory_policy_ref: None,
        })
        .await
        .unwrap();

    assert_eq!(pack.context_pack_id, 801);
    assert_eq!(pack.retrieval_id, Some(801));
    assert_eq!(pack.fragments.len(), 1);
    assert_eq!(pack.fragments[0].chunk_id, 31);
    assert_eq!(pack.estimated_tokens, 5);
    assert_eq!(pack.citations.len(), 1);
    assert!(pack.truncated);
}

#[tokio::test]
async fn context_pack_includes_memory_fragments_when_memory_policy_ref_is_present() {
    let backend = RecordingRetrievalBackend::new(vec![hit(
        41,
        7,
        301,
        "Knowledge Result",
        "knowledge context",
        0.91,
        5,
    )]);
    let traces = RecordingTraceStore::new(811);
    let memory = RecordingMemoryContextProvider::new(vec![
        KnowledgeMemoryContextFragment {
            memory_id: "mem-001".to_string(),
            title: Some("Preference".to_string()),
            content: "prefers concise operational answers".to_string(),
            score: Some(0.84),
            rank: 1,
            token_count: Some(4),
            source_uri: Some("memory://mem-001".to_string()),
            policy_ref: Some("memory.session.summary".to_string()),
        },
        KnowledgeMemoryContextFragment {
            memory_id: "mem-002".to_string(),
            title: Some("Overflow".to_string()),
            content: "this memory does not fit the remaining budget".to_string(),
            score: Some(0.72),
            rank: 2,
            token_count: Some(3),
            source_uri: Some("memory://mem-002".to_string()),
            policy_ref: Some("memory.session.summary".to_string()),
        },
    ]);
    let service = KnowledgeRetrievalService::with_memory(&backend, &traces, &memory);

    let pack = service
        .create_context_pack(KnowledgeContextPackRequest {
            tenant_id: 100001,
            actor_id: Some(30001),
            query: "budgeted memory context".to_string(),
            retrieval_profile_id: Some(31),
            bindings: vec![binding(7, 0, None, None)],
            context_budget_tokens: 9,
            include_citations: false,
            memory_policy_ref: Some("memory.session.summary".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(
        memory.requests(),
        vec![MemoryCall {
            tenant_id: 100001,
            actor_id: Some(30001),
            query: "budgeted memory context".to_string(),
            memory_policy_ref: "memory.session.summary".to_string(),
            max_tokens: 4,
        }]
    );
    assert_eq!(pack.fragments.len(), 1);
    assert_eq!(pack.fragments[0].chunk_id, 41);
    assert_eq!(pack.memory_fragments.len(), 1);
    assert_eq!(pack.memory_fragments[0].memory_id, "mem-001");
    assert_eq!(pack.estimated_tokens, 9);
    assert!(pack.truncated);
}

#[tokio::test]
async fn retrieval_rejects_blank_query_and_missing_bindings() {
    let backend = RecordingRetrievalBackend::new(vec![]);
    let traces = RecordingTraceStore::new(901);
    let service = KnowledgeRetrievalService::new(&backend, &traces);

    let blank_query_error = service
        .retrieve(KnowledgeRetrievalRequest {
            tenant_id: 100001,
            actor_id: None,
            query: "  ".to_string(),
            retrieval_profile_id: None,
            bindings: vec![binding(7, 0, None, None)],
            methods: vec![],
            top_k: None,
            include_citations: true,
            include_trace: true,
            context_budget_tokens: None,
            metadata: vec![],
        })
        .await
        .unwrap_err();
    assert!(blank_query_error.to_string().contains("query is required"));

    let missing_binding_error = service
        .retrieve(KnowledgeRetrievalRequest {
            tenant_id: 100001,
            actor_id: None,
            query: "valid".to_string(),
            retrieval_profile_id: None,
            bindings: vec![],
            methods: vec![],
            top_k: None,
            include_citations: true,
            include_trace: true,
            context_budget_tokens: None,
            metadata: vec![],
        })
        .await
        .unwrap_err();
    assert!(missing_binding_error
        .to_string()
        .contains("at least one binding is required"));
}

#[tokio::test]
async fn retrieval_can_reconstruct_persisted_trace_and_hits() {
    let backend = RecordingRetrievalBackend::new(vec![]);
    let traces = RecordingTraceStore::with_stored_trace(
        701,
        KnowledgeRetrievalTraceRecord {
            tenant_id: 100001,
            retrieval_trace_id: 701,
            actor_id: Some(9001),
            retrieval_profile_id: Some(31),
            query_text_redacted: Some("enterprise renewal support".to_string()),
            latency_ms: Some(25),
            result_count: 1,
            status: "succeeded".to_string(),
        },
        vec![KnowledgeRetrievalTraceHitRecord {
            chunk_id: 11,
            document_id: 100,
            document_version_id: Some(1100),
            space_id: 7,
            title: "Support Playbook".to_string(),
            content: "Escalate enterprise renewal issues.".to_string(),
            score: Some(0.94),
            result_rank: 1,
            token_count: Some(18),
            retrieval_method: KnowledgeRetrievalMethod::Hybrid,
            citation_json: None,
        }],
    );
    let service = KnowledgeRetrievalService::new(&backend, &traces);

    let result = service.retrieve_persisted(100001, 701).await.unwrap();

    assert_eq!(result.retrieval_id, 701);
    assert_eq!(result.trace.as_ref().unwrap().retrieval_trace_id, 701);
    assert_eq!(result.trace.as_ref().unwrap().result_count, 1);
    assert_eq!(result.hits[0].chunk_id, 11);
    assert_eq!(result.hits[0].document_id, 100);
    assert_eq!(result.hits[0].rank, 1);
}

#[derive(Debug, Clone, PartialEq)]
struct BackendCall {
    tenant_id: u64,
    query: String,
    space_id: u64,
    priority: i32,
    top_k: u32,
    min_score: Option<f64>,
    method: KnowledgeRetrievalMethod,
}

struct RecordingRetrievalBackend {
    hits: Vec<KnowledgeChunkSearchHit>,
    requests: Mutex<Vec<BackendCall>>,
}

impl RecordingRetrievalBackend {
    fn new(hits: Vec<KnowledgeChunkSearchHit>) -> Self {
        Self {
            hits,
            requests: Mutex::new(vec![]),
        }
    }

    fn requests(&self) -> Vec<BackendCall> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeRetrievalBackend for RecordingRetrievalBackend {
    async fn search_chunks(
        &self,
        request: KnowledgeChunkSearchRequest,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        self.requests.lock().unwrap().push(BackendCall {
            tenant_id: request.tenant_id,
            query: request.query.clone(),
            space_id: request.binding.space_id,
            priority: request.binding.priority,
            top_k: request.top_k,
            min_score: request.binding.min_score,
            method: request.method,
        });

        Ok(self
            .hits
            .iter()
            .filter(|hit| hit.space_id == request.binding.space_id)
            .filter(|hit| {
                request
                    .binding
                    .min_score
                    .map(|min_score| hit.score >= min_score)
                    .unwrap_or(true)
            })
            .take(request.top_k as usize)
            .cloned()
            .collect())
    }
}

struct RecordingTraceStore {
    next_trace_id: u64,
    trace: Mutex<Option<CreateKnowledgeRetrievalTraceRecord>>,
    hits: Mutex<Vec<CreateKnowledgeRetrievalHitRecord>>,
    stored_trace: Mutex<Option<KnowledgeRetrievalTraceRecord>>,
    stored_hits: Mutex<Vec<KnowledgeRetrievalTraceHitRecord>>,
}

impl RecordingTraceStore {
    fn new(next_trace_id: u64) -> Self {
        Self {
            next_trace_id,
            trace: Mutex::new(None),
            hits: Mutex::new(vec![]),
            stored_trace: Mutex::new(None),
            stored_hits: Mutex::new(vec![]),
        }
    }

    fn with_stored_trace(
        next_trace_id: u64,
        trace: KnowledgeRetrievalTraceRecord,
        hits: Vec<KnowledgeRetrievalTraceHitRecord>,
    ) -> Self {
        Self {
            next_trace_id,
            trace: Mutex::new(None),
            hits: Mutex::new(vec![]),
            stored_trace: Mutex::new(Some(trace)),
            stored_hits: Mutex::new(hits),
        }
    }

    fn created_trace(&self) -> Option<CreateKnowledgeRetrievalTraceRecord> {
        self.trace.lock().unwrap().clone()
    }

    fn created_hits(&self) -> Vec<CreateKnowledgeRetrievalHitRecord> {
        self.hits.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeRetrievalTraceStore for RecordingTraceStore {
    async fn create_trace(
        &self,
        record: CreateKnowledgeRetrievalTraceRecord,
    ) -> Result<u64, KnowledgeRetrievalTraceStoreError> {
        *self.trace.lock().unwrap() = Some(record);
        Ok(self.next_trace_id)
    }

    async fn create_hits(
        &self,
        records: Vec<CreateKnowledgeRetrievalHitRecord>,
    ) -> Result<(), KnowledgeRetrievalTraceStoreError> {
        *self.hits.lock().unwrap() = records;
        Ok(())
    }

    async fn retrieve_trace(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStoreError> {
        self.stored_trace
            .lock()
            .unwrap()
            .clone()
            .filter(|trace| {
                trace.tenant_id == tenant_id && trace.retrieval_trace_id == retrieval_trace_id
            })
            .ok_or(KnowledgeRetrievalTraceStoreError::NotFound(
                retrieval_trace_id,
            ))
    }

    async fn list_trace_hits(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<Vec<KnowledgeRetrievalTraceHitRecord>, KnowledgeRetrievalTraceStoreError> {
        let trace = self.retrieve_trace(tenant_id, retrieval_trace_id).await?;
        Ok(self
            .stored_hits
            .lock()
            .unwrap()
            .iter()
            .filter(|_| trace.tenant_id == tenant_id)
            .cloned()
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MemoryCall {
    tenant_id: u64,
    actor_id: Option<u64>,
    query: String,
    memory_policy_ref: String,
    max_tokens: u32,
}

struct RecordingMemoryContextProvider {
    fragments: Vec<KnowledgeMemoryContextFragment>,
    requests: Mutex<Vec<MemoryCall>>,
}

impl RecordingMemoryContextProvider {
    fn new(fragments: Vec<KnowledgeMemoryContextFragment>) -> Self {
        Self {
            fragments,
            requests: Mutex::new(vec![]),
        }
    }

    fn requests(&self) -> Vec<MemoryCall> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeMemoryContextProvider for RecordingMemoryContextProvider {
    async fn build_memory_context(
        &self,
        request: KnowledgeMemoryContextRequest,
    ) -> Result<KnowledgeMemoryContextResult, KnowledgeMemoryContextProviderError> {
        self.requests.lock().unwrap().push(MemoryCall {
            tenant_id: request.tenant_id,
            actor_id: request.actor_id,
            query: request.query.clone(),
            memory_policy_ref: request.memory_policy_ref.clone(),
            max_tokens: request.max_tokens,
        });
        Ok(KnowledgeMemoryContextResult {
            fragments: self.fragments.clone(),
            truncated: false,
        })
    }
}

fn binding(
    space_id: u64,
    priority: i32,
    top_k: Option<u32>,
    min_score: Option<f64>,
) -> KnowledgeRetrievalBinding {
    KnowledgeRetrievalBinding {
        space_id,
        source_filter: None,
        document_filter: None,
        priority,
        top_k,
        min_score,
    }
}

fn hit(
    chunk_id: u64,
    space_id: u64,
    document_id: u64,
    title: &str,
    content: &str,
    score: f64,
    token_count: u32,
) -> KnowledgeChunkSearchHit {
    KnowledgeChunkSearchHit {
        chunk_id,
        document_id,
        document_version_id: Some(document_id + 1000),
        space_id,
        title: title.to_string(),
        content: content.to_string(),
        score,
        token_count: Some(token_count),
        locator: Some(format!("chunk:{chunk_id}")),
        source_uri: Some(format!("kb://documents/{document_id}")),
        retrieval_method: KnowledgeRetrievalMethod::Hybrid,
        match_reason: Some("hybrid".to_string()),
    }
}
