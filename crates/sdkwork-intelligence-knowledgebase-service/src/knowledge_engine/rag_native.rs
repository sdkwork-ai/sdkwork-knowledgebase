use async_trait::async_trait;
use sdkwork_knowledgebase_agent_provider::CloudRouterEmbeddingClient;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    descriptor_for_mode, KnowledgeEngineDescriptor, KnowledgeEngineDocument,
    KnowledgeEngineDocumentList, KnowledgeEngineDocumentRef, KnowledgeEngineError,
    KnowledgeEngineHealth, KnowledgeEngineHealthStatus, KnowledgeEngineId,
    KnowledgeEngineListRequest, KnowledgeEngineReadRequest, KnowledgeEngineSearchHit,
    KnowledgeEngineSearchRequest, KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineExecutionContext;
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentKnowledgeMode, KnowledgeRetrievalBinding, KnowledgeRetrievalMethod,
};
use std::sync::Arc;

use crate::ports::knowledge_document_store::KnowledgeDocumentStore;
use crate::ports::knowledge_embedding_store::KnowledgeEmbeddingStore;
use crate::ports::knowledge_engine::RagKnowledgeEngine;
use crate::ports::knowledge_index_store::KnowledgeIndexStore;
use crate::ports::knowledge_retrieval_backend::{
    KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend, KnowledgeRetrievalBackendError,
};
use crate::ports::knowledge_retrieval_trace_store::KnowledgeRetrievalTraceStore;
use crate::rag::rebuild_rag_index_for_space;
use crate::retrieval::KnowledgeRetrievalService;
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeContextPack, KnowledgeContextPackRequest, KnowledgeRetrievalRequest,
    KnowledgeRetrievalResult,
};

use super::KnowledgeEngine;

pub struct RagIndexRebuildDeps {
    pub index_store: Arc<dyn KnowledgeIndexStore>,
    pub embedding_store: Arc<dyn KnowledgeEmbeddingStore>,
    pub embedder: Option<CloudRouterEmbeddingClient>,
}

pub struct RagNativeKnowledgeEngine {
    tenant_id: u64,
    documents: Arc<dyn KnowledgeDocumentStore>,
    backend: Arc<dyn KnowledgeRetrievalBackend>,
    traces: Arc<dyn KnowledgeRetrievalTraceStore>,
    index_rebuild: Option<RagIndexRebuildDeps>,
}

impl RagNativeKnowledgeEngine {
    pub fn new(
        tenant_id: u64,
        documents: Arc<dyn KnowledgeDocumentStore>,
        backend: Arc<dyn KnowledgeRetrievalBackend>,
        traces: Arc<dyn KnowledgeRetrievalTraceStore>,
    ) -> Self {
        Self {
            tenant_id,
            documents,
            backend,
            traces,
            index_rebuild: None,
        }
    }

    pub fn with_index_rebuild(mut self, index_rebuild: RagIndexRebuildDeps) -> Self {
        self.index_rebuild = Some(index_rebuild);
        self
    }

    pub(crate) fn index_rebuild_deps(&self) -> Option<&RagIndexRebuildDeps> {
        self.index_rebuild.as_ref()
    }

    fn retrieval_service(&self) -> KnowledgeRetrievalService<'_> {
        KnowledgeRetrievalService::new(self.backend.as_ref(), self.traces.as_ref())
    }
}

#[async_trait]
impl KnowledgeEngine for RagNativeKnowledgeEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        descriptor_for_mode(KnowledgeAgentKnowledgeMode::Rag)
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        Ok(KnowledgeEngineHealth {
            implementation_id: KnowledgeEngineId::RAG_NATIVE.to_string(),
            status: KnowledgeEngineHealthStatus::Available,
            detail: Some("native RAG retrieval engine".to_string()),
        })
    }

    async fn search(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineSearchRequest,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let hits = self
            .backend
            .search_chunks(KnowledgeChunkSearchRequest {
                tenant_id: self.tenant_id,
                query: request.query.clone(),
                binding: KnowledgeRetrievalBinding {
                    space_id: request.space_id,
                    source_filter: None,
                    document_filter: None,
                    priority: 0,
                    top_k: Some(request.top_k),
                    min_score: None,
                },
                method: KnowledgeRetrievalMethod::Hybrid,
                top_k: request.top_k.max(1),
                query_embedding: None,
            })
            .await
            .map_err(map_retrieval_backend_error)?;

        let search_hits = hits
            .into_iter()
            .map(|hit| KnowledgeEngineSearchHit {
                document: KnowledgeEngineDocumentRef {
                    document_id: hit.document_id.to_string(),
                    title: hit.title,
                    source_uri: hit.source_uri,
                },
                snippet: hit.content,
                score: Some(hit.score),
            })
            .collect();

        Ok(KnowledgeEngineSearchResult {
            implementation_id: KnowledgeEngineId::RAG_NATIVE.to_string(),
            hits: search_hits,
        })
    }

    async fn read_document(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineReadRequest,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let document_id = request.document_id.parse::<u64>().map_err(|_| {
            KnowledgeEngineError::Validation(format!(
                "rag document_id must be numeric: {}",
                request.document_id
            ))
        })?;

        let document = self
            .documents
            .get_document_by_id(document_id)
            .await
            .map_err(map_document_store_error)?;

        if document.space_id != request.space_id {
            return Err(KnowledgeEngineError::NotFound(format!(
                "rag document not found in space {}: {}",
                request.space_id, request.document_id
            )));
        }

        let hits = self
            .backend
            .search_chunks(KnowledgeChunkSearchRequest {
                tenant_id: self.tenant_id,
                query: document.title.clone(),
                binding: KnowledgeRetrievalBinding {
                    space_id: request.space_id,
                    source_filter: None,
                    document_filter: None,
                    priority: 0,
                    top_k: Some(8),
                    min_score: None,
                },
                method: KnowledgeRetrievalMethod::Exact,
                top_k: 8,
                query_embedding: None,
            })
            .await
            .map_err(map_retrieval_backend_error)?;

        let matching = hits
            .into_iter()
            .find(|hit| hit.document_id == document_id)
            .ok_or_else(|| {
                KnowledgeEngineError::NotFound(format!(
                    "rag indexed content not found for document: {}",
                    request.document_id
                ))
            })?;

        Ok(KnowledgeEngineDocument {
            document_id: document.id.to_string(),
            title: document.title,
            content: matching.content,
            source_uri: matching.source_uri,
        })
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        let documents = self
            .documents
            .list_documents_for_space(request.space_id, request.limit.max(1))
            .await
            .map_err(map_document_store_error)?;

        let items = documents
            .into_iter()
            .map(|document| KnowledgeEngineDocumentRef {
                document_id: document.id.to_string(),
                title: document.title,
                source_uri: document.original_file_drive_node_id,
            })
            .collect();

        Ok(KnowledgeEngineDocumentList { items })
    }
}

#[async_trait]
impl RagKnowledgeEngine for RagNativeKnowledgeEngine {
    async fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeEngineError> {
        self.retrieval_service()
            .retrieve(request)
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))
    }

    async fn create_context_pack(
        &self,
        request: KnowledgeContextPackRequest,
    ) -> Result<KnowledgeContextPack, KnowledgeEngineError> {
        self.retrieval_service()
            .create_context_pack(request)
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))
    }

    async fn rebuild_index(&self, space_id: u64) -> Result<(), KnowledgeEngineError> {
        let Some(rebuild) = &self.index_rebuild else {
            return Err(KnowledgeEngineError::Unsupported(
                "rag rebuild_index requires hosted embedding index wiring".to_string(),
            ));
        };

        rebuild_rag_index_for_space(
            self.tenant_id,
            space_id,
            rebuild.index_store.as_ref(),
            rebuild.embedding_store.as_ref(),
            rebuild.embedder.clone(),
        )
        .await
        .map_err(map_rag_index_rebuild_error)?;

        Ok(())
    }
}

fn map_retrieval_backend_error(error: KnowledgeRetrievalBackendError) -> KnowledgeEngineError {
    KnowledgeEngineError::Internal(error.to_string())
}

fn map_document_store_error(
    error: crate::ports::knowledge_document_store::KnowledgeDocumentStoreError,
) -> KnowledgeEngineError {
    match error {
        crate::ports::knowledge_document_store::KnowledgeDocumentStoreError::InvalidRecord(
            message,
        ) => KnowledgeEngineError::Validation(message),
        crate::ports::knowledge_document_store::KnowledgeDocumentStoreError::Unsupported(
            message,
        ) => KnowledgeEngineError::Unsupported(message),
        crate::ports::knowledge_document_store::KnowledgeDocumentStoreError::QuotaExceeded(
            error,
        ) => KnowledgeEngineError::Validation(error.to_string()),
        crate::ports::knowledge_document_store::KnowledgeDocumentStoreError::NotFound(message) => {
            KnowledgeEngineError::NotFound(message)
        }
        crate::ports::knowledge_document_store::KnowledgeDocumentStoreError::Internal(message) => {
            KnowledgeEngineError::Internal(message)
        }
    }
}

fn map_rag_index_rebuild_error(error: crate::rag::RagIndexRebuildError) -> KnowledgeEngineError {
    match error {
        crate::rag::RagIndexRebuildError::MissingEmbedder(message) => {
            KnowledgeEngineError::Unsupported(message)
        }
        crate::rag::RagIndexRebuildError::IndexStore(error) => {
            KnowledgeEngineError::Internal(error.to_string())
        }
        crate::rag::RagIndexRebuildError::Build(error) => {
            KnowledgeEngineError::Internal(error.to_string())
        }
    }
}
