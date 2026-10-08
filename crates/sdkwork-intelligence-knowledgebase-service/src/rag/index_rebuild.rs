use sdkwork_knowledgebase_agent_provider::CloudRouterEmbeddingClient;
use thiserror::Error;

use crate::knowledge_embedding_build::{
    KnowledgeEmbeddingBuildService, KnowledgeEmbeddingBuildServiceError,
};
use crate::ports::knowledge_embedding_store::KnowledgeEmbeddingStore;
use crate::ports::knowledge_index_store::{KnowledgeIndexStore, KnowledgeIndexStoreError};

#[derive(Debug, Error)]
pub enum RagIndexRebuildError {
    #[error("rag index rebuild requires embedding client wiring: {0}")]
    MissingEmbedder(String),
    #[error(transparent)]
    IndexStore(#[from] KnowledgeIndexStoreError),
    #[error(transparent)]
    Build(#[from] KnowledgeEmbeddingBuildServiceError),
}

pub async fn embed_rag_index_chunks(
    tenant_id: u64,
    index_id: u64,
    space_id: u64,
    embedding_store: &dyn KnowledgeEmbeddingStore,
    embedder: CloudRouterEmbeddingClient,
) -> Result<usize, RagIndexRebuildError> {
    let build = KnowledgeEmbeddingBuildService::new(embedding_store, embedder);
    build
        .embed_space_chunks(tenant_id, index_id, space_id, None, None)
        .await
        .map_err(RagIndexRebuildError::Build)
}

pub async fn rebuild_rag_index_for_space(
    tenant_id: u64,
    space_id: u64,
    index_store: &dyn KnowledgeIndexStore,
    embedding_store: &dyn KnowledgeEmbeddingStore,
    embedder: Option<CloudRouterEmbeddingClient>,
) -> Result<usize, RagIndexRebuildError> {
    let embedder = embedder.ok_or_else(|| {
        RagIndexRebuildError::MissingEmbedder(
            "cloud-router embedding client is required for rag index rebuild".to_string(),
        )
    })?;

    let index = index_store
        .get_or_create_active_vector_index(space_id)
        .await?;

    embed_rag_index_chunks(
        tenant_id,
        index.index_id,
        space_id,
        embedding_store,
        embedder,
    )
    .await
}
