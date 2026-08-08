use async_trait::async_trait;
#[path = "support/okf_engine_deps.rs"]
mod okf_engine_deps_support;
#[path = "support/okf_pagination.rs"]
mod okf_pagination_support;

use okf_engine_deps_support::okf_test_deps;
use okf_pagination_support::validated_okf_test_page_size;
use sdkwork_intelligence_knowledgebase_service::knowledge_engine::{
    build_default_registry, KnowledgeEngineRuntimeDeps, KnowledgeEngineSpaceResolver,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentStore, KnowledgeDocumentStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_engine::KnowledgeEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::{
    AppendKnowledgeOkfLogEntryRecord, CreateKnowledgeOkfConceptRevisionRecord,
    KnowledgeOkfConceptProjection, KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
    MarkKnowledgeOkfConceptCurrentRevisionRecord, UpsertKnowledgeOkfConceptRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderBindingStoreError,
    KnowledgeEngineProviderScope, RecordKnowledgeEngineProviderTestResult,
    ResolvedKnowledgeEngineProviderCredential,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_credential_resolver::{
    KnowledgeEngineProviderCredential, KnowledgeEngineProviderCredentialAccessContext,
    KnowledgeEngineProviderCredentialError, KnowledgeEngineProviderCredentialResolver,
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
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore, KnowledgeSpaceStoreError,
};
use sdkwork_intelligence_knowledgebase_service::provider_binding::{
    KnowledgeEngineProviderBindingService, KNOWLEDGE_PLATFORM_MANAGE_PERMISSION,
};
use sdkwork_knowledgebase_contract::knowledge_engine::{
    descriptor_for_external_search_read, KnowledgeEngineCapability, KnowledgeEngineDescriptor,
    KnowledgeEngineDocument, KnowledgeEngineDocumentList, KnowledgeEngineError,
    KnowledgeEngineHealth, KnowledgeEngineHealthStatus, KnowledgeEngineId,
    KnowledgeEngineListRequest, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
    KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, KnowledgeOkfConceptRevision, OkfConceptSummary, OkfLogEntry,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    CreateKnowledgeEngineProviderBindingRequest,
    CreateKnowledgeEngineProviderCredentialReferenceRequest, KnowledgeEngineDataScope,
    KnowledgeEngineExecutionContext, KnowledgeEngineProviderBinding,
    KnowledgeEngineProviderBindingList, KnowledgeEngineProviderBindingState,
    KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderCredentialReferenceList,
    ListKnowledgeEngineProviderBindingsRequest,
    ListKnowledgeEngineProviderCredentialReferencesRequest,
    RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    RotateKnowledgeEngineProviderCredentialReferenceRequest,
    UpdateKnowledgeEngineProviderBindingRequest,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::KnowledgeSpace;
use sdkwork_knowledgebase_engine_dify::{DifyConnectorConfig, DifyKnowledgeEngine};
use sdkwork_knowledgebase_engine_ragflow::{RagflowConnectorConfig, RagflowKnowledgeEngine};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

struct MockOkfConceptStore;

#[async_trait]
impl KnowledgeOkfConceptStore for MockOkfConceptStore {
    async fn upsert_concept(
        &self,
        _record: UpsertKnowledgeOkfConceptRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn create_revision(
        &self,
        _record: CreateKnowledgeOkfConceptRevisionRecord,
    ) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn next_revision_no(
        &self,
        _concept_row_id: u64,
    ) -> Result<u64, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn mark_current_revision(
        &self,
        _record: MarkKnowledgeOkfConceptCurrentRevisionRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn list_concept_summaries(
        &self,
        _space_id: u64,
        _limit: Option<u32>,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
        Ok(Vec::new())
    }

    async fn list_concept_summaries_page(
        &self,
        _space_id: u64,
        _cursor: Option<String>,
        page_size: u32,
    ) -> Result<(Vec<OkfConceptSummary>, Option<String>, bool), KnowledgeOkfConceptStoreError> {
        validated_okf_test_page_size(page_size)?;
        Ok((Vec::new(), None, false))
    }

    async fn list_concept_revisions_page(
        &self,
        _concept_row_id: u64,
        _cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeOkfConceptRevision>, Option<u64>, bool), KnowledgeOkfConceptStoreError>
    {
        validated_okf_test_page_size(page_size)?;
        Ok((Vec::new(), None, false))
    }

    async fn append_log_entry(
        &self,
        _record: AppendKnowledgeOkfLogEntryRecord,
    ) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn list_log_entries(
        &self,
        _space_id: u64,
    ) -> Result<Vec<OkfLogEntry>, KnowledgeOkfConceptStoreError> {
        Ok(Vec::new())
    }

    async fn batch_concept_projections_by_paths(
        &self,
        _space_id: u64,
        _logical_paths: Vec<String>,
    ) -> Result<Vec<KnowledgeOkfConceptProjection>, KnowledgeOkfConceptStoreError> {
        Ok(Vec::new())
    }

    async fn mark_concept_deleted(
        &self,
        _space_id: u64,
        _concept_row_id: u64,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }
}

struct MockDriveStorage;

struct MockDocumentStore;

#[async_trait]
impl KnowledgeDocumentStore for MockDocumentStore {
    async fn create_document(
        &self,
        _record: CreateKnowledgeDocumentRecord,
    ) -> Result<
        sdkwork_knowledgebase_contract::document::KnowledgeDocument,
        KnowledgeDocumentStoreError,
    > {
        Err(KnowledgeDocumentStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }
}

#[async_trait]
impl KnowledgeDriveStorage for MockDriveStorage {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Ok(KnowledgeObjectRef {
            storage_provider_id: "mock".to_string(),
            bucket: "mock".to_string(),
            object_key: request.logical_path.clone(),
            logical_path: request.logical_path,
            object_role: request.object_role,
            content_type: request.content_type,
            size_bytes: request.body.len() as u64,
            checksum_sha256_hex: request.checksum_sha256_hex,
            etag: None,
            version_id: None,
        })
    }

    async fn head_object(
        &self,
        request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::NotFound(
            request
                .logical_path
                .unwrap_or_else(|| request.object_key.clone()),
        ))
    }

    async fn get_object_text(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        Err(KnowledgeStorageError::NotFound(
            object_ref.logical_path.clone(),
        ))
    }
}

struct MockRetrievalBackend;

#[async_trait]
impl KnowledgeRetrievalBackend for MockRetrievalBackend {
    async fn search_chunks(
        &self,
        _request: KnowledgeChunkSearchRequest,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        Ok(Vec::new())
    }
}

struct MockRetrievalTraceStore;

#[async_trait]
impl KnowledgeRetrievalTraceStore for MockRetrievalTraceStore {
    async fn create_trace(
        &self,
        _record: CreateKnowledgeRetrievalTraceRecord,
    ) -> Result<u64, KnowledgeRetrievalTraceStoreError> {
        Ok(1)
    }

    async fn create_hits(
        &self,
        _records: Vec<CreateKnowledgeRetrievalHitRecord>,
    ) -> Result<(), KnowledgeRetrievalTraceStoreError> {
        Ok(())
    }

    async fn retrieve_trace(
        &self,
        _tenant_id: u64,
        _retrieval_trace_id: u64,
    ) -> Result<KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStoreError> {
        Err(KnowledgeRetrievalTraceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn list_trace_hits(
        &self,
        _tenant_id: u64,
        _retrieval_trace_id: u64,
    ) -> Result<Vec<KnowledgeRetrievalTraceHitRecord>, KnowledgeRetrievalTraceStoreError> {
        Ok(Vec::new())
    }
}

struct MockSpaceStore {
    spaces: HashMap<u64, KnowledgeSpace>,
}

#[async_trait]
impl KnowledgeSpaceStore for MockSpaceStore {
    async fn create_space(
        &self,
        _record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn get_space(&self, space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        self.spaces
            .get(&space_id)
            .cloned()
            .ok_or_else(|| KnowledgeSpaceStoreError::Internal("missing space".to_string()))
    }

    async fn mark_drive_space_bound(
        &self,
        _space_id: u64,
        _record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn mark_okf_bundle_initialized(
        &self,
        _space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn mark_space_deleted(&self, _space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn update_space(
        &self,
        _space_id: u64,
        _record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }
}

#[derive(Default)]
struct MockProviderBindingStore {
    active: HashMap<u64, KnowledgeEngineProviderBinding>,
    credential_lookup_count: Arc<AtomicUsize>,
}

#[async_trait]
impl KnowledgeEngineProviderBindingStore for MockProviderBindingStore {
    async fn create_credential_reference(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _actor_id: &str,
        _request: CreateKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        Err(unsupported_provider_store())
    }

    async fn resolve_credential_reference(
        &self,
        _scope: KnowledgeEngineProviderScope,
        credential_reference_id: u64,
        implementation_id: &str,
    ) -> Result<ResolvedKnowledgeEngineProviderCredential, KnowledgeEngineProviderBindingStoreError>
    {
        self.credential_lookup_count.fetch_add(1, Ordering::SeqCst);
        Ok(ResolvedKnowledgeEngineProviderCredential {
            credential_reference_id,
            implementation_id: implementation_id.to_string(),
            reference_locator: "test://provider-credential".to_string(),
            version: 1,
        })
    }

    async fn get_credential_reference(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _credential_reference_id: u64,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        Err(unsupported_provider_store())
    }

    async fn list_credential_references(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _request: ListKnowledgeEngineProviderCredentialReferencesRequest,
    ) -> Result<
        KnowledgeEngineProviderCredentialReferenceList,
        KnowledgeEngineProviderBindingStoreError,
    > {
        Err(unsupported_provider_store())
    }

    async fn rotate_credential_reference(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _credential_reference_id: u64,
        _actor_id: &str,
        _request: RotateKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        Err(unsupported_provider_store())
    }

    async fn revoke_credential_reference(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _credential_reference_id: u64,
        _actor_id: &str,
        _request: RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        Err(unsupported_provider_store())
    }

    async fn create_binding(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _actor_id: &str,
        _request: CreateKnowledgeEngineProviderBindingRequest,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        Err(unsupported_provider_store())
    }

    async fn get_binding(
        &self,
        _scope: KnowledgeEngineProviderScope,
        binding_id: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        self.active
            .values()
            .find(|binding| binding.id == binding_id)
            .cloned()
            .ok_or(KnowledgeEngineProviderBindingStoreError::NotFound(
                binding_id,
            ))
    }

    async fn get_active_binding_for_space(
        &self,
        scope: KnowledgeEngineProviderScope,
        space_id: u64,
    ) -> Result<Option<KnowledgeEngineProviderBinding>, KnowledgeEngineProviderBindingStoreError>
    {
        Ok(self
            .active
            .get(&space_id)
            .filter(|binding| {
                binding.tenant_id == scope.tenant_id
                    && binding.organization_id == scope.organization_id
            })
            .cloned())
    }

    async fn list_bindings(
        &self,
        scope: KnowledgeEngineProviderScope,
        request: ListKnowledgeEngineProviderBindingsRequest,
    ) -> Result<KnowledgeEngineProviderBindingList, KnowledgeEngineProviderBindingStoreError> {
        let cursor = request
            .cursor
            .as_deref()
            .map(str::parse::<u64>)
            .transpose()
            .map_err(|_| {
                KnowledgeEngineProviderBindingStoreError::InvalidRequest(
                    "invalid test cursor".to_string(),
                )
            })?;
        let page_size = request.page_size.unwrap_or(20).clamp(1, 200) as usize;
        let mut items = self
            .active
            .values()
            .filter(|binding| {
                binding.tenant_id == scope.tenant_id
                    && binding.organization_id == scope.organization_id
                    && request
                        .space_id
                        .is_none_or(|space_id| binding.space_id == space_id)
                    && request
                        .lifecycle_state
                        .is_none_or(|state| binding.lifecycle_state == state)
                    && cursor.is_none_or(|cursor| binding.id < cursor)
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by_key(|binding| std::cmp::Reverse(binding.id));
        let has_more = items.len() > page_size;
        items.truncate(page_size);
        let next_cursor = has_more
            .then(|| items.last().map(|binding| binding.id.to_string()))
            .flatten();
        Ok(KnowledgeEngineProviderBindingList { items, next_cursor })
    }

    async fn update_draft_binding(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _binding_id: u64,
        _actor_id: &str,
        _request: UpdateKnowledgeEngineProviderBindingRequest,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        Err(unsupported_provider_store())
    }

    async fn begin_binding_test(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _binding_id: u64,
        _actor_id: &str,
        _expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        Err(unsupported_provider_store())
    }

    async fn record_binding_test_result(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _binding_id: u64,
        _result: RecordKnowledgeEngineProviderTestResult,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        Err(unsupported_provider_store())
    }

    async fn activate_binding(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _binding_id: u64,
        _actor_id: &str,
        _expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        Err(unsupported_provider_store())
    }

    async fn disable_binding(
        &self,
        _scope: KnowledgeEngineProviderScope,
        _binding_id: u64,
        _actor_id: &str,
        _expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        Err(unsupported_provider_store())
    }
}

#[derive(Clone, Default)]
struct ConcurrencyHealthEngine {
    active_probes: Arc<AtomicUsize>,
    max_active_probes: Arc<AtomicUsize>,
}

#[async_trait]
impl KnowledgeEngine for ConcurrencyHealthEngine {
    fn descriptor(&self) -> KnowledgeEngineDescriptor {
        descriptor_for_external_search_read("concurrency", "Concurrency Test Provider")
    }

    fn bind_provider(
        &self,
        _binding: &KnowledgeEngineProviderBinding,
        _credential: Option<KnowledgeEngineProviderCredential>,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        Ok(Arc::new(self.clone()))
    }

    async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
        let active = self.active_probes.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active_probes.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        self.active_probes.fetch_sub(1, Ordering::SeqCst);
        Ok(KnowledgeEngineHealth {
            implementation_id: KnowledgeEngineId::external("concurrency").0,
            status: KnowledgeEngineHealthStatus::Available,
            detail: None,
        })
    }

    async fn search(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineSearchRequest,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "not used by health concurrency tests".to_string(),
        ))
    }

    async fn read_document(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineReadRequest,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "not used by health concurrency tests".to_string(),
        ))
    }

    async fn list_documents(
        &self,
        _context: &KnowledgeEngineExecutionContext,
        _request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        Err(KnowledgeEngineError::Unsupported(
            "not used by health concurrency tests".to_string(),
        ))
    }
}

#[tokio::test]
async fn native_space_mode_is_not_overridden_by_provider_binding() {
    let registry = Arc::new(wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(Arc::new(MockOkfConceptStore), Arc::new(MockDriveStorage)),
        rag_documents: Arc::new(MockDocumentStore),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![Arc::new(configured_dify_engine())],
    }));

    let resolver = KnowledgeEngineSpaceResolver::new(
        registry,
        Arc::new(MockSpaceStore {
            spaces: HashMap::from([(
                9,
                KnowledgeSpace {
                    id: 9,
                    uuid: "space-9".to_string(),
                    name: "External Space".to_string(),
                    description: None,
                    drive_space_id: Some("drive-9".to_string()),
                    status: sdkwork_knowledgebase_contract::space::KnowledgeSpaceStatus::Active,
                    okf_bundle_initialized: false,
                    knowledge_mode: KnowledgeAgentKnowledgeMode::Rag,
                },
            )]),
        }),
        Arc::new(MockProviderBindingStore {
            active: HashMap::from([(9, active_binding(1, 9, "dify"))]),
            ..Default::default()
        }),
        provider_scope(),
        credential_resolver(),
    );

    let engine = resolver
        .resolve_for_space(9, None)
        .await
        .expect("resolve native RAG mode");
    assert_eq!(
        engine.descriptor().implementation_id,
        KnowledgeEngineId::RAG_NATIVE
    );
}

#[tokio::test]
async fn explicit_native_override_wins_for_external_space() {
    let registry = Arc::new(wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(Arc::new(MockOkfConceptStore), Arc::new(MockDriveStorage)),
        rag_documents: Arc::new(MockDocumentStore),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![Arc::new(configured_dify_engine())],
    }));
    let resolver = KnowledgeEngineSpaceResolver::new(
        registry,
        Arc::new(MockSpaceStore {
            spaces: HashMap::from([(11, external_space(11))]),
        }),
        Arc::new(MockProviderBindingStore {
            active: HashMap::from([(11, active_binding(2, 11, "dify"))]),
            ..Default::default()
        }),
        provider_scope(),
        credential_resolver(),
    );

    let engine = resolver
        .resolve_for_space(11, Some(KnowledgeAgentKnowledgeMode::OkfBundle))
        .await
        .expect("resolve explicit OKF override");
    assert_eq!(
        engine.descriptor().implementation_id,
        KnowledgeEngineId::OKF_NATIVE
    );
}

#[tokio::test]
async fn external_space_uses_the_single_active_binding_as_selection_authority() {
    let registry = Arc::new(wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(Arc::new(MockOkfConceptStore), Arc::new(MockDriveStorage)),
        rag_documents: Arc::new(MockDocumentStore),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![
            Arc::new(configured_dify_engine()),
            Arc::new(configured_ragflow_engine()),
        ],
    }));
    let resolver = KnowledgeEngineSpaceResolver::new(
        registry,
        Arc::new(MockSpaceStore {
            spaces: HashMap::from([(12, external_space(12))]),
        }),
        Arc::new(MockProviderBindingStore {
            active: HashMap::from([(12, active_binding(3, 12, "dify"))]),
            ..Default::default()
        }),
        provider_scope(),
        credential_resolver(),
    );

    let engine = resolver
        .resolve_for_space(12, None)
        .await
        .expect("resolve explicit active Provider binding");
    assert_eq!(
        engine.descriptor().implementation_id,
        KnowledgeEngineId::external("dify").0
    );
}

#[tokio::test]
async fn resolve_for_external_mode_space_requires_active_provider_binding() {
    let registry = Arc::new(wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(Arc::new(MockOkfConceptStore), Arc::new(MockDriveStorage)),
        rag_documents: Arc::new(MockDocumentStore),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![Arc::new(configured_dify_engine())],
    }));

    let external_space = external_space(11);

    let resolver = KnowledgeEngineSpaceResolver::new(
        registry.clone(),
        Arc::new(MockSpaceStore {
            spaces: HashMap::from([(11, external_space.clone())]),
        }),
        Arc::new(MockProviderBindingStore {
            active: HashMap::from([(11, active_binding(4, 11, "dify"))]),
            ..Default::default()
        }),
        provider_scope(),
        credential_resolver(),
    );

    let engine = resolver
        .resolve_for_space(11, None)
        .await
        .expect("resolve external mode space");
    assert_eq!(
        engine.descriptor().implementation_id,
        KnowledgeEngineId::external("dify").0
    );
    assert_eq!(
        engine.descriptor().agent_provider_id,
        KnowledgeEngineId::external_agent_provider("dify")
    );

    let missing_binding = KnowledgeEngineSpaceResolver::new(
        registry,
        Arc::new(MockSpaceStore {
            spaces: HashMap::from([(11, external_space)]),
        }),
        Arc::new(MockProviderBindingStore::default()),
        provider_scope(),
        credential_resolver(),
    );
    let result = missing_binding.resolve_for_space(11, None).await;
    assert!(result.is_err());
    assert!(result
        .err()
        .expect("error value")
        .to_string()
        .contains("no active external Provider binding"));
}

#[tokio::test]
async fn execution_authorization_precedes_credential_lookup_and_provider_http() {
    let credential_lookup_count = Arc::new(AtomicUsize::new(0));
    let credential_resolve_count = Arc::new(AtomicUsize::new(0));
    let binding_store = Arc::new(MockProviderBindingStore {
        active: HashMap::from([(13, active_binding(5, 13, "dify"))]),
        credential_lookup_count: credential_lookup_count.clone(),
    });
    let credential_resolver = Arc::new(StaticCredentialResolver {
        resolve_count: credential_resolve_count.clone(),
    });
    let registry = Arc::new(wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(Arc::new(MockOkfConceptStore), Arc::new(MockDriveStorage)),
        rag_documents: Arc::new(MockDocumentStore),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![Arc::new(configured_dify_engine())],
    }));
    let resolver = KnowledgeEngineSpaceResolver::new(
        registry,
        Arc::new(MockSpaceStore {
            spaces: HashMap::from([(13, external_space(13))]),
        }),
        binding_store,
        provider_scope(),
        credential_resolver,
    );

    let handle = resolver
        .resolve_for_space(13, None)
        .await
        .expect("resolve external handle without resolving credentials");
    assert_eq!(credential_lookup_count.load(Ordering::SeqCst), 0);
    assert_eq!(credential_resolve_count.load(Ordering::SeqCst), 0);

    let mut denied_context = execution_context(13);
    denied_context.actor_id.clear();
    let denied = handle.search(&denied_context, search_request(13)).await;
    assert!(matches!(
        denied,
        Err(KnowledgeEngineError::PermissionDenied(_))
    ));
    assert_eq!(credential_lookup_count.load(Ordering::SeqCst), 0);
    assert_eq!(credential_resolve_count.load(Ordering::SeqCst), 0);

    let authorized_error = handle
        .search(&execution_context(13), search_request(13))
        .await
        .expect_err("the test endpoint must reject the outbound request");
    assert_eq!(credential_lookup_count.load(Ordering::SeqCst), 1);
    assert_eq!(credential_resolve_count.load(Ordering::SeqCst), 1);
    let rendered = authorized_error.to_string();
    assert!(!rendered.contains("test://provider-credential"));
    assert!(!rendered.contains("test-only"));
}

#[tokio::test]
async fn active_binding_health_probes_are_concurrent_and_bounded() {
    let engine = Arc::new(ConcurrencyHealthEngine::default());
    let registry = Arc::new(wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(Arc::new(MockOkfConceptStore), Arc::new(MockDriveStorage)),
        rag_documents: Arc::new(MockDocumentStore),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![engine.clone()],
    }));
    let active = (1_u64..=9)
        .map(|space_id| (space_id, active_binding(space_id, space_id, "concurrency")))
        .collect();
    let service = KnowledgeEngineProviderBindingService::new(
        Arc::new(MockProviderBindingStore {
            active,
            ..Default::default()
        }),
        registry,
        credential_resolver(),
    );
    let mut context = execution_context(0);
    context.permission_scope = vec![KNOWLEDGE_PLATFORM_MANAGE_PERMISSION.to_string()];

    let summary = service
        .probe_active_bindings_health(&context)
        .await
        .expect("probe active Provider bindings");

    assert_eq!(
        summary.implementation_ids,
        vec![KnowledgeEngineId::external("concurrency").0]
    );
    assert!(!summary.degraded);
    let max_active = engine.max_active_probes.load(Ordering::SeqCst);
    assert!(max_active > 1, "health probes must execute concurrently");
    assert!(
        max_active <= 8,
        "health probe concurrency must stay bounded"
    );
}

fn external_space(space_id: u64) -> KnowledgeSpace {
    KnowledgeSpace {
        id: space_id,
        uuid: format!("space-{space_id}"),
        name: "External Mode Space".to_string(),
        description: None,
        drive_space_id: Some(format!("drive-{space_id}")),
        status: sdkwork_knowledgebase_contract::space::KnowledgeSpaceStatus::Active,
        okf_bundle_initialized: false,
        knowledge_mode: KnowledgeAgentKnowledgeMode::External,
    }
}

fn provider_scope() -> KnowledgeEngineProviderScope {
    KnowledgeEngineProviderScope {
        tenant_id: 1,
        organization_id: 7,
    }
}

fn execution_context(space_id: u64) -> KnowledgeEngineExecutionContext {
    let deadline_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_millis() as u64
        + 30_000;
    KnowledgeEngineExecutionContext {
        tenant_id: 1,
        organization_id: 7,
        actor_id: "knowledge-reader".to_string(),
        permission_scope: vec!["knowledge.read".to_string()],
        data_scope: KnowledgeEngineDataScope {
            allowed_space_ids: vec![space_id],
            allowed_source_ids: Vec::new(),
            allowed_document_ids: Vec::new(),
        },
        space_id,
        binding_id: None,
        trace_id: "trace-credential-ordering".to_string(),
        deadline_unix_ms,
    }
}

fn search_request(space_id: u64) -> KnowledgeEngineSearchRequest {
    KnowledgeEngineSearchRequest {
        tenant_id: 1,
        space_id,
        query: "credential ordering".to_string(),
        top_k: 3,
    }
}

#[derive(Default)]
struct StaticCredentialResolver {
    resolve_count: Arc<AtomicUsize>,
}

#[async_trait]
impl KnowledgeEngineProviderCredentialResolver for StaticCredentialResolver {
    fn validate_reference_locator(
        &self,
        _implementation_id: &str,
        _reference_locator: &str,
    ) -> Result<(), KnowledgeEngineProviderCredentialError> {
        Ok(())
    }

    async fn resolve(
        &self,
        _context: &KnowledgeEngineProviderCredentialAccessContext,
        _reference: &ResolvedKnowledgeEngineProviderCredential,
    ) -> Result<KnowledgeEngineProviderCredential, KnowledgeEngineProviderCredentialError> {
        self.resolve_count.fetch_add(1, Ordering::SeqCst);
        KnowledgeEngineProviderCredential::new("test-only")
    }
}

fn credential_resolver() -> Arc<dyn KnowledgeEngineProviderCredentialResolver> {
    Arc::new(StaticCredentialResolver::default())
}

fn active_binding(id: u64, space_id: u64, provider: &str) -> KnowledgeEngineProviderBinding {
    KnowledgeEngineProviderBinding {
        id,
        uuid: format!("binding-{id}"),
        tenant_id: 1,
        organization_id: 7,
        space_id,
        implementation_id: KnowledgeEngineId::external(provider).0,
        remote_resource_type: "dataset".to_string(),
        remote_resource_id: format!("resource-{id}"),
        credential_reference_id: Some(81),
        lifecycle_state: KnowledgeEngineProviderBindingState::Active,
        capability_snapshot: vec![
            KnowledgeEngineCapability::Health,
            KnowledgeEngineCapability::Search,
            KnowledgeEngineCapability::ReadDocument,
        ],
        capability_snapshot_version: 1,
        last_tested_at: Some("2026-07-20T00:00:00Z".to_string()),
        activated_at: Some("2026-07-20T00:00:01Z".to_string()),
        disabled_at: None,
        last_error_category: None,
        created_by: "tenant-admin".to_string(),
        updated_by: "tenant-admin".to_string(),
        created_at: "2026-07-20T00:00:00Z".to_string(),
        updated_at: "2026-07-20T00:00:01Z".to_string(),
        version: 1,
    }
}

fn unsupported_provider_store() -> KnowledgeEngineProviderBindingStoreError {
    KnowledgeEngineProviderBindingStoreError::Internal("unsupported in test fake".to_string())
}

fn configured_dify_engine() -> DifyKnowledgeEngine {
    DifyKnowledgeEngine::with_config(DifyConnectorConfig {
        base_url: "http://127.0.0.1:1/v1".to_string(),
        api_key: zeroize::Zeroizing::new("test-only".to_string()),
        default_dataset_id: None,
    })
}

fn configured_ragflow_engine() -> RagflowKnowledgeEngine {
    RagflowKnowledgeEngine::with_config(RagflowConnectorConfig {
        base_url: "http://127.0.0.1:1/api/v1".to_string(),
        api_key: zeroize::Zeroizing::new("test-only".to_string()),
        default_dataset_id: None,
    })
}


/// Builds the default engine registry for tests, unwrapping the fallible builder.
fn wrap_build_default_registry(
    deps: sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngineRuntimeDeps,
) -> sdkwork_intelligence_knowledgebase_service::knowledge_engine::DefaultKnowledgeEngineRegistry {
    build_default_registry(deps).expect("default knowledge engine registry must build")
}

