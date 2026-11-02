//! Object-level access rules for knowledge spaces.
//!
//! The Drive node ACL is a projection written separately from the space record. These tests
//! pin the invariant that the recorded creator always keeps access to the space it created
//! while every other actor still needs an explicit Drive grant.

use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::okf::OkfBundleInitializerService;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_access_control::{
    GrantKnowledgeSpaceAccessRequest, KnowledgeAccessCheckRequest, KnowledgeAccessControl,
    KnowledgeAccessControlError, KnowledgeAccessGrant, KnowledgeAccessRole,
    KnowledgeNodeAccessCheckRequest, KnowledgeSpaceMember, KnowledgeSpaceMemberList,
    KnowledgeSubjectType, ListKnowledgeSpaceMembersRequest, RevokeKnowledgeSpaceAccessRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_storage::{
    HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeObjectRef, KnowledgeStorageError,
    PutKnowledgeObjectRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    BindKnowledgeDriveSpaceRecord, CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
    KnowledgeSpaceStoreError, UpdateKnowledgeSpaceRecord,
};
use sdkwork_intelligence_knowledgebase_service::space::{
    KnowledgeSpaceService, KnowledgeSpaceServiceError,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::{
    CreateKnowledgeSpaceRequest, KnowledgeSpace, KnowledgeSpaceStatus,
};
use std::sync::Mutex;

const TENANT: u64 = 10_001;
const CREATOR: u64 = 4242;
const OTHER_ACTOR: u64 = 5150;

#[tokio::test]
async fn creator_keeps_access_and_repairs_the_missing_drive_acl_projection() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let access = RecordingAccessControl::denied();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    let space = service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &CREATOR.to_string())
        .await
        .expect("the recorded creator must always reach its own space");

    assert_eq!(space.id, space_id());
    let repairs = access.granted_requests();
    assert_eq!(
        repairs.len(),
        1,
        "the missing Drive grant must be repaired on the creator's first authorized access"
    );
    let repair = &repairs[0];
    assert_eq!(repair.subject_type, KnowledgeSubjectType::User);
    assert_eq!(repair.subject_id, CREATOR.to_string());
    assert_eq!(repair.role, KnowledgeAccessRole::Owner);
    assert_eq!(repair.drive_space_id, "drv-kb-001");
}

#[tokio::test]
async fn a_non_creator_without_a_drive_grant_stays_denied() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let access = RecordingAccessControl::denied();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    let error = service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &OTHER_ACTOR.to_string())
        .await
        .expect_err("a space that is neither created by nor granted to the actor stays closed");

    assert!(matches!(error, KnowledgeSpaceServiceError::AccessDenied(_)));
    assert!(
        access.granted_requests().is_empty(),
        "a denied actor must never trigger an owner grant"
    );
}

#[tokio::test]
async fn a_granted_reader_reaches_reads_but_not_owner_operations() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let access = RecordingAccessControl::allowed(KnowledgeAccessRole::Reader);
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &OTHER_ACTOR.to_string())
        .await
        .expect("a granted reader must keep reading");

    let error = service
        .update_space(
            space_id(),
            &TENANT.to_string(),
            &OTHER_ACTOR.to_string(),
            sdkwork_knowledgebase_contract::UpdateKnowledgeSpaceRequest {
                name: Some("Renamed".to_string()),
                description: None,
            },
        )
        .await
        .expect_err("a reader must not rename a space");
    assert!(matches!(error, KnowledgeSpaceServiceError::AccessDenied(_)));
}

#[tokio::test]
async fn legacy_space_without_a_recorded_creator_keeps_the_drive_decision() {
    // `created_by == 0` marks a row written before ownership was recorded: there is no
    // creator to admit, so the Drive ACL decision stands alone.
    let store = MemorySpaceStore::with_space(space(0, Some("drv-kb-001")));
    let access = RecordingAccessControl::denied();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    let error = service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &CREATOR.to_string())
        .await
        .expect_err("a space with no recorded creator cannot be claimed by any actor");

    assert!(matches!(error, KnowledgeSpaceServiceError::AccessDenied(_)));
    assert!(access.granted_requests().is_empty());
}

#[tokio::test]
async fn creator_keeps_access_even_when_the_space_has_no_drive_binding() {
    let store = MemorySpaceStore::with_space(space(CREATOR, None));
    let access = RecordingAccessControl::denied();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &CREATOR.to_string())
        .await
        .expect("the creator must still be able to rename or delete a space whose binding failed");

    let error = service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &OTHER_ACTOR.to_string())
        .await
        .expect_err("an unbound space is not open to other actors");
    assert!(matches!(error, KnowledgeSpaceServiceError::AccessDenied(_)));
}

#[tokio::test]
async fn creator_access_survives_a_failed_acl_repair() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let access = RecordingAccessControl::denied_repair_failing();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &CREATOR.to_string())
        .await
        .expect("a failed projection repair must not lock the creator out of its own space");
    assert_eq!(access.granted_requests().len(), 1);
}

#[tokio::test]
async fn creator_can_manage_members_without_a_drive_grant() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let access = RecordingAccessControl::denied();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer).with_access_control(&access);

    service
        .grant_space_member(
            space_id(),
            &TENANT.to_string(),
            KnowledgeSubjectType::User,
            "777",
            KnowledgeAccessRole::Reader,
            &CREATOR.to_string(),
        )
        .await
        .expect("the creator must be able to share its own space");
    assert_eq!(
        access.granted_requests().len(),
        2,
        "one owner-grant repair plus the requested reader grant"
    );
}

#[tokio::test]
async fn missing_access_control_stays_fail_closed_for_the_creator_too() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer);

    let error = service
        .get_space_with_access_check(space_id(), &TENANT.to_string(), &CREATOR.to_string())
        .await
        .expect_err("without an injected access control the read path must stay closed");

    assert!(matches!(
        error,
        KnowledgeSpaceServiceError::InvalidRequest(_)
    ));
}

#[tokio::test]
async fn creating_a_space_records_the_authenticated_creator() {
    let store = MemorySpaceStore::with_space(space(CREATOR, Some("drv-kb-001")));
    let access = RecordingAccessControl::denied();
    let drive = StubDrive;
    let initializer = OkfBundleInitializerService::new(&drive);
    let service = KnowledgeSpaceService::new(&store, &initializer)
        .with_access_control(&access)
        .with_creator(CREATOR);

    let created = service
        .create_space(CreateKnowledgeSpaceRequest {
            name: "Research Space".to_string(),
            description: None,
            owner_subject_type: None,
            owner_subject_id: None,
            knowledge_mode: KnowledgeAgentKnowledgeMode::Rag,
        })
        .await
        .expect("create a personal knowledge space");

    assert_eq!(created.created_by, CREATOR);
    assert_eq!(store.last_created_by(), Some(CREATOR));
}

fn space_id() -> u64 {
    1
}

fn space(created_by: u64, drive_space_id: Option<&str>) -> KnowledgeSpace {
    KnowledgeSpace {
        id: space_id(),
        uuid: "kb-001".to_string(),
        created_by,
        name: "Research Space".to_string(),
        description: None,
        drive_space_id: drive_space_id.map(str::to_string),
        status: KnowledgeSpaceStatus::Active,
        okf_bundle_initialized: false,
        knowledge_mode: KnowledgeAgentKnowledgeMode::OkfBundle,
    }
}

struct StubDrive;

#[async_trait]
impl KnowledgeDriveStorage for StubDrive {
    async fn put_object(
        &self,
        _request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::Internal(
            "stub drive does not store objects".to_string(),
        ))
    }

    async fn head_object(
        &self,
        _request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError> {
        Err(KnowledgeStorageError::Internal(
            "stub drive does not store objects".to_string(),
        ))
    }

    async fn get_object_text(
        &self,
        _object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError> {
        Err(KnowledgeStorageError::Internal(
            "stub drive does not store objects".to_string(),
        ))
    }
}

struct MemorySpaceStore {
    space: Mutex<KnowledgeSpace>,
    last_created_by: Mutex<Option<u64>>,
    next_id: Mutex<u64>,
}

impl MemorySpaceStore {
    fn with_space(space: KnowledgeSpace) -> Self {
        Self {
            space: Mutex::new(space),
            last_created_by: Mutex::new(None),
            next_id: Mutex::new(1),
        }
    }

    fn last_created_by(&self) -> Option<u64> {
        *self.last_created_by.lock().unwrap()
    }
}

#[async_trait]
impl KnowledgeSpaceStore for MemorySpaceStore {
    async fn create_space(
        &self,
        record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        *self.last_created_by.lock().unwrap() = Some(record.created_by);
        let mut next_id = self.next_id.lock().unwrap();
        let mut space = self.space.lock().unwrap();
        space.id = *next_id;
        *next_id += 1;
        space.created_by = record.created_by;
        space.name = record.name;
        space.description = record.description;
        space.knowledge_mode = record.knowledge_mode;
        Ok(space.clone())
    }

    async fn get_space(&self, _space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        // Later accesses must reuse the create-time id so assertions stay about access, not ids.
        Ok(self.space.lock().unwrap().clone())
    }

    async fn mark_drive_space_bound(
        &self,
        _space_id: u64,
        record: BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        space.drive_space_id = Some(record.drive_space_id);
        Ok(space.clone())
    }

    async fn mark_okf_bundle_initialized(
        &self,
        _space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        space.okf_bundle_initialized = true;
        Ok(space.clone())
    }

    async fn update_space(
        &self,
        _space_id: u64,
        record: UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        if let Some(name) = record.name {
            space.name = name;
        }
        space.description = record.description.or_else(|| space.description.clone());
        Ok(space.clone())
    }

    async fn mark_space_deleted(&self, _space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        let mut space = self.space.lock().unwrap();
        space.status = KnowledgeSpaceStatus::Deleted;
        Ok(())
    }
}

/// Records every permission interaction so a test can assert both the verdict and the
/// repair that the service attempted.
struct RecordingAccessControl {
    drive_role: Option<KnowledgeAccessRole>,
    repair_fails: bool,
    granted: Mutex<Vec<GrantKnowledgeSpaceAccessRequest>>,
}

impl RecordingAccessControl {
    fn denied() -> Self {
        Self {
            drive_role: None,
            repair_fails: false,
            granted: Mutex::new(Vec::new()),
        }
    }

    fn denied_repair_failing() -> Self {
        Self {
            drive_role: None,
            repair_fails: true,
            granted: Mutex::new(Vec::new()),
        }
    }

    fn allowed(role: KnowledgeAccessRole) -> Self {
        Self {
            drive_role: Some(role),
            repair_fails: false,
            granted: Mutex::new(Vec::new()),
        }
    }

    fn granted_requests(&self) -> Vec<GrantKnowledgeSpaceAccessRequest> {
        self.granted.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeAccessControl for RecordingAccessControl {
    async fn check_space_access(
        &self,
        request: KnowledgeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
        Ok(KnowledgeAccessGrant {
            allowed: self
                .drive_role
                .is_some_and(|role| role.satisfies(&request.required_role)),
            effective_role: self.drive_role,
        })
    }

    async fn check_node_access(
        &self,
        _request: KnowledgeNodeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
        Ok(KnowledgeAccessGrant {
            allowed: self.drive_role.is_some(),
            effective_role: None,
        })
    }

    async fn grant_space_access(
        &self,
        request: GrantKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError> {
        self.granted.lock().unwrap().push(request);
        if self.repair_fails {
            return Err(KnowledgeAccessControlError::Upstream(
                "drive permission store unavailable".to_string(),
            ));
        }
        Ok(())
    }

    async fn revoke_space_access(
        &self,
        _request: RevokeKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError> {
        Ok(())
    }

    async fn list_space_members(
        &self,
        _request: ListKnowledgeSpaceMembersRequest,
    ) -> Result<KnowledgeSpaceMemberList, KnowledgeAccessControlError> {
        Ok(KnowledgeSpaceMemberList {
            members: Vec::<KnowledgeSpaceMember>::new(),
            next_cursor: None,
        })
    }
}
