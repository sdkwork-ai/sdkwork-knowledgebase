use async_trait::async_trait;
#[path = "support/okf_pagination.rs"]
mod okf_pagination_support;

use okf_pagination_support::validated_okf_test_page_size;
use sdkwork_intelligence_knowledgebase_service::okf::{
    rebuild_bundle_index_for_space, run_okf_compile_workflow, run_okf_eval_workflow,
    OkfBundleWorkflowDeps, OkfBundleWorkflowEngine,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_workspace::{
    EnsureKnowledgeDriveNodesRequest, KnowledgeDriveWorkspace, KnowledgeDriveWorkspaceError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_bundle_file_store::{
    CreateKnowledgeOkfBundleFileRecord, KnowledgeOkfBundleFileStore,
    KnowledgeOkfBundleFileStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::{
    AppendKnowledgeOkfLogEntryRecord, CreateKnowledgeOkfConceptRevisionRecord,
    KnowledgeOkfConceptProjection, KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
    MarkKnowledgeOkfConceptCurrentRevisionRecord, UpsertKnowledgeOkfConceptRecord,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_source_store::{
    CreateKnowledgeSourceRecord, KnowledgeSourceStore, KnowledgeSourceStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore, KnowledgeSpaceStoreError,
    UpdateKnowledgeSpaceRecord,
};
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;
use sdkwork_knowledgebase_contract::okf::{OkfBundleLintResult, OkfConceptSummary, OkfLogEntry};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::source::KnowledgeSource;
use sdkwork_knowledgebase_contract::space::{KnowledgeSpace, KnowledgeSpaceStatus};
use sdkwork_knowledgebase_contract::KnowledgeOkfBundleFile;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

#[tokio::test]
async fn compile_workflow_refreshes_standard_bundle_catalog_and_drive_nodes() {
    let drive = MemoryDrive::default();
    let concepts = MemoryOkfConceptStore::default();
    let spaces = FixedSpaceStore::new(KnowledgeSpace {
        id: 7,
        uuid: "space-7".to_string(),
        created_by: 0,
        name: "Workflow Space".to_string(),
        description: None,
        drive_space_id: Some("drv-kb-007".to_string()),
        status: KnowledgeSpaceStatus::Active,
        okf_bundle_initialized: true,
        knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
    });
    let file_entries = MemoryOkfBundleFileStore::default();
    let workspace = MemoryDriveWorkspace::default();
    let engine = RecordingWorkflowEngine::new(&drive, &concepts, &spaces);
    let sources = MemorySourceStore;

    let deps = OkfBundleWorkflowDeps {
        concepts: &concepts,
        drive: &drive,
        space_store: &spaces,
        source_store: &sources,
        link_store: None,
        bundle_file_store: Some(&file_entries),
        drive_workspace: Some(&workspace),
        engine: Some(&engine),
    };

    run_okf_compile_workflow(deps, 7, None, "tester")
        .await
        .unwrap();

    assert!(drive.has_path("okf/schema/AGENTS.md"));
    assert!(drive.has_path("okf/index.md"));
    assert!(file_entries.paths().contains(&"okf/index.md".to_string()));
    assert!(file_entries.paths().contains(&"okf/log.md".to_string()));
    assert!(workspace.paths().contains(&"okf/index.md".to_string()));
    assert_eq!(engine.rebuild_calls(), vec![7]);
    assert_eq!(concepts.log_count(), 1);
    assert_eq!(drive.put_count("okf/index.md"), 1);
    assert_eq!(drive.put_count("okf/log.md"), 1);
}

#[tokio::test]
async fn eval_workflow_refreshes_catalog_after_linting() {
    let drive = MemoryDrive::default();
    let concepts = MemoryOkfConceptStore::default();
    let spaces = FixedSpaceStore::new(KnowledgeSpace {
        id: 9,
        uuid: "space-9".to_string(),
        created_by: 0,
        name: "Eval Space".to_string(),
        description: None,
        drive_space_id: Some("drv-kb-009".to_string()),
        status: KnowledgeSpaceStatus::Active,
        okf_bundle_initialized: true,
        knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
    });
    let file_entries = MemoryOkfBundleFileStore::default();
    let workspace = MemoryDriveWorkspace::default();
    let engine = RecordingWorkflowEngine::new(&drive, &concepts, &spaces);
    let sources = MemorySourceStore;

    let deps = OkfBundleWorkflowDeps {
        concepts: &concepts,
        drive: &drive,
        space_store: &spaces,
        source_store: &sources,
        link_store: None,
        bundle_file_store: Some(&file_entries),
        drive_workspace: Some(&workspace),
        engine: Some(&engine),
    };

    let lint = run_okf_eval_workflow(deps, 9, "reviewer").await.unwrap();

    assert_eq!(lint.conformance, "pass");
    assert!(drive.has_path("okf/log.md"));
    assert!(file_entries
        .paths()
        .contains(&"okf/schema/AGENTS.md".to_string()));
    assert_eq!(engine.rebuild_calls(), vec![9]);
    assert_eq!(concepts.log_count(), 1);
}

struct FixedSpaceStore {
    space: KnowledgeSpace,
}

impl FixedSpaceStore {
    fn new(space: KnowledgeSpace) -> Self {
        Self { space }
    }
}

#[async_trait]
impl KnowledgeSpaceStore for FixedSpaceStore {
    async fn create_space(
        &self,
        _record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn get_space(&self, space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        if space_id == self.space.id {
            Ok(self.space.clone())
        } else {
            Err(KnowledgeSpaceStoreError::Internal(
                "missing space".to_string(),
            ))
        }
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

    async fn update_space(
        &self,
        _space_id: u64,
        _record: UpdateKnowledgeSpaceRecord,
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
}

#[derive(Default)]
struct MemorySourceStore;

#[async_trait]
impl KnowledgeSourceStore for MemorySourceStore {
    async fn create_source(
        &self,
        _record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        Err(KnowledgeSourceStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn list_sources_for_space(
        &self,
        _space_id: u64,
    ) -> Result<Vec<KnowledgeSource>, KnowledgeSourceStoreError> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct MemoryOkfConceptStore {
    logs: Mutex<Vec<OkfLogEntry>>,
}

impl MemoryOkfConceptStore {
    fn log_count(&self) -> usize {
        self.logs.lock().unwrap().len()
    }
}

#[async_trait]
impl KnowledgeOkfConceptStore for MemoryOkfConceptStore {
    async fn upsert_concept(
        &self,
        _record: UpsertKnowledgeOkfConceptRecord,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConcept,
        KnowledgeOkfConceptStoreError,
    > {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }

    async fn create_revision(
        &self,
        _record: CreateKnowledgeOkfConceptRevisionRecord,
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConceptRevision,
        KnowledgeOkfConceptStoreError,
    > {
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
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConcept,
        KnowledgeOkfConceptStoreError,
    > {
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
    ) -> Result<
        (
            Vec<sdkwork_knowledgebase_contract::okf::KnowledgeOkfConceptRevision>,
            Option<u64>,
            bool,
        ),
        KnowledgeOkfConceptStoreError,
    > {
        validated_okf_test_page_size(page_size)?;
        Ok((Vec::new(), None, false))
    }

    async fn append_log_entry(
        &self,
        record: AppendKnowledgeOkfLogEntryRecord,
    ) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError> {
        let entry = OkfLogEntry {
            occurred_at: record.event_time,
            event_type: sdkwork_knowledgebase_contract::okf::OkfLogEventType::Compile,
            title: record.title,
            actor: record.actor,
            affected_concepts: record.affected_concepts,
            audit_event_id: record.audit_event_id,
            warnings: record.warnings,
        };
        self.logs.lock().unwrap().push(entry.clone());
        Ok(entry)
    }

    async fn list_log_entries(
        &self,
        _space_id: u64,
    ) -> Result<Vec<OkfLogEntry>, KnowledgeOkfConceptStoreError> {
        Ok(self.logs.lock().unwrap().clone())
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
    ) -> Result<
        sdkwork_knowledgebase_contract::okf::KnowledgeOkfConcept,
        KnowledgeOkfConceptStoreError,
    > {
        Err(KnowledgeOkfConceptStoreError::Internal(
            "unsupported in test fake".to_string(),
        ))
    }
}

#[derive(Default)]
struct MemoryDrive {
    objects: Mutex<HashMap<String, KnowledgeObjectRef>>,
    put_counts: Mutex<HashMap<String, u32>>,
}

impl MemoryDrive {
    fn has_path(&self, logical_path: &str) -> bool {
        self.objects.lock().unwrap().contains_key(logical_path)
    }

    fn put_count(&self, logical_path: &str) -> u32 {
        self.put_counts
            .lock()
            .unwrap()
            .get(logical_path)
            .copied()
            .unwrap_or(0)
    }
}

#[async_trait]
impl KnowledgeDriveStorage for MemoryDrive {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        let digest = Sha256::digest(&request.body);
        let checksum = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let object_ref = KnowledgeObjectRef {
            storage_provider_id: "provider-kb".to_string(),
            bucket: "test".to_string(),
            object_key: request.logical_path.clone(),
            logical_path: request.logical_path.clone(),
            object_role: request.object_role,
            content_type: request.content_type,
            size_bytes: request.body.len() as u64,
            checksum_sha256_hex: Some(checksum),
            etag: None,
            version_id: None,
        };
        self.objects
            .lock()
            .unwrap()
            .insert(request.logical_path.clone(), object_ref.clone());
        *self
            .put_counts
            .lock()
            .unwrap()
            .entry(request.logical_path)
            .or_insert(0) += 1;
        Ok(object_ref)
    }

    async fn head_object(
        &self,
        request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        let logical_path = request
            .logical_path
            .as_deref()
            .ok_or_else(|| KnowledgeStorageError::internal("missing logical_path"))?;
        self.objects
            .lock()
            .unwrap()
            .get(logical_path)
            .cloned()
            .ok_or_else(|| KnowledgeStorageError::internal("missing object"))
    }

    async fn get_object_text(
        &self,
        _object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("unsupported in test fake"))
    }
}

#[derive(Default)]
struct MemoryOkfBundleFileStore {
    entries: Mutex<Vec<CreateKnowledgeOkfBundleFileRecord>>,
}

impl MemoryOkfBundleFileStore {
    fn paths(&self) -> Vec<String> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .map(|entry| entry.logical_path.clone())
            .collect()
    }
}

#[async_trait]
impl KnowledgeOkfBundleFileStore for MemoryOkfBundleFileStore {
    async fn create_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        self.upsert_file_entry(record).await
    }

    async fn upsert_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        self.entries.lock().unwrap().push(record.clone());
        Ok(KnowledgeOkfBundleFile {
            id: self.entries.lock().unwrap().len() as u64,
            space_id: record.space_id,
            logical_path: record.logical_path,
            file_kind: record.file_kind,
            artifact_role: record.artifact_role,
            drive_bucket: record.drive_bucket,
            drive_object_key: record.drive_object_key,
            checksum_sha256_hex: record.checksum_sha256_hex,
            staged_import_root: None,
            import_id: None,
        })
    }
}

#[derive(Default)]
struct MemoryDriveWorkspace {
    paths: Mutex<Vec<String>>,
}

impl MemoryDriveWorkspace {
    fn paths(&self) -> Vec<String> {
        self.paths.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeDriveWorkspace for MemoryDriveWorkspace {
    async fn ensure_nodes(
        &self,
        request: EnsureKnowledgeDriveNodesRequest,
    ) -> Result<(), KnowledgeDriveWorkspaceError> {
        for node in request.nodes {
            self.paths.lock().unwrap().push(node.logical_path);
        }
        Ok(())
    }
}

struct RecordingWorkflowEngine<'a> {
    rebuild_calls: Mutex<Vec<u64>>,
    drive: &'a MemoryDrive,
    concepts: &'a MemoryOkfConceptStore,
    spaces: &'a FixedSpaceStore,
}

impl<'a> RecordingWorkflowEngine<'a> {
    fn new(
        drive: &'a MemoryDrive,
        concepts: &'a MemoryOkfConceptStore,
        spaces: &'a FixedSpaceStore,
    ) -> Self {
        Self {
            rebuild_calls: Mutex::new(Vec::new()),
            drive,
            concepts,
            spaces,
        }
    }

    fn rebuild_calls(&self) -> Vec<u64> {
        self.rebuild_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl OkfBundleWorkflowEngine for RecordingWorkflowEngine<'_> {
    async fn rebuild_index(&self, space_id: u64) -> Result<(), KnowledgeEngineError> {
        self.rebuild_calls.lock().unwrap().push(space_id);
        rebuild_bundle_index_for_space(self.drive, self.concepts, self.spaces, space_id)
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))
    }

    async fn lint_bundle_report(
        &self,
        _space_id: u64,
    ) -> Result<OkfBundleLintResult, KnowledgeEngineError> {
        Ok(OkfBundleLintResult {
            conformance: "pass".to_string(),
            issues: Vec::new(),
        })
    }
}
