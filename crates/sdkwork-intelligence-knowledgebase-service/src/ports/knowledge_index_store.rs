use async_trait::async_trait;
use sdkwork_knowledgebase_contract::rag::KnowledgeIndex;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeIndexStore: Send + Sync {
    async fn get_index(&self, index_id: u64) -> Result<KnowledgeIndex, KnowledgeIndexStoreError>;

    async fn get_or_create_active_vector_index(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeIndex, KnowledgeIndexStoreError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeIndexStoreError {
    #[error("knowledge index was not found: {0}")]
    NotFound(String),
    #[error("knowledge index store internal error: {0}")]
    Internal(String),
}
