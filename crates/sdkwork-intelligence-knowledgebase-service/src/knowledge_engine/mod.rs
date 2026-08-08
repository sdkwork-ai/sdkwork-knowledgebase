//! Native and external knowledge engine implementations for the product SPI.

mod execution_handle;
mod kernel_bridge;
mod okf_native;
mod okf_search;
mod rag_native;
mod space_resolver;

pub use kernel_bridge::{
    format_scoped_document_id, parse_namespace_space_id, parse_scoped_document_id,
    scoped_document_refs,
};
pub use space_resolver::KnowledgeEngineSpaceResolver;

pub use execution_handle::KnowledgeEngineExecutionHandle;
pub use okf_native::{OkfNativeKnowledgeEngine, OkfNativeKnowledgeEngineDeps};
pub use okf_search::{normalize_query, rank_okf_concept, rank_okf_concepts};
pub use rag_native::{RagIndexRebuildDeps, RagNativeKnowledgeEngine};

pub use crate::ports::knowledge_engine::{
    ExternalKnowledgeEngine, InMemoryKnowledgeEngineRegistry, KnowledgeEngine,
    KnowledgeEngineRegistrar, KnowledgeEngineRegistry, OkfBundleEngine, RagKnowledgeEngine,
};

use std::sync::Arc;

use crate::ports::knowledge_document_store::KnowledgeDocumentStore;
use crate::ports::knowledge_drive_object_ref_store::KnowledgeDriveObjectRefStore;
use crate::ports::knowledge_drive_storage::KnowledgeDriveStorage;
use crate::ports::knowledge_drive_workspace::KnowledgeDriveWorkspace;
use crate::ports::knowledge_embedding_store::KnowledgeEmbeddingStore;
use crate::ports::knowledge_index_store::KnowledgeIndexStore;
use crate::ports::knowledge_okf_bundle_file_store::KnowledgeOkfBundleFileStore;
use crate::ports::knowledge_okf_candidate_store::KnowledgeOkfCandidateStore;
use crate::ports::knowledge_okf_concept_link_store::KnowledgeOkfConceptLinkStore;
use crate::ports::knowledge_okf_concept_store::KnowledgeOkfConceptStore;
use crate::ports::knowledge_retrieval_backend::KnowledgeRetrievalBackend;
use crate::ports::knowledge_retrieval_trace_store::KnowledgeRetrievalTraceStore;
use crate::ports::knowledge_source_store::KnowledgeSourceStore;
use crate::ports::knowledge_space_store::KnowledgeSpaceStore;
use crate::ports::okf_concept_revision_metadata_store::OkfConceptRevisionMetadataStore;
use sdkwork_knowledgebase_agent_provider::CloudRouterEmbeddingClient;
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;

pub struct DefaultKnowledgeEngineRegistry {
    registry: InMemoryKnowledgeEngineRegistry,
    okf_native: Arc<OkfNativeKnowledgeEngine>,
    rag_native: Arc<RagNativeKnowledgeEngine>,
    tenant_id: u64,
}

impl DefaultKnowledgeEngineRegistry {
    pub fn registry(&self) -> &InMemoryKnowledgeEngineRegistry {
        &self.registry
    }

    pub fn okf_native(&self) -> &OkfNativeKnowledgeEngine {
        self.okf_native.as_ref()
    }

    pub fn rag_native(&self) -> &RagNativeKnowledgeEngine {
        self.rag_native.as_ref()
    }

    pub fn tenant_id(&self) -> u64 {
        self.tenant_id
    }

    pub async fn rebuild_okf_index(&self, space_id: u64) -> Result<(), KnowledgeEngineError> {
        OkfBundleEngine::rebuild_index(self.okf_native.as_ref(), space_id).await
    }

    pub async fn lint_okf_bundle_report(
        &self,
        space_id: u64,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleLintResult, KnowledgeEngineError>
    {
        use crate::ports::knowledge_engine::OkfBundleEngine;

        OkfBundleEngine::lint_bundle_report(self.okf_native.as_ref(), space_id).await
    }

    pub async fn import_okf_bundle_for_actor(
        &self,
        request: sdkwork_knowledgebase_contract::okf::OkfBundleImportRequest,
        actor: &str,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleImportResult, KnowledgeEngineError>
    {
        self.okf_native
            .import_bundle_for_actor(request, actor)
            .await
    }

    pub async fn export_okf_bundle(
        &self,
        request: sdkwork_knowledgebase_contract::okf::OkfBundleExportRequest,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf_bundle_file::KnowledgeOkfBundleFile,
        KnowledgeEngineError,
    > {
        use crate::ports::knowledge_engine::OkfBundleEngine;

        OkfBundleEngine::export_bundle(self.okf_native.as_ref(), request).await
    }

    pub async fn list_okf_concepts(
        &self,
        space_id: u64,
    ) -> Result<Vec<sdkwork_knowledgebase_contract::okf::OkfConceptSummary>, KnowledgeEngineError>
    {
        use crate::ports::knowledge_engine::OkfBundleEngine;

        OkfBundleEngine::list_concepts(self.okf_native.as_ref(), space_id).await
    }

    pub async fn upsert_okf_concept(
        &self,
        request: sdkwork_knowledgebase_contract::okf::OkfConceptUpsertRequest,
    ) -> Result<sdkwork_knowledgebase_contract::okf::KnowledgeOkfConcept, KnowledgeEngineError>
    {
        use crate::ports::knowledge_engine::OkfBundleEngine;

        OkfBundleEngine::upsert_concept(self.okf_native.as_ref(), request).await
    }

    pub async fn delete_okf_concept(
        &self,
        space_id: u64,
        concept_row_id: u64,
        actor: &str,
    ) -> Result<(), KnowledgeEngineError> {
        use crate::ports::knowledge_engine::OkfBundleEngine;

        OkfBundleEngine::delete_concept(self.okf_native.as_ref(), space_id, concept_row_id, actor)
            .await
    }

    pub async fn publish_okf_concept(
        &self,
        request: sdkwork_knowledgebase_contract::okf::PublishKnowledgeOkfConceptRequest,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConceptPublication,
        KnowledgeEngineError,
    > {
        use crate::ports::knowledge_engine::OkfBundleEngine;

        OkfBundleEngine::publish_concept(self.okf_native.as_ref(), request).await
    }

    pub async fn publish_okf_existing_revision(
        &self,
        request: crate::okf::PublishExistingOkfConceptRevisionRequest,
        drive_space_id: Option<&str>,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConceptPublication,
        KnowledgeEngineError,
    > {
        self.okf_native
            .publish_existing_revision(request, drive_space_id)
            .await
    }

    pub async fn import_okf_bundle_files(
        &self,
        request: crate::okf::ImportOkfBundleRequest,
        drive_space_id: Option<&str>,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleImportResult, KnowledgeEngineError>
    {
        self.okf_native
            .import_bundle_files(request, drive_space_id)
            .await
    }

    pub async fn rebuild_rag_index(&self, space_id: u64) -> Result<usize, KnowledgeEngineError> {
        let Some(rebuild) = self.rag_native.index_rebuild_deps() else {
            return Err(KnowledgeEngineError::Unsupported(
                "rag rebuild_index requires hosted embedding index wiring".to_string(),
            ));
        };

        crate::rag::rebuild_rag_index_for_space(
            self.tenant_id,
            space_id,
            rebuild.index_store.as_ref(),
            rebuild.embedding_store.as_ref(),
            rebuild.embedder.clone(),
        )
        .await
        .map_err(map_rag_index_rebuild_error)
    }

    pub async fn embed_rag_index(
        &self,
        index_id: u64,
        space_id: u64,
        embedder: CloudRouterEmbeddingClient,
    ) -> Result<usize, KnowledgeEngineError> {
        let Some(rebuild) = self.rag_native.index_rebuild_deps() else {
            return Err(KnowledgeEngineError::Unsupported(
                "rag index embedding requires hosted embedding index wiring".to_string(),
            ));
        };

        crate::rag::embed_rag_index_chunks(
            self.tenant_id,
            index_id,
            space_id,
            rebuild.embedding_store.as_ref(),
            embedder,
        )
        .await
        .map_err(map_rag_index_rebuild_error)
    }
}

impl KnowledgeEngineRegistry for DefaultKnowledgeEngineRegistry {
    fn resolve_for_mode(
        &self,
        mode: sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        self.registry.resolve_for_mode(mode)
    }

    fn resolve_by_id(
        &self,
        implementation_id: &str,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        self.registry.resolve_by_id(implementation_id)
    }

    fn list_registered(
        &self,
    ) -> Vec<sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineDescriptor> {
        self.registry.list_registered()
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

pub struct KnowledgeEngineRuntimeDeps {
    pub tenant_id: u64,
    pub okf: OkfNativeKnowledgeEngineDeps,
    pub rag_documents: Arc<dyn KnowledgeDocumentStore>,
    pub retrieval_backend: Arc<dyn KnowledgeRetrievalBackend>,
    pub retrieval_traces: Arc<dyn KnowledgeRetrievalTraceStore>,
    pub rag_index_store: Option<Arc<dyn KnowledgeIndexStore>>,
    pub rag_embedding_store: Option<Arc<dyn KnowledgeEmbeddingStore>>,
    pub rag_embedder: Option<CloudRouterEmbeddingClient>,
    pub external_engines: Vec<Arc<dyn KnowledgeEngine>>,
}

impl KnowledgeEngineRuntimeDeps {
    #[allow(clippy::too_many_arguments)]
    pub fn okf_from_stores(
        concepts: Arc<dyn KnowledgeOkfConceptStore>,
        drive: Arc<dyn KnowledgeDriveStorage>,
        revision_metadata: Arc<dyn OkfConceptRevisionMetadataStore>,
        object_refs: Arc<dyn KnowledgeDriveObjectRefStore>,
        link_store: Arc<dyn KnowledgeOkfConceptLinkStore>,
        candidate_store: Arc<dyn KnowledgeOkfCandidateStore>,
        bundle_file_store: Arc<dyn KnowledgeOkfBundleFileStore>,
        drive_workspace: Arc<dyn KnowledgeDriveWorkspace>,
        source_store: Arc<dyn KnowledgeSourceStore>,
        space_store: Arc<dyn KnowledgeSpaceStore>,
    ) -> OkfNativeKnowledgeEngineDeps {
        OkfNativeKnowledgeEngineDeps {
            concepts,
            drive,
            revision_metadata,
            object_refs,
            link_store,
            candidate_store,
            bundle_file_store,
            drive_workspace,
            source_store,
            space_store,
        }
    }
}

pub fn build_default_registry(
    deps: KnowledgeEngineRuntimeDeps,
) -> Result<DefaultKnowledgeEngineRegistry, KnowledgeEngineError> {
    let mut registry = InMemoryKnowledgeEngineRegistry::new();
    let okf_native = Arc::new(OkfNativeKnowledgeEngine::from_deps(deps.okf));
    registry.register(okf_native.clone())?;

    let mut rag_engine = RagNativeKnowledgeEngine::new(
        deps.tenant_id,
        deps.rag_documents,
        deps.retrieval_backend,
        deps.retrieval_traces,
    );
    if let (Some(index_store), Some(embedding_store)) =
        (deps.rag_index_store, deps.rag_embedding_store)
    {
        rag_engine = rag_engine.with_index_rebuild(RagIndexRebuildDeps {
            index_store,
            embedding_store,
            embedder: deps.rag_embedder,
        });
    }
    let rag_native = Arc::new(rag_engine);
    registry.register(rag_native.clone())?;

    for engine in deps.external_engines {
        registry.register(engine)?;
    }
    Ok(DefaultKnowledgeEngineRegistry {
        registry,
        okf_native,
        rag_native,
        tenant_id: deps.tenant_id,
    })
}

#[async_trait::async_trait]
impl crate::okf::OkfBundleWorkflowEngine for DefaultKnowledgeEngineRegistry {
    async fn rebuild_index(&self, space_id: u64) -> Result<(), KnowledgeEngineError> {
        self.rebuild_okf_index(space_id).await
    }

    async fn lint_bundle_report(
        &self,
        space_id: u64,
    ) -> Result<sdkwork_knowledgebase_contract::okf::OkfBundleLintResult, KnowledgeEngineError>
    {
        self.lint_okf_bundle_report(space_id).await
    }
}
