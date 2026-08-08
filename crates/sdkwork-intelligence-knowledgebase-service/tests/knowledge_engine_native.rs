use async_trait::async_trait;
#[path = "support/okf_engine_deps.rs"]
mod okf_engine_deps_support;
#[path = "support/okf_pagination.rs"]
mod okf_pagination_support;

use okf_engine_deps_support::{okf_test_deps, UnavailableLinkStore, UnavailableSpaceStore};
use okf_pagination_support::validated_okf_test_page_size;
use sdkwork_intelligence_knowledgebase_service::knowledge_engine::{
    build_default_registry, KnowledgeEngine, KnowledgeEngineRegistry, KnowledgeEngineRuntimeDeps,
    OkfNativeKnowledgeEngine,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentStore, KnowledgeDocumentStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::{
    AppendKnowledgeOkfLogEntryRecord, CreateKnowledgeOkfConceptRevisionRecord,
    KnowledgeOkfConceptProjection, KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
    MarkKnowledgeOkfConceptCurrentRevisionRecord, UpsertKnowledgeOkfConceptRecord,
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
use sdkwork_knowledgebase_contract::document::KnowledgeDocument;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineHealthStatus, KnowledgeEngineId, KnowledgeEngineListRequest,
    KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, KnowledgeOkfConceptRevision, OkfConceptSummary, OkfLogEntry,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    KnowledgeEngineDataScope, KnowledgeEngineExecutionContext,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use std::collections::HashMap;
use std::sync::Arc;

struct MockOkfConceptStore {
    summaries: HashMap<u64, Vec<OkfConceptSummary>>,
}

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
        space_id: u64,
        limit: Option<u32>,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
        let mut summaries = self.summaries.get(&space_id).cloned().unwrap_or_default();
        if let Some(limit) = limit {
            summaries.truncate(limit.max(1) as usize);
        }
        Ok(summaries)
    }

    async fn list_concept_summaries_page(
        &self,
        space_id: u64,
        cursor: Option<String>,
        page_size: u32,
    ) -> Result<(Vec<OkfConceptSummary>, Option<String>, bool), KnowledgeOkfConceptStoreError> {
        let page_size = validated_okf_test_page_size(page_size)?;
        let fetch_size = page_size + 1;
        let mut summaries = Vec::with_capacity(fetch_size);

        for summary in self
            .summaries
            .get(&space_id)
            .into_iter()
            .flatten()
            .filter(|summary| match cursor.as_ref() {
                Some(cursor) => summary.concept_id.as_str() > cursor.as_str(),
                None => true,
            })
        {
            let index = summaries
                .partition_point(|item: &OkfConceptSummary| item.concept_id <= summary.concept_id);
            summaries.insert(index, summary.clone());
            if summaries.len() > fetch_size {
                summaries.pop();
            }
        }

        let has_more = summaries.len() > page_size;
        summaries.truncate(page_size);
        let next_cursor = if has_more {
            summaries.last().map(|item| item.concept_id.clone())
        } else {
            None
        };
        Ok((summaries, next_cursor, has_more))
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

struct MockDriveStorage {
    objects: HashMap<String, String>,
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
        let logical_path = request
            .logical_path
            .clone()
            .unwrap_or_else(|| request.object_key.clone());
        if self.objects.contains_key(&logical_path) {
            Ok(KnowledgeObjectRef {
                storage_provider_id: "mock".to_string(),
                bucket: "mock".to_string(),
                object_key: logical_path.clone(),
                logical_path,
                object_role: request.object_role,
                content_type: "text/markdown".to_string(),
                size_bytes: 0,
                checksum_sha256_hex: None,
                etag: None,
                version_id: None,
            })
        } else {
            Err(KnowledgeStorageError::NotFound(logical_path))
        }
    }

    async fn get_object_text(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        self.objects
            .get(&object_ref.logical_path)
            .cloned()
            .ok_or_else(|| KnowledgeStorageError::NotFound(object_ref.logical_path.clone()))
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

struct MockRetrievalBackend;

struct MockDocumentStore {
    documents: HashMap<u64, KnowledgeDocument>,
}

#[async_trait]
impl KnowledgeDocumentStore for MockDocumentStore {
    async fn create_document(
        &self,
        _record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        Err(KnowledgeDocumentStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn get_document_by_id(
        &self,
        document_id: u64,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        self.documents.get(&document_id).cloned().ok_or_else(|| {
            KnowledgeDocumentStoreError::Internal(format!(
                "missing knowledge document: {document_id}"
            ))
        })
    }

    async fn list_documents_for_space(
        &self,
        space_id: u64,
        limit: u32,
    ) -> Result<Vec<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        Ok(self
            .documents
            .values()
            .filter(|document| document.space_id == space_id)
            .take(limit.max(1) as usize)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl KnowledgeRetrievalBackend for MockRetrievalBackend {
    async fn search_chunks(
        &self,
        request: KnowledgeChunkSearchRequest,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        if request.query.contains("missing") {
            return Ok(vec![]);
        }

        Ok(vec![KnowledgeChunkSearchHit {
            chunk_id: 1,
            document_id: 42,
            document_version_id: Some(1),
            space_id: request.binding.space_id,
            collection_id: None,
            title: "Sample Doc".to_string(),
            content: "sample content".to_string(),
            score: 0.9,
            token_count: Some(2),
            locator: None,
            source_uri: Some("kb://42".to_string()),
            retrieval_method: request.method,
            match_reason: None,
        }])
    }
}

#[tokio::test]
async fn default_registry_registers_native_engines() {
    let engines = wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(
            Arc::new(MockOkfConceptStore {
                summaries: HashMap::new(),
            }),
            Arc::new(MockDriveStorage {
                objects: HashMap::new(),
            }),
        ),
        rag_documents: Arc::new(MockDocumentStore {
            documents: HashMap::new(),
        }),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: Vec::new(),
    });

    let descriptors = engines.list_registered();
    assert_eq!(descriptors.len(), 2);
    assert!(engines
        .resolve_by_id(&KnowledgeEngineId::external("dify").0)
        .is_err());
    assert!(engines
        .resolve_by_id(&KnowledgeEngineId::external("ragflow").0)
        .is_err());

    let okf = engines
        .resolve_for_mode(KnowledgeAgentKnowledgeMode::OkfBundle)
        .expect("okf engine");
    assert_eq!(
        okf.descriptor().implementation_id,
        KnowledgeEngineId::OKF_NATIVE
    );

    let rag = engines
        .resolve_for_mode(KnowledgeAgentKnowledgeMode::Rag)
        .expect("rag engine");
    assert_eq!(
        rag.descriptor().implementation_id,
        KnowledgeEngineId::RAG_NATIVE
    );
}

#[tokio::test]
async fn runtime_registers_explicit_ragflow_adapter() {
    use sdkwork_knowledgebase_engine_ragflow::{RagflowConnectorConfig, RagflowKnowledgeEngine};

    let engines = wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(
            Arc::new(MockOkfConceptStore {
                summaries: HashMap::new(),
            }),
            Arc::new(MockDriveStorage {
                objects: HashMap::new(),
            }),
        ),
        rag_documents: Arc::new(MockDocumentStore {
            documents: HashMap::new(),
        }),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![Arc::new(RagflowKnowledgeEngine::with_config(
            RagflowConnectorConfig {
                base_url: "http://127.0.0.1:1".to_string(),
                api_key: Default::default(),
                default_dataset_id: None,
            },
        ))],
    });

    let ragflow = engines
        .resolve_by_id(&KnowledgeEngineId::external("ragflow").0)
        .expect("ragflow engine");
    assert_eq!(
        ragflow.descriptor().display_name,
        "RAGFlow (external adapter)"
    );
}

#[tokio::test]
async fn runtime_registers_explicit_dify_adapter() {
    use sdkwork_knowledgebase_engine_dify::{DifyConnectorConfig, DifyKnowledgeEngine};

    let engines = wrap_build_default_registry(KnowledgeEngineRuntimeDeps {
        tenant_id: 1,
        okf: okf_test_deps(
            Arc::new(MockOkfConceptStore {
                summaries: HashMap::new(),
            }),
            Arc::new(MockDriveStorage {
                objects: HashMap::new(),
            }),
        ),
        rag_documents: Arc::new(MockDocumentStore {
            documents: HashMap::new(),
        }),
        retrieval_backend: Arc::new(MockRetrievalBackend),
        retrieval_traces: Arc::new(MockRetrievalTraceStore),
        rag_index_store: None,
        rag_embedding_store: None,
        rag_embedder: None,
        external_engines: vec![Arc::new(DifyKnowledgeEngine::with_config(
            DifyConnectorConfig {
                base_url: "http://127.0.0.1:1".to_string(),
                api_key: Default::default(),
                default_dataset_id: None,
            },
        ))],
    });

    let dify = engines
        .resolve_by_id(&KnowledgeEngineId::external("dify").0)
        .expect("dify engine");
    assert_eq!(dify.descriptor().display_name, "Dify (external adapter)");
}

#[tokio::test]
async fn okf_native_engine_search_and_read() {
    let summaries = vec![OkfConceptSummary {
        title: "Rust Ownership".to_string(),
        concept_id: "concept-ownership".to_string(),
        concept_type: "concept".to_string(),
        logical_path: "concepts/ownership.md".to_string(),
        bundle_relative_path: "concepts/ownership.md".to_string(),
        description: "Explains ownership rules".to_string(),
        source_count: 1,
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        tags: vec!["rust".to_string()],
    }];

    let mut objects = HashMap::new();
    objects.insert(
        "concepts/ownership.md".to_string(),
        "# Ownership".to_string(),
    );

    let engine = OkfNativeKnowledgeEngine::from_deps(okf_test_deps(
        Arc::new(MockOkfConceptStore {
            summaries: HashMap::from([(7, summaries)]),
        }),
        Arc::new(MockDriveStorage { objects }),
    ));

    let health = engine.health().await.expect("health");
    assert_eq!(health.status, KnowledgeEngineHealthStatus::Available);

    let search = engine
        .search(
            &native_execution_context(7),
            KnowledgeEngineSearchRequest {
                tenant_id: 1,
                space_id: 7,
                query: "ownership".to_string(),
                top_k: 5,
            },
        )
        .await
        .expect("search");
    assert_eq!(search.hits.len(), 1);
    assert_eq!(search.hits[0].document.document_id, "concept-ownership");

    let document = engine
        .read_document(
            &native_execution_context(7),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 7,
                document_id: "concept-ownership".to_string(),
            },
        )
        .await
        .expect("read");
    assert_eq!(document.content, "# Ownership");

    let listed = engine
        .list_documents(
            &native_execution_context(7),
            KnowledgeEngineListRequest {
                tenant_id: 1,
                space_id: 7,
                limit: 10,
            },
        )
        .await
        .expect("list");
    assert_eq!(listed.items.len(), 1);
}

#[tokio::test]
async fn okf_native_search_propagates_link_store_failure() {
    let mut deps = okf_test_deps(
        Arc::new(MockOkfConceptStore {
            summaries: HashMap::new(),
        }),
        Arc::new(MockDriveStorage {
            objects: HashMap::new(),
        }),
    );
    deps.link_store = Arc::new(UnavailableLinkStore);
    let engine = OkfNativeKnowledgeEngine::from_deps(deps);

    let error = engine
        .search(
            &native_execution_context(7),
            KnowledgeEngineSearchRequest {
                tenant_id: 1,
                space_id: 7,
                query: "ownership".to_string(),
                top_k: 5,
            },
        )
        .await
        .expect_err("link repository failures must not become an empty link graph");

    assert!(error.to_string().contains("test link store unavailable"));
}

#[tokio::test]
async fn okf_native_search_propagates_space_store_failure() {
    let mut deps = okf_test_deps(
        Arc::new(MockOkfConceptStore {
            summaries: HashMap::new(),
        }),
        Arc::new(MockDriveStorage {
            objects: HashMap::new(),
        }),
    );
    deps.space_store = Arc::new(UnavailableSpaceStore);
    let engine = OkfNativeKnowledgeEngine::from_deps(deps);

    let error = engine
        .search(
            &native_execution_context(7),
            KnowledgeEngineSearchRequest {
                tenant_id: 1,
                space_id: 7,
                query: "ownership".to_string(),
                top_k: 5,
            },
        )
        .await
        .expect_err("space repository failures must not become an unbound Drive lookup");

    assert!(error.to_string().contains("test space store unavailable"));
}

fn native_execution_context(space_id: u64) -> KnowledgeEngineExecutionContext {
    let now_ms = sdkwork_utils_rust::to_unix_millis(sdkwork_utils_rust::now());
    KnowledgeEngineExecutionContext {
        tenant_id: 1,
        organization_id: 1,
        actor_id: "native-engine-test".to_string(),
        permission_scope: vec!["knowledge.read".to_string()],
        data_scope: KnowledgeEngineDataScope {
            allowed_space_ids: vec![space_id],
            allowed_source_ids: Vec::new(),
            allowed_document_ids: Vec::new(),
        },
        space_id,
        binding_id: None,
        trace_id: "trace-native-engine-test".to_string(),
        deadline_unix_ms: u64::try_from(now_ms).expect("test clock") + 60_000,
    }
}

#[tokio::test]
async fn rag_native_engine_lists_documents_from_store() {
    use sdkwork_intelligence_knowledgebase_service::knowledge_engine::RagNativeKnowledgeEngine;
    use sdkwork_knowledgebase_contract::document::{
        KnowledgeDocument, KnowledgeDocumentState, KnowledgeDocumentVersionState,
        KnowledgeDocumentVisibility,
    };

    let document = KnowledgeDocument {
        id: 42,
        space_id: 7,
        collection_id: 1,
        source_id: None,
        original_file_drive_node_id: Some("drive-node-42".to_string()),
        title: "Sample Doc".to_string(),
        mime_type: Some("text/markdown".to_string()),
        language: None,
        current_version_id: Some(1),
        visibility: KnowledgeDocumentVisibility::Space,
        content_state: KnowledgeDocumentState::Ready,
        index_state: KnowledgeDocumentVersionState::Succeeded,
    };

    let engine = RagNativeKnowledgeEngine::new(
        1,
        Arc::new(MockDocumentStore {
            documents: HashMap::from([(42, document)]),
        }),
        Arc::new(MockRetrievalBackend),
        Arc::new(MockRetrievalTraceStore),
    );

    let listed = engine
        .list_documents(
            &native_execution_context(7),
            KnowledgeEngineListRequest {
                tenant_id: 1,
                space_id: 7,
                limit: 10,
            },
        )
        .await
        .expect("list");
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].document_id, "42");
    assert_eq!(listed.items[0].title, "Sample Doc");
}

#[tokio::test]
async fn rag_native_rebuild_index_without_wiring_is_unsupported() {
    use sdkwork_intelligence_knowledgebase_service::knowledge_engine::{
        RagKnowledgeEngine, RagNativeKnowledgeEngine,
    };
    use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;

    let engine = RagNativeKnowledgeEngine::new(
        1,
        Arc::new(MockDocumentStore {
            documents: HashMap::new(),
        }),
        Arc::new(MockRetrievalBackend),
        Arc::new(MockRetrievalTraceStore),
    );

    let error = engine
        .rebuild_index(7)
        .await
        .expect_err("rebuild without wiring");
    assert!(matches!(error, KnowledgeEngineError::Unsupported(_)));
}


/// Builds the default engine registry for tests, unwrapping the fallible builder.
fn wrap_build_default_registry(
    deps: sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngineRuntimeDeps,
) -> sdkwork_intelligence_knowledgebase_service::knowledge_engine::DefaultKnowledgeEngineRegistry {
    build_default_registry(deps).expect("default knowledge engine registry must build")
}

