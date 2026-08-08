use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ChunkEmbeddingUpsertRequest {
    pub tenant_id: u64,
    pub index_id: u64,
    pub chunk_id: u64,
    pub vector: Vec<f32>,
    pub provider_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChunkEmbeddingIndexRequest {
    pub tenant_id: u64,
    pub index_id: u64,
    pub chunk_id: u64,
    pub content: Option<String>,
    pub embedding_provider_id: Option<String>,
    pub embedding_model: Option<String>,
    pub index_embedding_model: Option<String>,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum KnowledgeEmbeddingStoreError {
    #[error("knowledge embedding store internal error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait KnowledgeEmbeddingStore: Send + Sync {
    async fn upsert_chunk_embedding(
        &self,
        request: ChunkEmbeddingUpsertRequest,
    ) -> Result<(), KnowledgeEmbeddingStoreError>;

    async fn upsert_chunk_embeddings_batch(
        &self,
        requests: &[ChunkEmbeddingUpsertRequest],
    ) -> Result<(), KnowledgeEmbeddingStoreError> {
        for request in requests {
            self.upsert_chunk_embedding(request.clone()).await?;
        }
        Ok(())
    }

    async fn load_chunk_content(
        &self,
        chunk_id: u64,
    ) -> Result<Option<String>, KnowledgeEmbeddingStoreError>;

    async fn list_active_chunk_id_content_page(
        &self,
        space_id: u64,
        after_chunk_id: u64,
        limit: u32,
    ) -> Result<Vec<(u64, String)>, KnowledgeEmbeddingStoreError>;
}
