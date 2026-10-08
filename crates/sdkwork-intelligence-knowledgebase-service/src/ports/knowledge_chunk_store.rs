use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeChunkRecord {
    pub space_id: u64,
    pub document_id: u64,
    pub document_version_id: u64,
    pub chunk_index: u32,
    pub content_text: String,
    pub content_hash: String,
    pub token_count: Option<u32>,
    pub locator: Option<String>,
}

#[async_trait]
pub trait KnowledgeChunkStore: Send + Sync {
    async fn replace_version_chunks(
        &self,
        document_version_id: u64,
        chunks: Vec<CreateKnowledgeChunkRecord>,
    ) -> Result<usize, KnowledgeChunkStoreError>;

    async fn list_chunk_ids_for_document_version(
        &self,
        document_version_id: u64,
    ) -> Result<Vec<u64>, KnowledgeChunkStoreError>;

    async fn list_chunk_texts_for_document_version(
        &self,
        document_version_id: u64,
    ) -> Result<Vec<String>, KnowledgeChunkStoreError>;

    async fn list_chunk_id_content_for_document_version(
        &self,
        document_version_id: u64,
    ) -> Result<Vec<(u64, String)>, KnowledgeChunkStoreError> {
        let chunk_ids = self
            .list_chunk_ids_for_document_version(document_version_id)
            .await?;
        let chunk_texts = self
            .list_chunk_texts_for_document_version(document_version_id)
            .await?;
        if chunk_ids.len() != chunk_texts.len() {
            return Err(KnowledgeChunkStoreError::Internal(
                "chunk ids and texts length mismatch".to_string(),
            ));
        }
        Ok(chunk_ids.into_iter().zip(chunk_texts).collect())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeChunkStoreError {
    #[error("knowledge chunk store invalid record: {0}")]
    InvalidRecord(String),
    #[error("knowledge chunk store internal error: {0}")]
    Internal(String),
}
