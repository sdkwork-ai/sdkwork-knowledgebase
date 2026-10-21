use async_trait::async_trait;
use sdkwork_knowledgebase_contract::group_space::{
    GroupKnowledgeSpaceBinding, GroupKnowledgeSpaceMember,
};
use sdkwork_knowledgebase_contract::space::KnowledgeSpace;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceScope {
    pub tenant_id: u64,
    pub organization_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveGroupKnowledgeSpaceRequest {
    pub scope: GroupKnowledgeSpaceScope,
    pub conversation_id: String,
    pub group_name: String,
    pub source_event_id: String,
    pub provisioning_idempotency_key: String,
    pub created_by: String,
    pub membership_epoch: u64,
    pub members: Vec<GroupKnowledgeSpaceMember>,
}

/// Immutable target identity issued by Knowledgebase. IM stores it with its link and includes it
/// on every lifecycle event so an out-of-order event cannot act on a different binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceTarget {
    pub knowledgebase_binding_id: u64,
    pub knowledgebase_binding_uuid: String,
    pub knowledge_space_id: u64,
    pub knowledge_space_uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynchronizeGroupKnowledgeSpaceMembersCommand {
    pub scope: GroupKnowledgeSpaceScope,
    pub conversation_id: String,
    pub group_name: String,
    pub source_event_id: String,
    pub target: GroupKnowledgeSpaceTarget,
    pub membership_epoch: u64,
    /// IM-owned link generation. It is a replay/fingerprint input only and must never be
    /// compared with the mutable Knowledgebase binding version.
    pub upstream_link_generation: u64,
    pub members: Vec<GroupKnowledgeSpaceMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveGroupKnowledgeSpaceCommand {
    pub scope: GroupKnowledgeSpaceScope,
    pub conversation_id: String,
    pub source_event_id: String,
    pub target: GroupKnowledgeSpaceTarget,
    pub membership_epoch: u64,
    /// IM-owned link generation, retained in the durable command fingerprint only.
    pub upstream_link_generation: u64,
    /// Audit attribution from the framework-verified signed caller context. It is not a member
    /// authorization input for the internal IM lifecycle path.
    pub archived_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceReservation {
    pub binding: GroupKnowledgeSpaceBinding,
    pub space: Option<KnowledgeSpace>,
    pub requires_provisioning: bool,
    /// Opaque database lease. It is valid only for the current provisioning attempt and is never
    /// returned from HTTP APIs.
    pub provisioning_lease_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceMembershipChange {
    pub binding: GroupKnowledgeSpaceBinding,
    pub previous_members: Vec<GroupKnowledgeSpaceMember>,
    pub current_members: Vec<GroupKnowledgeSpaceMember>,
    pub requires_acl_projection: bool,
}

/// A durable reservation for an active group's external ACL projection. The opaque lease is
/// intentionally confined to the service/repository boundary and must never be exposed through
/// an HTTP response or persisted outside the binding store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceMembershipSyncReservation {
    pub binding: GroupKnowledgeSpaceBinding,
    pub previous_members: Vec<GroupKnowledgeSpaceMember>,
    pub current_members: Vec<GroupKnowledgeSpaceMember>,
    pub requires_acl_projection: bool,
    pub synchronization_lease_token: Option<String>,
}

/// Durable archive reservation. `archiving` is fail-closed, while a matching lease holder
/// converges Drive ACL cleanup and physical-space archival before finalizing the inbox event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceArchiveReservation {
    pub binding: GroupKnowledgeSpaceBinding,
    pub space: Option<KnowledgeSpace>,
    pub requires_archive: bool,
    pub archive_lease_token: Option<String>,
}

#[async_trait]
pub trait KnowledgeGroupSpaceBindingStore: Send + Sync {
    async fn reserve_group_space(
        &self,
        request: ReserveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceReservation, KnowledgeGroupSpaceBindingStoreError>;

    async fn get_group_space(
        &self,
        scope: GroupKnowledgeSpaceScope,
        conversation_id: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    /// Resolves a group-managed space for one deployment organization. The organization
    /// predicate is mandatory: another organization in the same tenant may also own a binding
    /// for this space id, and that binding must fail closed rather than fall back to generic
    /// Drive ACLs.
    async fn find_group_space_for_space_in_tenant(
        &self,
        tenant_id: u64,
        organization_id: u64,
        space_id: u64,
    ) -> Result<Option<GroupKnowledgeSpaceBinding>, KnowledgeGroupSpaceBindingStoreError>;

    async fn list_active_group_members(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<Vec<GroupKnowledgeSpaceMember>, KnowledgeGroupSpaceBindingStoreError>;

    /// Returns true whenever a membership-to-Drive ACL projection has been reserved or has
    /// failed without a successful retry. Callers must deny group content access in this state.
    async fn has_unsettled_group_membership_projection(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<bool, KnowledgeGroupSpaceBindingStoreError>;

    /// Checks that an external ACL projection lease is still current immediately before/after a
    /// provider grant. Archive start invalidates this fence before cleanup so a delayed worker
    /// cannot re-grant a former member after archive revocation has begun.
    async fn is_group_membership_projection_lease_current(
        &self,
        command: &SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
    ) -> Result<bool, KnowledgeGroupSpaceBindingStoreError>;

    /// Returns whether an external membership ACL mutation may still be in flight for this
    /// binding. Archive finalization must not transition terminal while this durable fence is
    /// held, even though the binding itself has already become fail-closed.
    async fn has_active_group_membership_projection_lease(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<bool, KnowledgeGroupSpaceBindingStoreError>;

    async fn mark_group_space_active(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
        provisioning_lease_token: &str,
        updated_by: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    async fn mark_group_space_failed(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
        provisioning_lease_token: &str,
        error_code: &str,
        updated_by: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    async fn synchronize_group_members(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupSpaceBindingStoreError>;

    /// Validates replay/epoch semantics and reserves the sole active ACL projection for a
    /// binding. The reservation is stored independently from the committed binding so the
    /// current snapshot remains auditable while access checks fail closed before any Drive
    /// mutation is made.
    async fn prepare_group_membership_sync(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
    ) -> Result<GroupKnowledgeSpaceMembershipSyncReservation, KnowledgeGroupSpaceBindingStoreError>;

    /// Atomically commits the member snapshot only after the caller has completed the matching
    /// external Drive ACL projection under the reservation lease.
    async fn complete_group_membership_sync(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupSpaceBindingStoreError>;

    /// Leaves the binding fail-closed when an external ACL projection fails. The same IM event
    /// can later reclaim the expired reservation and converge idempotently.
    async fn fail_group_membership_sync(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
        error_code: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    /// Releases a membership projection only after the service has confirmed that a direct
    /// Drive grant made by a stale projection was successfully compensated during archive.
    async fn settle_group_membership_sync_after_archive(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    async fn mark_acl_projection_active(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<(), KnowledgeGroupSpaceBindingStoreError>;

    async fn mark_acl_projection_failed(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
        error_code: &str,
    ) -> Result<(), KnowledgeGroupSpaceBindingStoreError>;

    /// Starts or resumes the archive saga. It persists a fail-closed archive intent and fences
    /// unsettled membership projections before any external ACL or physical-space mutation.
    async fn begin_group_space_archive(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
    ) -> Result<GroupKnowledgeSpaceArchiveReservation, KnowledgeGroupSpaceBindingStoreError>;

    /// Returns bounded, durable archive work for one tenant/organization. A worker must still
    /// call `begin_group_space_archive` to claim the short lease before touching Drive.
    async fn list_resumable_group_space_archives(
        &self,
        scope: GroupKnowledgeSpaceScope,
        limit: u32,
    ) -> Result<Vec<ArchiveGroupKnowledgeSpaceCommand>, KnowledgeGroupSpaceBindingStoreError>;

    /// Returns bounded archive work for the deployment organization of one worker-authorized
    /// tenant. The persisted row supplies each command's organization scope; the deployment
    /// organization predicate prevents a worker from scanning another organization's bindings
    /// that happen to share the tenant.
    async fn list_resumable_group_space_archives_for_tenant(
        &self,
        tenant_id: u64,
        organization_id: u64,
        limit: u32,
    ) -> Result<Vec<ArchiveGroupKnowledgeSpaceCommand>, KnowledgeGroupSpaceBindingStoreError>;

    /// Finalizes a lease-owned archive after external cleanup converges. This deactivates the
    /// local member projection, writes the replay inbox event, and marks the binding terminal.
    async fn complete_group_space_archive(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
        archive_lease_token: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    /// Persists one bounded page of direct-ACL cleanup. `cleanup_completed` disambiguates a
    /// completed first page from the offset-pagination restart cursor `None` used after deletes.
    async fn advance_group_space_archive_acl_cleanup(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
        archive_lease_token: &str,
        next_cursor: Option<String>,
        cleanup_completed: bool,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;

    /// Releases a failed archive lease without reopening the binding. A retry of the same
    /// deterministic IM source event can immediately resume the fail-closed saga.
    async fn release_group_space_archive_lease(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
        archive_lease_token: &str,
        error_code: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeGroupSpaceBindingStoreError {
    #[error("group knowledge space invalid request: {0}")]
    InvalidRequest(String),
    #[error("group knowledge space not found for conversation: {0}")]
    NotFound(String),
    #[error("group knowledge space conflict: {0}")]
    Conflict(String),
    #[error("group knowledge space invalid lifecycle transition: {0}")]
    InvalidLifecycle(String),
    #[error("group knowledge space internal error: {0}")]
    Internal(String),
}
