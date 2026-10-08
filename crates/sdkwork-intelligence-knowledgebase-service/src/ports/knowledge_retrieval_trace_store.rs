use async_trait::async_trait;
use sdkwork_knowledgebase_contract::rag::KnowledgeRetrievalMethod;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeRetrievalTraceStore: Send + Sync {
    async fn create_trace(
        &self,
        record: CreateKnowledgeRetrievalTraceRecord,
    ) -> Result<u64, KnowledgeRetrievalTraceStoreError>;

    async fn create_hits(
        &self,
        records: Vec<CreateKnowledgeRetrievalHitRecord>,
    ) -> Result<(), KnowledgeRetrievalTraceStoreError>;

    async fn retrieve_trace(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStoreError>;

    async fn list_trace_hits(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<Vec<KnowledgeRetrievalTraceHitRecord>, KnowledgeRetrievalTraceStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeRetrievalTraceRecord {
    pub tenant_id: u64,
    pub actor_id: Option<u64>,
    pub retrieval_profile_id: Option<u64>,
    pub query_hash_sha256_hex: String,
    pub query_text_redacted: Option<String>,
    pub request_payload_json: Option<String>,
    pub latency_ms: Option<u64>,
    pub result_count: u32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateKnowledgeRetrievalHitRecord {
    pub tenant_id: u64,
    pub retrieval_trace_id: u64,
    pub chunk_id: u64,
    pub document_id: u64,
    pub document_version_id: Option<u64>,
    pub score: Option<f64>,
    pub result_rank: u32,
    pub match_reason: Option<String>,
    pub citation_json: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRetrievalTraceRecord {
    pub tenant_id: u64,
    pub retrieval_trace_id: u64,
    pub actor_id: Option<u64>,
    pub retrieval_profile_id: Option<u64>,
    pub query_text_redacted: Option<String>,
    pub latency_ms: Option<u64>,
    pub result_count: u32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeRetrievalTraceHitRecord {
    pub chunk_id: u64,
    pub document_id: u64,
    pub document_version_id: Option<u64>,
    pub space_id: u64,
    pub title: String,
    pub content: String,
    pub score: Option<f64>,
    pub result_rank: u32,
    pub token_count: Option<u32>,
    pub retrieval_method: KnowledgeRetrievalMethod,
    pub citation_json: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeRetrievalTraceStoreError {
    #[error("knowledge retrieval trace not found: {0}")]
    NotFound(u64),
    #[error("knowledge retrieval trace store internal error: {0}")]
    Internal(String),
}
