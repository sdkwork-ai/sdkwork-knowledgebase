use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::okf::{
    OkfBundleFileRegistryService, OkfBundleInitializerService,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_space::{
    CreateKnowledgeDriveSpaceRequest, KnowledgeDriveSpaceBinding, KnowledgeDriveSpaceProvisioner,
    KnowledgeDriveSpaceProvisionerError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_workspace::{
    EnsureKnowledgeDriveNodeKind, EnsureKnowledgeDriveNodesRequest, KnowledgeDriveWorkspace,
    KnowledgeDriveWorkspaceError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_bundle_file_store::{
    CreateKnowledgeOkfBundleFileRecord, KnowledgeOkfBundleFileStore,
    KnowledgeOkfBundleFileStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore, KnowledgeSpaceStoreError,
    UpdateKnowledgeSpaceRecord,
};
use sdkwork_intelligence_knowledgebase_service::space::KnowledgeSpaceService;
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::{
    CreateKnowledgeSpaceRequest, KnowledgeSpace, KnowledgeSpaceStatus,
};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn creating_space_initializes_okf_bundle_standard_files_through_drive() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive).with_registry(&registry);
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer);

    let created = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: Some("OKF bundle research".to_string()),
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap();

    assert_eq!(created.name, "Research Space");
    assert_eq!(created.status, KnowledgeSpaceStatus::Active);
    assert!(created.okf_bundle_initialized);
    assert_eq!(
        drive.paths(),
        vec![
            "okf/schema/AGENTS.md",
            "okf/schema/okf_profile.yaml",
            "okf/index.md",
            "okf/log.md"
        ]
    );
    assert_eq!(file_entries.logical_paths(), drive.paths());
    assert_eq!(file_entries.space_ids(), vec![1, 1, 1, 1]);
}

#[tokio::test]
async fn creating_space_binds_dedicated_drive_knowledge_space_before_okf_initialization() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive).with_registry(&registry);
    let drive_spaces = RecordingDriveSpaceProvisioner::new("drv-kb-001");
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer)
        .with_drive_context("tenant-9001", "user-123")
        .with_wiki_context(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                tenant_id: 9001,
                organization_id: 0,
            },
            123,
        )
        .with_drive_space_provisioner(&drive_spaces);

    let created = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap();

    assert_eq!(created.drive_space_id.as_deref(), Some("drv-kb-001"));
    assert_eq!(drive_spaces.calls(), 1);
    assert_eq!(
        drive_spaces.requested_tenant_id(),
        Some("tenant-9001".to_string())
    );
    assert_eq!(
        drive_spaces.requested_operator_id(),
        Some("user-123".to_string())
    );
    assert_eq!(
        drive_spaces.requested_knowledge_space_uuid(),
        Some("space-1".to_string())
    );
    assert_eq!(
        drive_spaces.requested_owner_subject_type(),
        Some("app".to_string())
    );
    assert_eq!(
        drive_spaces.requested_owner_subject_id(),
        Some("sdkwork-knowledgebase:space-1".to_string())
    );
    assert!(created.okf_bundle_initialized);
    assert_eq!(drive.paths().len(), 4);
}

#[tokio::test]
async fn drive_space_provisioning_requires_context_before_creating_space_record() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive).with_registry(&registry);
    let drive_spaces = RecordingDriveSpaceProvisioner::new("drv-kb-001");
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer)
        .with_drive_space_provisioner(&drive_spaces);

    let error = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("drive tenant_id and operator_id are required"));
    assert_eq!(store.space_count(), 0);
    assert_eq!(drive_spaces.calls(), 0);
    assert!(drive.paths().is_empty());
}

#[tokio::test]
async fn drive_space_provisioning_failure_does_not_leave_half_initialized_space() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive).with_registry(&registry);
    let drive_spaces = FailingDriveSpaceProvisioner;
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer)
        .with_drive_context("tenant-9001", "user-123")
        .with_wiki_context(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                tenant_id: 9001,
                organization_id: 0,
            },
            123,
        )
        .with_drive_space_provisioner(&drive_spaces);

    let error = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("drive space create failed intentionally"));
    assert_eq!(store.space_count(), 0);
    assert!(drive.paths().is_empty());
    assert!(file_entries.logical_paths().is_empty());
}

#[tokio::test]
async fn okf_bundle_initialization_failure_releases_created_drive_space_and_local_space() {
    let store = MemorySpaceStore::default();
    let drive = FailingDrive;
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive).with_registry(&registry);
    let drive_spaces = RecordingDriveSpaceProvisioner::new("drv-kb-001");
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer)
        .with_drive_context("tenant-9001", "user-123")
        .with_wiki_context(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                tenant_id: 9001,
                organization_id: 0,
            },
            123,
        )
        .with_drive_space_provisioner(&drive_spaces);

    let error = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("standard okf bundle file write failed intentionally"));
    assert_eq!(store.space_count(), 0);
    assert_eq!(drive_spaces.delete_calls(), 1);
    assert_eq!(
        drive_spaces.deleted_drive_space_id(),
        Some("drv-kb-001".to_string())
    );
    assert!(file_entries.logical_paths().is_empty());
}

#[tokio::test]
async fn creating_bound_space_initializes_browser_visible_okf_drive_nodes() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let drive_workspace = RecordingDriveWorkspace::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive)
        .with_registry(&registry)
        .with_drive_workspace(&drive_workspace);
    let drive_spaces = RecordingDriveSpaceProvisioner::new("drv-kb-001");
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer)
        .with_drive_context("tenant-9001", "user-123")
        .with_wiki_context(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                tenant_id: 9001,
                organization_id: 0,
            },
            123,
        )
        .with_drive_space_provisioner(&drive_spaces);

    service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap();

    assert_eq!(drive_workspace.request_count(), 1);
    let request = drive_workspace.request().unwrap();
    assert_eq!(request.drive_space_id, "drv-kb-001");
    let paths: Vec<String> = request
        .nodes
        .iter()
        .map(|node| node.logical_path.clone())
        .collect();
    for expected in [
        "okf",
        "okf/schema",
        ".sdkwork/governance/revisions",
        "okf/schema/AGENTS.md",
        "okf/schema/okf_profile.yaml",
        "okf/index.md",
        "okf/log.md",
    ] {
        assert!(paths.contains(&expected.to_string()), "missing {expected}");
    }
}

#[tokio::test]
async fn creating_external_space_ensures_drive_permission_anchor_without_okf_bundle() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let drive_workspace = RecordingDriveWorkspace::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive)
        .with_registry(&registry)
        .with_drive_workspace(&drive_workspace);
    let drive_spaces = RecordingDriveSpaceProvisioner::new("drv-kb-001");
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer)
        .with_drive_context("tenant-9001", "user-123")
        .with_wiki_context(
            sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                tenant_id: 9001,
                organization_id: 0,
            },
            123,
        )
        .with_drive_space_provisioner(&drive_spaces);

    let created = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "External Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::External,
        })
        .await
        .unwrap();

    assert_eq!(
        created.knowledge_mode,
        KnowledgeAgentKnowledgeMode::External
    );
    assert!(!created.okf_bundle_initialized);
    assert_eq!(created.drive_space_id.as_deref(), Some("drv-kb-001"));
    assert!(drive.paths().is_empty());
    assert_eq!(drive_workspace.request_count(), 1);
    let request = drive_workspace.request().unwrap();
    assert_eq!(request.drive_space_id, "drv-kb-001");
    assert_eq!(request.nodes.len(), 1);
    assert_eq!(request.nodes[0].logical_path, "workspace");
    assert_eq!(request.nodes[0].kind, EnsureKnowledgeDriveNodeKind::Folder);
}

#[tokio::test]
async fn workspace_backed_initialization_requires_drive_space_binding() {
    let store = MemorySpaceStore::default();
    let drive = RecordingDrive::default();
    let drive_workspace = RecordingDriveWorkspace::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive)
        .with_registry(&registry)
        .with_drive_workspace(&drive_workspace);
    let service = KnowledgeSpaceService::new(&store, &okf_bundle_initializer);

    let error = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: Some("user".to_string()),
            owner_subject_id: Some("test-owner".to_string()),
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .unwrap_err();

    assert!(error.to_string().contains("drive_space_id is required"));
    assert_eq!(store.space_count(), 0);
    assert!(drive.paths().is_empty());
    assert!(file_entries.logical_paths().is_empty());
    assert!(drive_workspace.request().is_none());
}

#[tokio::test]
async fn initializer_requires_drive_space_before_persisting_standard_files_when_workspace_enabled()
{
    let drive = RecordingDrive::default();
    let drive_workspace = RecordingDriveWorkspace::default();
    let file_entries = MemoryOkfBundleFileEntryStore::default();
    let registry = OkfBundleFileRegistryService::new(&file_entries);
    let okf_bundle_initializer = OkfBundleInitializerService::new(&drive)
        .with_registry(&registry)
        .with_drive_workspace(&drive_workspace);

    let error = okf_bundle_initializer
        .initialize_standard_files(1, "Research Space", None)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("drive_space_id is required"));
    assert!(drive.paths().is_empty());
    assert!(file_entries.logical_paths().is_empty());
    assert!(drive_workspace.request().is_none());
}

#[derive(Default)]
struct MemorySpaceStore {
    next_id: Mutex<u64>,
    spaces: Mutex<Vec<KnowledgeSpace>>,
}

impl MemorySpaceStore {
    fn space_count(&self) -> usize {
        self.spaces
            .lock()
            .unwrap()
            .iter()
            .filter(|space| space.status == KnowledgeSpaceStatus::Active)
            .count()
    }
}

#[async_trait]
impl KnowledgeSpaceStore for MemorySpaceStore {
    async fn create_space(
        &self,
        record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut next_id = self.next_id.lock().unwrap();
        *next_id += 1;
        let space = KnowledgeSpace {
            id: *next_id,
            uuid: format!("space-{}", *next_id),
            created_by: record.created_by,
            name: record.name,
            description: record.description,
            drive_space_id: None,
            status: KnowledgeSpaceStatus::Active,
            okf_bundle_initialized: false,
            knowledge_mode: record.knowledge_mode,
        };
        self.spaces.lock().unwrap().push(space.clone());
        Ok(space)
    }

    async fn get_space(&self, space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        self.spaces
            .lock()
            .unwrap()
            .iter()
            .find(|space| space.id == space_id)
            .cloned()
            .ok_or_else(|| KnowledgeSpaceStoreError::Internal("missing space".to_string()))
    }

    async fn mark_drive_space_bound(
        &self,
        space_id: u64,
        record: sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut spaces = self.spaces.lock().unwrap();
        let space = spaces
            .iter_mut()
            .find(|space| space.id == space_id)
            .ok_or_else(|| KnowledgeSpaceStoreError::Internal("missing space".to_string()))?;
        space.drive_space_id = Some(record.drive_space_id);
        Ok(space.clone())
    }

    async fn mark_okf_bundle_initialized(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut spaces = self.spaces.lock().unwrap();
        let space = spaces
            .iter_mut()
            .find(|space| space.id == space_id)
            .ok_or_else(|| KnowledgeSpaceStoreError::Internal("missing space".to_string()))?;
        space.okf_bundle_initialized = true;
        Ok(space.clone())
    }

    async fn update_space(
        &self,
        space_id: u64,
        record: UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut spaces = self.spaces.lock().unwrap();
        let space = spaces
            .iter_mut()
            .find(|space| space.id == space_id)
            .ok_or_else(|| KnowledgeSpaceStoreError::Internal("missing space".to_string()))?;
        if let Some(name) = record.name {
            space.name = name;
        }
        if let Some(description) = record.description {
            space.description = Some(description);
        }
        Ok(space.clone())
    }

    async fn mark_space_deleted(&self, space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        let mut spaces = self.spaces.lock().unwrap();
        let space = spaces
            .iter_mut()
            .find(|space| space.id == space_id)
            .ok_or_else(|| KnowledgeSpaceStoreError::Internal("missing space".to_string()))?;
        space.status = KnowledgeSpaceStatus::Deleted;
        Ok(())
    }
}

struct RecordingDriveSpaceProvisioner {
    drive_space_id: String,
    calls: Mutex<u32>,
    delete_calls: Mutex<u32>,
    requested_tenant_id: Mutex<Option<String>>,
    requested_operator_id: Mutex<Option<String>>,
    requested_knowledge_space_uuid: Mutex<Option<String>>,
    requested_owner_subject_type: Mutex<Option<String>>,
    requested_owner_subject_id: Mutex<Option<String>>,
    deleted_drive_space_id: Mutex<Option<String>>,
}

impl RecordingDriveSpaceProvisioner {
    fn new(drive_space_id: &str) -> Self {
        Self {
            drive_space_id: drive_space_id.to_string(),
            calls: Mutex::new(0),
            delete_calls: Mutex::new(0),
            requested_tenant_id: Mutex::new(None),
            requested_operator_id: Mutex::new(None),
            requested_knowledge_space_uuid: Mutex::new(None),
            requested_owner_subject_type: Mutex::new(None),
            requested_owner_subject_id: Mutex::new(None),
            deleted_drive_space_id: Mutex::new(None),
        }
    }

    fn calls(&self) -> u32 {
        *self.calls.lock().unwrap()
    }

    fn requested_tenant_id(&self) -> Option<String> {
        self.requested_tenant_id.lock().unwrap().clone()
    }

    fn requested_operator_id(&self) -> Option<String> {
        self.requested_operator_id.lock().unwrap().clone()
    }

    fn requested_knowledge_space_uuid(&self) -> Option<String> {
        self.requested_knowledge_space_uuid.lock().unwrap().clone()
    }

    fn requested_owner_subject_type(&self) -> Option<String> {
        self.requested_owner_subject_type.lock().unwrap().clone()
    }

    fn requested_owner_subject_id(&self) -> Option<String> {
        self.requested_owner_subject_id.lock().unwrap().clone()
    }

    fn delete_calls(&self) -> u32 {
        *self.delete_calls.lock().unwrap()
    }

    fn deleted_drive_space_id(&self) -> Option<String> {
        self.deleted_drive_space_id.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeDriveSpaceProvisioner for RecordingDriveSpaceProvisioner {
    async fn create_knowledge_drive_space(
        &self,
        request: CreateKnowledgeDriveSpaceRequest,
    ) -> Result<KnowledgeDriveSpaceBinding, KnowledgeDriveSpaceProvisionerError> {
        *self.calls.lock().unwrap() += 1;
        *self.requested_tenant_id.lock().unwrap() = Some(request.tenant_id);
        *self.requested_operator_id.lock().unwrap() = Some(request.operator_id);
        *self.requested_knowledge_space_uuid.lock().unwrap() = Some(request.knowledge_space_uuid);
        *self.requested_owner_subject_type.lock().unwrap() = Some(request.owner_subject_type);
        *self.requested_owner_subject_id.lock().unwrap() = Some(request.owner_subject_id);
        Ok(KnowledgeDriveSpaceBinding {
            drive_space_id: self.drive_space_id.clone(),
        })
    }

    async fn delete_knowledge_drive_space(
        &self,
        request: sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_space::DeleteKnowledgeDriveSpaceRequest,
    ) -> Result<(), KnowledgeDriveSpaceProvisionerError> {
        *self.delete_calls.lock().unwrap() += 1;
        *self.deleted_drive_space_id.lock().unwrap() = Some(request.drive_space_id);
        Ok(())
    }
}

struct FailingDriveSpaceProvisioner;

#[async_trait]
impl KnowledgeDriveSpaceProvisioner for FailingDriveSpaceProvisioner {
    async fn create_knowledge_drive_space(
        &self,
        _request: CreateKnowledgeDriveSpaceRequest,
    ) -> Result<KnowledgeDriveSpaceBinding, KnowledgeDriveSpaceProvisionerError> {
        Err(KnowledgeDriveSpaceProvisionerError::Upstream(
            "drive space create failed intentionally".to_string(),
        ))
    }

    async fn delete_knowledge_drive_space(
        &self,
        _request: sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_space::DeleteKnowledgeDriveSpaceRequest,
    ) -> Result<(), KnowledgeDriveSpaceProvisionerError> {
        Ok(())
    }
}

#[derive(Default)]
struct RecordingDrive {
    paths: Arc<Mutex<Vec<String>>>,
}

#[derive(Default)]
struct RecordingDriveWorkspace {
    requests: Mutex<Vec<EnsureKnowledgeDriveNodesRequest>>,
}

impl RecordingDriveWorkspace {
    fn request(&self) -> Option<EnsureKnowledgeDriveNodesRequest> {
        let requests = self.requests.lock().unwrap();
        requests.first().cloned()
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

#[async_trait]
impl KnowledgeDriveWorkspace for RecordingDriveWorkspace {
    async fn ensure_nodes(
        &self,
        request: EnsureKnowledgeDriveNodesRequest,
    ) -> Result<(), KnowledgeDriveWorkspaceError> {
        self.requests.lock().unwrap().push(request);
        Ok(())
    }
}

#[derive(Default)]
struct MemoryOkfBundleFileEntryStore {
    logical_paths: Mutex<Vec<String>>,
    space_ids: Mutex<Vec<u64>>,
}

impl MemoryOkfBundleFileEntryStore {
    fn logical_paths(&self) -> Vec<String> {
        self.logical_paths.lock().unwrap().clone()
    }

    fn space_ids(&self) -> Vec<u64> {
        self.space_ids.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeOkfBundleFileStore for MemoryOkfBundleFileEntryStore {
    async fn create_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<
        sdkwork_knowledgebase_contract::KnowledgeOkfBundleFile,
        KnowledgeOkfBundleFileStoreError,
    > {
        self.logical_paths
            .lock()
            .unwrap()
            .push(record.logical_path.clone());
        self.space_ids.lock().unwrap().push(record.space_id);

        Ok(sdkwork_knowledgebase_contract::KnowledgeOkfBundleFile {
            id: self.logical_paths.lock().unwrap().len() as u64,
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

impl RecordingDrive {
    fn paths(&self) -> Vec<String> {
        self.paths.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeDriveStorage for RecordingDrive {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        self.paths
            .lock()
            .unwrap()
            .push(request.logical_path.clone());
        Ok(KnowledgeObjectRef {
            storage_provider_id: "provider-kb".to_string(),
            bucket: "test".to_string(),
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
        _request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("not needed"))
    }

    async fn get_object_text(
        &self,
        _object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("not needed"))
    }
}

struct FailingDrive;

#[async_trait]
impl KnowledgeDriveStorage for FailingDrive {
    async fn put_object(
        &self,
        _request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal(
            "standard okf bundle file write failed intentionally",
        ))
    }

    async fn head_object(
        &self,
        _request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("not needed"))
    }

    async fn get_object_text(
        &self,
        _object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        Err(KnowledgeStorageError::internal("not needed"))
    }
}
