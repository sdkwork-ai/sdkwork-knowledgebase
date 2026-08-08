use crate::{
    group_space_access::{knowledge_access_role, GroupKnowledgeSpaceAccessAuthorizer},
    okf::OkfBundleInitializerService,
    ports::{
        knowledge_access_control::{
            GrantKnowledgeSpaceAccessRequest, KnowledgeAccessControl, KnowledgeAccessControlError,
            KnowledgeAccessRole, KnowledgeSubjectType, ListKnowledgeSpaceMembersRequest,
            RevokeKnowledgeSpaceAccessRequest,
        },
        knowledge_group_space_binding_store::{
            ArchiveGroupKnowledgeSpaceCommand, GroupKnowledgeSpaceMembershipChange,
            GroupKnowledgeSpaceScope, GroupKnowledgeSpaceTarget, KnowledgeGroupSpaceBindingStore,
            KnowledgeGroupSpaceBindingStoreError, ReserveGroupKnowledgeSpaceRequest,
            SynchronizeGroupKnowledgeSpaceMembersCommand,
        },
        knowledge_space_store::{KnowledgeSpaceStore, KnowledgeSpaceStoreError},
    },
    space::{KnowledgeSpaceService, KnowledgeSpaceServiceError},
    wiki_initialization::KnowledgeWikiInitializationService,
};
use sdkwork_knowledgebase_contract::{
    group_space::{
        ArchiveGroupKnowledgeSpaceRequest, EnsureGroupKnowledgeSpaceRequest,
        GroupKnowledgeSpaceBinding, GroupKnowledgeSpaceLifecycleState, GroupKnowledgeSpaceMember,
        GroupKnowledgeSpaceMemberRole, SynchronizeGroupKnowledgeSpaceMembersRequest,
    },
    space::KnowledgeSpace,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

const GROUP_SPACE_OWNER_SUBJECT_TYPE: &str = "app";
const GROUP_SPACE_OWNER_SUBJECT_PREFIX: &str = "sdkwork-knowledgebase:group-binding:";
const GROUP_SPACE_ARCHIVE_ACL_PAGE_SIZE: u32 = 200;
const TRUSTED_IM_SERVICE_ACTOR_ID: &str = "sdkwork-im";

/// Orchestrates the authoritative group-to-knowledgebase aggregate. IM supplies a trusted,
/// versioned membership snapshot through an approved backend SDK; this service owns the KB
/// lifecycle, direct-user ACL projection, and fail-closed recovery behavior.
pub struct KnowledgeGroupKnowledgeSpaceService<'a> {
    binding_store: &'a dyn KnowledgeGroupSpaceBindingStore,
    space_store: &'a dyn KnowledgeSpaceStore,
    okf_bundle_initializer: &'a OkfBundleInitializerService<'a>,
    drive_space_provisioner:
        &'a dyn crate::ports::knowledge_drive_space::KnowledgeDriveSpaceProvisioner,
    access_control: &'a dyn KnowledgeAccessControl,
    operator_id: String,
    publication_actor_id: Option<u64>,
    wiki_initializer: Option<&'a KnowledgeWikiInitializationService<'a>>,
}

impl<'a> KnowledgeGroupKnowledgeSpaceService<'a> {
    pub fn new(
        binding_store: &'a dyn KnowledgeGroupSpaceBindingStore,
        space_store: &'a dyn KnowledgeSpaceStore,
        okf_bundle_initializer: &'a OkfBundleInitializerService<'a>,
        drive_space_provisioner: &'a dyn crate::ports::knowledge_drive_space::KnowledgeDriveSpaceProvisioner,
        access_control: &'a dyn KnowledgeAccessControl,
        operator_id: impl Into<String>,
        publication_actor_id: Option<u64>,
    ) -> Self {
        Self {
            binding_store,
            space_store,
            okf_bundle_initializer,
            drive_space_provisioner,
            access_control,
            operator_id: operator_id.into().trim().to_string(),
            publication_actor_id,
            wiki_initializer: None,
        }
    }

    pub fn with_wiki_initializer(
        mut self,
        wiki_initializer: &'a KnowledgeWikiInitializationService<'a>,
    ) -> Self {
        self.wiki_initializer = Some(wiki_initializer);
        self
    }

    pub async fn ensure(
        &self,
        scope: GroupKnowledgeSpaceScope,
        actor_id: &str,
        request: EnsureGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.validate_ensure_command(actor_id, &request.members)?;
        let reservation = self
            .binding_store
            .reserve_group_space(ReserveGroupKnowledgeSpaceRequest {
                scope,
                conversation_id: request.conversation_id,
                group_name: request.group_name,
                source_event_id: request.source_event_id,
                provisioning_idempotency_key: request.provisioning_idempotency_key,
                created_by: actor_id.to_string(),
                membership_epoch: request.membership_epoch,
                members: request.members,
            })
            .await?;
        self.complete_reservation(
            scope,
            actor_id,
            reservation.binding,
            reservation.requires_provisioning,
            reservation.provisioning_lease_token,
            reservation.space,
        )
        .await
    }

    /// Creates or resumes a group knowledge space from IM's durable internal lifecycle outbox.
    ///
    /// The RPC adapter admits only the mTLS-authenticated `sdkwork-im` service with a verified,
    /// short-lived service caller context. IM performs the user owner/admin authorization before
    /// producing that outbox event; replay delivery must therefore never fabricate a user session
    /// or re-authorize against a membership snapshot that may already have changed.
    pub async fn ensure_from_im(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        request: EnsureGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceServiceError> {
        require_trusted_im_service_actor(service_actor_id)?;
        let reservation = self
            .binding_store
            .reserve_group_space(ReserveGroupKnowledgeSpaceRequest {
                scope,
                conversation_id: request.conversation_id,
                group_name: request.group_name,
                source_event_id: request.source_event_id,
                provisioning_idempotency_key: request.provisioning_idempotency_key,
                created_by: service_actor_id.to_string(),
                membership_epoch: request.membership_epoch,
                members: request.members,
            })
            .await?;
        self.complete_reservation(
            scope,
            service_actor_id,
            reservation.binding,
            reservation.requires_provisioning,
            reservation.provisioning_lease_token,
            reservation.space,
        )
        .await
    }

    pub async fn synchronize_members(
        &self,
        scope: GroupKnowledgeSpaceScope,
        actor_id: &str,
        request: SynchronizeGroupKnowledgeSpaceMembersRequest,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupKnowledgeSpaceServiceError> {
        if is_blank(Some(actor_id)) {
            return Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidRequest(
                "IM synchronization actor is required".to_string(),
            ));
        }
        validate_group_member_snapshot_size(request.members.len())?;
        let command = SynchronizeGroupKnowledgeSpaceMembersCommand {
            scope,
            conversation_id: request.conversation_id,
            group_name: request.group_name,
            source_event_id: request.source_event_id,
            target: GroupKnowledgeSpaceTarget {
                knowledgebase_binding_id: request.knowledgebase_binding_id,
                knowledgebase_binding_uuid: request.knowledgebase_binding_uuid,
                knowledge_space_id: request.knowledge_space_id,
                knowledge_space_uuid: request.knowledge_space_uuid,
            },
            membership_epoch: request.membership_epoch,
            upstream_link_generation: request.upstream_link_generation,
            members: request.members,
        };
        let existing_binding = self
            .binding_store
            .get_group_space(scope, &command.conversation_id)
            .await?;
        if existing_binding.lifecycle_state == GroupKnowledgeSpaceLifecycleState::Provisioning {
            // A provisioning binding has no user-visible Drive surface yet, so its snapshot can
            // be atomically updated without an external ACL projection.
            return self
                .binding_store
                .synchronize_group_members(command)
                .await
                .map_err(Into::into);
        }

        let reservation = self
            .binding_store
            .prepare_group_membership_sync(command.clone())
            .await?;
        if !reservation.requires_acl_projection {
            return Ok(GroupKnowledgeSpaceMembershipChange {
                binding: reservation.binding,
                previous_members: reservation.previous_members,
                current_members: reservation.current_members,
                requires_acl_projection: false,
            });
        }

        let synchronization_lease_token =
            reservation.synchronization_lease_token.ok_or_else(|| {
                KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                    "a reserved group membership synchronization must include its lease token"
                        .to_string(),
                )
            })?;
        let space_id = reservation.binding.space_id.ok_or_else(|| {
            KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                "an active group binding must reference a knowledge space".to_string(),
            )
        })?;
        let space = self.space_store.get_group_managed_space(space_id).await?;
        let projection_fence = GroupMembershipProjectionFence {
            command: &command,
            synchronization_lease_token: &synchronization_lease_token,
        };
        if let Err(error) = self
            .project_acl_delta(
                scope,
                &space,
                &reservation.previous_members,
                &reservation.current_members,
                Some(&projection_fence),
            )
            .await
        {
            if matches!(
                &error,
                KnowledgeGroupKnowledgeSpaceServiceError::MembershipProjectionFencedByArchive
            ) {
                self.binding_store
                    .settle_group_membership_sync_after_archive(
                        command.clone(),
                        &synchronization_lease_token,
                    )
                    .await?;
            } else {
                // A failed compensation remains lease-owned while archive is in progress. That
                // prevents terminal archival until a durable worker can repeat the final Drive
                // ACL sweep; an active binding still follows the normal failed-projection path.
                // The compensation outcome is observable: a durable failure is recorded so an
                // operator can investigate instead of the error vanishing silently.
                let binding_id = command.target.knowledgebase_binding_id;
                if let Err(compensation_error) = self
                    .binding_store
                    .fail_group_membership_sync(
                        command,
                        &synchronization_lease_token,
                        "group_membership_acl_projection_failed",
                    )
                    .await
                {
                    tracing::error!(
                        %compensation_error,
                        binding_id,
                        "group membership sync compensation failed; binding stays lease-owned"
                    );
                }
            }
            return Err(error);
        }

        self.binding_store
            .complete_group_membership_sync(command, &synchronization_lease_token)
            .await
            .map_err(Into::into)
    }

    /// Synchronizes an IM membership snapshot from the durable internal lifecycle channel.
    ///
    /// IM has already authorized the group mutation before enqueueing its outbox event. The
    /// service identity remains the authority at replay time; member records are a projection
    /// input and must never be treated as a replacement authorization source.
    pub async fn synchronize_members_from_im(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        request: SynchronizeGroupKnowledgeSpaceMembersRequest,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupKnowledgeSpaceServiceError> {
        require_trusted_im_service_actor(service_actor_id)?;
        self.synchronize_members(scope, service_actor_id, request)
            .await
    }

    pub async fn archive(
        &self,
        scope: GroupKnowledgeSpaceScope,
        actor_id: &str,
        request: ArchiveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        let binding = self
            .binding_store
            .get_group_space(scope, &request.conversation_id)
            .await?;
        let space_id = binding.space_id.ok_or_else(|| {
            KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                "group knowledge space has no bound space".to_string(),
            )
        })?;
        self.require_group_owner(scope, space_id, actor_id).await?;
        self.archive_authorized(scope, actor_id, request).await
    }

    /// Archives from the internal IM lifecycle channel. mTLS service identity and the signed
    /// caller context are verified by the RPC adapter before this method is reached. The event
    /// actor is audit attribution only: it must not be re-authorized against the mutable local
    /// membership projection, because an owner can leave or transfer ownership before delivery.
    pub async fn archive_from_im(
        &self,
        scope: GroupKnowledgeSpaceScope,
        service_actor_id: &str,
        archived_by: &str,
        request: ArchiveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        require_trusted_im_service_actor(service_actor_id)?;
        if is_blank(Some(archived_by)) {
            return Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidRequest(
                "IM archive audit actor is required".to_string(),
            ));
        }
        self.archive_authorized(scope, archived_by, request).await
    }

    async fn archive_authorized(
        &self,
        scope: GroupKnowledgeSpaceScope,
        archived_by: &str,
        request: ArchiveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        let command = ArchiveGroupKnowledgeSpaceCommand {
            scope,
            conversation_id: request.conversation_id,
            source_event_id: request.source_event_id,
            target: GroupKnowledgeSpaceTarget {
                knowledgebase_binding_id: request.knowledgebase_binding_id,
                knowledgebase_binding_uuid: request.knowledgebase_binding_uuid,
                knowledge_space_id: request.knowledge_space_id,
                knowledge_space_uuid: request.knowledge_space_uuid,
            },
            membership_epoch: request.membership_epoch,
            upstream_link_generation: request.upstream_link_generation,
            archived_by: archived_by.to_string(),
        };
        self.archive_command(command).await
    }

    /// Runs one bounded archive-saga step from durable work. The worker calls this only after it
    /// has loaded a command reconstructed from the binding's immutable archive intent.
    pub async fn resume_archiving_from_worker(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.archive_command(command).await
    }

    async fn archive_command(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        let scope = command.scope;
        let reservation = self
            .binding_store
            .begin_group_space_archive(command.clone())
            .await?;
        if !reservation.requires_archive {
            return Ok(reservation.binding);
        }
        let archive_lease_token = reservation.archive_lease_token.ok_or_else(|| {
            KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                "a reserved group archive must include its lease token".to_string(),
            )
        })?;

        let archive_result = async {
            let space = reservation.space.ok_or_else(|| {
                KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                    "an archiving group binding must reference a knowledge space".to_string(),
                )
            })?;
            let mut archive_binding = reservation.binding;
            if self
                .binding_store
                .has_active_group_membership_projection_lease(scope, archive_binding.id)
                .await?
            {
                // A membership worker may have passed its pre-grant fence and still be making
                // an external Drive mutation. Do not begin a sweep that could be invalidated by
                // that worker. Clear any previously completed marker before releasing this
                // archive lease so a later retry must perform a fresh full sweep after the
                // projection has quiesced.
                return self
                    .binding_store
                    .advance_group_space_archive_acl_cleanup(
                        command.clone(),
                        &archive_lease_token,
                        None,
                        false,
                    )
                    .await
                    .map_err(Into::into);
            }
            if archive_binding.archive_acl_cleanup_completed_at.is_none() {
                let progress = self
                    .revoke_direct_group_space_access_page(
                        scope,
                        &space,
                        archive_binding.archive_acl_cursor.clone(),
                    )
                    .await?;
                let cleanup_completed =
                    matches!(progress, GroupArchiveAclCleanupProgress::Complete);
                let next_cursor = match progress {
                    GroupArchiveAclCleanupProgress::Complete => None,
                    GroupArchiveAclCleanupProgress::Pending { next_cursor } => next_cursor,
                };
                archive_binding = self
                    .binding_store
                    .advance_group_space_archive_acl_cleanup(
                        command.clone(),
                        &archive_lease_token,
                        next_cursor,
                        cleanup_completed,
                    )
                    .await?;
                if !cleanup_completed {
                    // Archive intent and its progress are durable. Return quickly so the IM
                    // relay lease is never held while a large group's ACL cleanup converges.
                    return Ok(archive_binding);
                }
            }
            if self
                .binding_store
                .has_active_group_membership_projection_lease(scope, archive_binding.id)
                .await?
            {
                // A provider operation raced with this completed sweep. Reset the durable
                // marker before releasing the archive lease, so the next attempt is forced to
                // perform a complete fresh sweep after that projection has quiesced.
                return self
                    .binding_store
                    .advance_group_space_archive_acl_cleanup(
                        command.clone(),
                        &archive_lease_token,
                        None,
                        false,
                    )
                    .await
                    .map_err(Into::into);
            }
            self.space_store
                .archive_group_managed_space(space.id)
                .await?;
            self.binding_store
                .complete_group_space_archive(command.clone(), &archive_lease_token)
                .await
                .map_err(Into::into)
        }
        .await;

        match archive_result {
            Ok(binding) => Ok(binding),
            Err(error) => {
                // The binding remains fail-closed in `archiving`; releasing only this attempt's
                // lease lets the deterministic IM outbox retry converge immediately.
                let _ = self
                    .binding_store
                    .release_group_space_archive_lease(
                        command,
                        &archive_lease_token,
                        "group_space_archive_saga_failed",
                    )
                    .await;
                Err(error)
            }
        }
    }

    pub async fn retrieve(
        &self,
        scope: GroupKnowledgeSpaceScope,
        conversation_id: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupKnowledgeSpaceServiceError> {
        self.binding_store
            .get_group_space(scope, conversation_id)
            .await
            .map_err(Into::into)
    }

    async fn complete_reservation(
        &self,
        scope: GroupKnowledgeSpaceScope,
        actor_id: &str,
        binding: GroupKnowledgeSpaceBinding,
        requires_provisioning: bool,
        provisioning_lease_token: Option<String>,
        reservation_space: Option<KnowledgeSpace>,
    ) -> Result<GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceServiceError> {
        if !requires_provisioning {
            return Ok(GroupKnowledgeSpaceOperation {
                binding,
                space: reservation_space,
            });
        }
        let lease_token = provisioning_lease_token.ok_or_else(|| {
            KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                "a provisioning reservation must include its lease token".to_string(),
            )
        })?;
        let space_id = binding.space_id.ok_or_else(|| {
            KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                "a provisioning group binding must reference a hidden space".to_string(),
            )
        })?;

        let owner_subject_id = format!("{GROUP_SPACE_OWNER_SUBJECT_PREFIX}{}", binding.uuid);
        let initialized = match self
            .space_service(scope)
            .initialize_group_managed_space(
                space_id,
                GROUP_SPACE_OWNER_SUBJECT_TYPE,
                &owner_subject_id,
            )
            .await
        {
            Ok(space) => space,
            Err(error) => {
                return self
                    .fail_provisioning(scope, &binding, &lease_token, actor_id, error)
                    .await
            }
        };

        let current_members = self
            .binding_store
            .list_active_group_members(scope, binding.id)
            .await?;
        if let Err(error) = self
            .project_acl_delta(scope, &initialized, &[], &current_members, None)
            .await
        {
            return self
                .fail_provisioning(scope, &binding, &lease_token, actor_id, error)
                .await;
        }

        let active_space = match self
            .space_service(scope)
            .activate_group_managed_space(space_id)
            .await
        {
            Ok(space) => space,
            Err(error) => {
                return self
                    .fail_provisioning(scope, &binding, &lease_token, actor_id, error)
                    .await
            }
        };
        let active_binding = self
            .binding_store
            .mark_group_space_active(scope, binding.id, &lease_token, actor_id)
            .await?;
        Ok(GroupKnowledgeSpaceOperation {
            binding: active_binding,
            space: Some(active_space),
        })
    }

    async fn fail_provisioning<T>(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding: &GroupKnowledgeSpaceBinding,
        lease_token: &str,
        actor_id: &str,
        error: T,
    ) -> Result<GroupKnowledgeSpaceOperation, KnowledgeGroupKnowledgeSpaceServiceError>
    where
        T: std::fmt::Display,
    {
        if let Some(space_id) = binding.space_id {
            if let Err(cleanup_error) = self.space_store.mark_space_deleted(space_id).await {
                tracing::error!(
                    space_id,
                    binding_id = binding.id,
                    error = %cleanup_error,
                    "failed to compensate group knowledge space after provisioning failure"
                );
            }
        }
        if let Err(cleanup_error) = self
            .binding_store
            .mark_group_space_failed(
                scope,
                binding.id,
                lease_token,
                "group_space_provisioning_failed",
                actor_id,
            )
            .await
        {
            tracing::error!(
                binding_id = binding.id,
                error = %cleanup_error,
                "failed to persist group knowledge space provisioning failure"
            );
        }
        Err(KnowledgeGroupKnowledgeSpaceServiceError::Provisioning(
            error.to_string(),
        ))
    }

    async fn require_group_owner(
        &self,
        scope: GroupKnowledgeSpaceScope,
        space_id: u64,
        actor_id: &str,
    ) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
        let authorizer = GroupKnowledgeSpaceAccessAuthorizer::new(self.binding_store);
        let is_group_space = authorizer
            .authorize(scope, space_id, actor_id, KnowledgeAccessRole::Owner)
            .await?;
        if is_group_space.is_none() {
            return Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidRequest(
                "the requested space is not group-managed".to_string(),
            ));
        }
        Ok(())
    }

    /// Processes one durable ACL cleanup page. Drive currently uses offset page tokens, so a
    /// page that performs deletions deliberately restarts at offset zero on the next attempt;
    /// advancing its old offset would skip members shifted by the revocations.
    async fn revoke_direct_group_space_access_page(
        &self,
        scope: GroupKnowledgeSpaceScope,
        space: &KnowledgeSpace,
        cursor: Option<String>,
    ) -> Result<GroupArchiveAclCleanupProgress, KnowledgeGroupKnowledgeSpaceServiceError> {
        revoke_direct_group_space_access_page_with_access_control(
            self.access_control,
            self.operator_id.as_str(),
            scope,
            space,
            cursor,
        )
        .await
    }

    fn validate_ensure_command(
        &self,
        actor_id: &str,
        members: &[GroupKnowledgeSpaceMember],
    ) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
        validate_group_member_snapshot_size(members.len())?;
        validate_group_command_actor(
            actor_id,
            self.operator_id.as_str(),
            members,
            GroupKnowledgeSpaceCommandAuthorization::OwnerOrAdmin,
        )
    }

    fn space_service(&self, scope: GroupKnowledgeSpaceScope) -> KnowledgeSpaceService<'_> {
        let service = KnowledgeSpaceService::new(self.space_store, self.okf_bundle_initializer)
            .with_drive_context(scope.tenant_id.to_string(), self.operator_id.clone())
            .with_drive_space_provisioner(self.drive_space_provisioner)
            .with_access_control(self.access_control);
        let service = match self.publication_actor_id {
            Some(actor_id) => service.with_wiki_context(
                crate::ports::knowledge_wiki_persistence::WikiPersistenceScope {
                    tenant_id: scope.tenant_id,
                    organization_id: scope.organization_id,
                },
                actor_id,
            ),
            None => service,
        };
        match self.wiki_initializer {
            Some(initializer) => service.with_wiki_initializer(initializer),
            None => service,
        }
    }

    async fn project_acl_delta(
        &self,
        scope: GroupKnowledgeSpaceScope,
        space: &KnowledgeSpace,
        previous_members: &[GroupKnowledgeSpaceMember],
        current_members: &[GroupKnowledgeSpaceMember],
        projection_fence: Option<&GroupMembershipProjectionFence<'_>>,
    ) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
        let drive_space_id = space.drive_space_id.as_deref().ok_or_else(|| {
            KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
                "group knowledge space must have a Drive space before ACL projection".to_string(),
            )
        })?;
        for previous in previous_members {
            let previous_access = previous.role.access_level();
            let next_access = current_members
                .iter()
                .find(|current| current.actor_id == previous.actor_id)
                .and_then(|current| current.role.access_level());
            if previous_access.is_some() && previous_access != next_access {
                self.access_control
                    .revoke_space_access(RevokeKnowledgeSpaceAccessRequest {
                        tenant_id: scope.tenant_id.to_string(),
                        drive_space_id: drive_space_id.to_string(),
                        drive_node_id: None,
                        subject_type: KnowledgeSubjectType::User,
                        subject_id: previous.actor_id.clone(),
                        operator_id: self.operator_id.clone(),
                    })
                    .await?;
            }
        }
        for current in current_members {
            let Some(access_level) = current.role.access_level() else {
                continue;
            };
            let previous_access = previous_members
                .iter()
                .find(|previous| previous.actor_id == current.actor_id)
                .and_then(|previous| previous.role.access_level());
            if previous_access == Some(access_level) {
                continue;
            }
            if let Some(fence) = projection_fence {
                self.require_membership_projection_fence(fence).await?;
            }
            self.access_control
                .grant_space_access(GrantKnowledgeSpaceAccessRequest {
                    tenant_id: scope.tenant_id.to_string(),
                    drive_space_id: drive_space_id.to_string(),
                    drive_node_id: None,
                    subject_type: KnowledgeSubjectType::User,
                    subject_id: current.actor_id.clone(),
                    role: knowledge_access_role(access_level),
                    operator_id: self.operator_id.clone(),
                })
                .await?;
            if let Some(fence) = projection_fence {
                let lease_is_current = self
                    .binding_store
                    .is_group_membership_projection_lease_current(
                        fence.command,
                        fence.synchronization_lease_token,
                    )
                    .await?;
                if !lease_is_current {
                    // Archive may have started after the pre-grant fence. This compensation is
                    // mandatory: on failure the projection lease stays durable and prevents
                    // archive terminalization until a worker repeats the final ACL sweep.
                    self.access_control
                        .revoke_space_access(RevokeKnowledgeSpaceAccessRequest {
                            tenant_id: scope.tenant_id.to_string(),
                            drive_space_id: drive_space_id.to_string(),
                            drive_node_id: None,
                            subject_type: KnowledgeSubjectType::User,
                            subject_id: current.actor_id.clone(),
                            operator_id: self.operator_id.clone(),
                        })
                        .await?;
                    return Err(
                        KnowledgeGroupKnowledgeSpaceServiceError::MembershipProjectionFencedByArchive,
                    );
                }
            }
        }
        Ok(())
    }

    async fn require_membership_projection_fence(
        &self,
        fence: &GroupMembershipProjectionFence<'_>,
    ) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
        if self
            .binding_store
            .is_group_membership_projection_lease_current(
                fence.command,
                fence.synchronization_lease_token,
            )
            .await?
        {
            return Ok(());
        }
        Err(KnowledgeGroupKnowledgeSpaceServiceError::MembershipProjectionFencedByArchive)
    }
}

#[derive(Clone, Copy)]
enum GroupKnowledgeSpaceCommandAuthorization {
    OwnerOrAdmin,
}

fn validate_group_member_snapshot_size(
    member_count: usize,
) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
    if member_count > sdkwork_knowledgebase_contract::group_space::GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS
    {
        return Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidRequest(format!(
            "group membership snapshot must contain no more than {} members",
            sdkwork_knowledgebase_contract::group_space::GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS
        )));
    }
    Ok(())
}

fn validate_group_command_actor(
    actor_id: &str,
    operator_id: &str,
    members: &[GroupKnowledgeSpaceMember],
    authorization: GroupKnowledgeSpaceCommandAuthorization,
) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
    if is_blank(Some(actor_id)) || is_blank(Some(operator_id)) {
        return Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidRequest(
            "group command actor and KB operator are required".to_string(),
        ));
    }

    let permitted = members.iter().any(|member| {
        member.actor_id == actor_id
            && matches!(
                (authorization, member.role),
                (
                    GroupKnowledgeSpaceCommandAuthorization::OwnerOrAdmin,
                    GroupKnowledgeSpaceMemberRole::Owner | GroupKnowledgeSpaceMemberRole::Admin
                )
            )
    });
    if permitted {
        return Ok(());
    }

    let detail = match authorization {
        GroupKnowledgeSpaceCommandAuthorization::OwnerOrAdmin => {
            "only the current IM group owner or administrator can create its knowledgebase"
        }
    };
    Err(KnowledgeGroupKnowledgeSpaceServiceError::Denied(
        detail.to_string(),
    ))
}

fn require_trusted_im_service_actor(
    service_actor_id: &str,
) -> Result<(), KnowledgeGroupKnowledgeSpaceServiceError> {
    if service_actor_id != TRUSTED_IM_SERVICE_ACTOR_ID {
        return Err(KnowledgeGroupKnowledgeSpaceServiceError::Denied(
            "only the trusted IM internal lifecycle service may mutate group knowledge spaces"
                .to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupKnowledgeSpaceOperation {
    pub binding: GroupKnowledgeSpaceBinding,
    pub space: Option<KnowledgeSpace>,
}

enum GroupArchiveAclCleanupProgress {
    Pending { next_cursor: Option<String> },
    Complete,
}

async fn revoke_direct_group_space_access_page_with_access_control(
    access_control: &dyn KnowledgeAccessControl,
    operator_id: &str,
    scope: GroupKnowledgeSpaceScope,
    space: &KnowledgeSpace,
    cursor: Option<String>,
) -> Result<GroupArchiveAclCleanupProgress, KnowledgeGroupKnowledgeSpaceServiceError> {
    let drive_space_id = space.drive_space_id.as_deref().ok_or_else(|| {
        KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
            "group knowledge space must have a Drive space before archive cleanup".to_string(),
        )
    })?;
    let page = access_control
        .list_space_members(ListKnowledgeSpaceMembersRequest {
            tenant_id: scope.tenant_id.to_string(),
            drive_space_id: drive_space_id.to_string(),
            drive_node_id: None,
            cursor: cursor.clone(),
            page_size: Some(GROUP_SPACE_ARCHIVE_ACL_PAGE_SIZE),
        })
        .await?;
    let mut revoked_direct_user = false;
    for member in page.members {
        if member.inherited || member.subject_type != KnowledgeSubjectType::User {
            continue;
        }
        access_control
            .revoke_space_access(RevokeKnowledgeSpaceAccessRequest {
                tenant_id: scope.tenant_id.to_string(),
                drive_space_id: drive_space_id.to_string(),
                drive_node_id: None,
                subject_type: KnowledgeSubjectType::User,
                subject_id: member.subject_id,
                operator_id: operator_id.to_string(),
            })
            .await?;
        revoked_direct_user = true;
    }
    if revoked_direct_user {
        // Drive page tokens are offsets. Revocation shifts the remaining page, so restart from
        // zero rather than advancing a stale offset and silently skipping direct-user grants.
        return Ok(GroupArchiveAclCleanupProgress::Pending { next_cursor: None });
    }
    let Some(next_cursor) = page.next_cursor else {
        return Ok(GroupArchiveAclCleanupProgress::Complete);
    };
    if is_blank(Some(next_cursor.as_str())) || cursor.as_deref() == Some(next_cursor.as_str()) {
        return Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidLifecycle(
            "Drive member pagination cursor did not advance during archive cleanup".to_string(),
        ));
    }
    Ok(GroupArchiveAclCleanupProgress::Pending {
        next_cursor: Some(next_cursor),
    })
}

struct GroupMembershipProjectionFence<'a> {
    command: &'a SynchronizeGroupKnowledgeSpaceMembersCommand,
    synchronization_lease_token: &'a str,
}

#[derive(Debug, Error)]
pub enum KnowledgeGroupKnowledgeSpaceServiceError {
    #[error("group knowledge space request is invalid: {0}")]
    InvalidRequest(String),
    #[error("group knowledge space access denied: {0}")]
    Denied(String),
    #[error("group knowledge space lifecycle is invalid: {0}")]
    InvalidLifecycle(String),
    #[error("group membership ACL projection was fenced by archive")]
    MembershipProjectionFencedByArchive,
    #[error("group knowledge space provisioning failed: {0}")]
    Provisioning(String),
    #[error(transparent)]
    Binding(#[from] KnowledgeGroupSpaceBindingStoreError),
    #[error(transparent)]
    SpaceStore(#[from] KnowledgeSpaceStoreError),
    #[error(transparent)]
    Space(#[from] KnowledgeSpaceServiceError),
    #[error(transparent)]
    AccessControl(#[from] KnowledgeAccessControlError),
    #[error(transparent)]
    Authorization(#[from] crate::group_space_access::GroupKnowledgeSpaceAccessAuthorizerError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::knowledge_access_control::{
        KnowledgeAccessCheckRequest, KnowledgeAccessGrant, KnowledgeNodeAccessCheckRequest,
        KnowledgeSpaceMember, KnowledgeSpaceMemberList,
    };
    use async_trait::async_trait;
    use sdkwork_knowledgebase_contract::group_space::GroupKnowledgeSpacePrincipalKind;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    fn member(actor_id: &str, role: GroupKnowledgeSpaceMemberRole) -> GroupKnowledgeSpaceMember {
        GroupKnowledgeSpaceMember {
            principal_kind: GroupKnowledgeSpacePrincipalKind::User,
            actor_id: actor_id.to_string(),
            role,
            access_level: None,
        }
    }

    fn members() -> Vec<GroupKnowledgeSpaceMember> {
        vec![
            member("owner", GroupKnowledgeSpaceMemberRole::Owner),
            member("admin", GroupKnowledgeSpaceMemberRole::Admin),
            member("member", GroupKnowledgeSpaceMemberRole::Member),
            member("guest", GroupKnowledgeSpaceMemberRole::Guest),
        ]
    }

    #[derive(Default)]
    struct OffsetDriveAccessControl {
        members: Arc<Mutex<Vec<KnowledgeSpaceMember>>>,
        fail_next_revoke: AtomicBool,
    }

    impl OffsetDriveAccessControl {
        fn with_members(members: Vec<KnowledgeSpaceMember>) -> Self {
            Self {
                members: Arc::new(Mutex::new(members)),
                fail_next_revoke: AtomicBool::new(false),
            }
        }

        fn direct_user_count(&self) -> usize {
            self.members
                .lock()
                .expect("members mutex")
                .iter()
                .filter(|member| {
                    !member.inherited && member.subject_type == KnowledgeSubjectType::User
                })
                .count()
        }
    }

    #[async_trait]
    impl KnowledgeAccessControl for OffsetDriveAccessControl {
        async fn check_space_access(
            &self,
            _request: KnowledgeAccessCheckRequest,
        ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
            Err(KnowledgeAccessControlError::Internal(
                "not used by archive ACL cleanup test".to_string(),
            ))
        }

        async fn check_node_access(
            &self,
            _request: KnowledgeNodeAccessCheckRequest,
        ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
            Err(KnowledgeAccessControlError::Internal(
                "not used by archive ACL cleanup test".to_string(),
            ))
        }

        async fn grant_space_access(
            &self,
            request: GrantKnowledgeSpaceAccessRequest,
        ) -> Result<(), KnowledgeAccessControlError> {
            self.members
                .lock()
                .expect("members mutex")
                .push(KnowledgeSpaceMember {
                    subject_type: request.subject_type,
                    subject_id: request.subject_id,
                    role: request.role,
                    inherited: false,
                });
            Ok(())
        }

        async fn revoke_space_access(
            &self,
            request: RevokeKnowledgeSpaceAccessRequest,
        ) -> Result<(), KnowledgeAccessControlError> {
            if self.fail_next_revoke.swap(false, Ordering::SeqCst) {
                return Err(KnowledgeAccessControlError::Upstream(
                    "injected revoke failure".to_string(),
                ));
            }
            self.members
                .lock()
                .expect("members mutex")
                .retain(|member| {
                    member.inherited
                        || member.subject_type != request.subject_type
                        || member.subject_id != request.subject_id
                });
            Ok(())
        }

        async fn list_space_members(
            &self,
            request: ListKnowledgeSpaceMembersRequest,
        ) -> Result<KnowledgeSpaceMemberList, KnowledgeAccessControlError> {
            let page_size = request
                .page_size
                .unwrap_or(GROUP_SPACE_ARCHIVE_ACL_PAGE_SIZE) as usize;
            let offset = request
                .cursor
                .as_deref()
                .map(|cursor| {
                    cursor
                        .strip_prefix("offset:")
                        .ok_or_else(|| {
                            KnowledgeAccessControlError::InvalidRequest(
                                "invalid offset cursor".to_string(),
                            )
                        })?
                        .parse::<usize>()
                        .map_err(|_| {
                            KnowledgeAccessControlError::InvalidRequest(
                                "invalid offset cursor".to_string(),
                            )
                        })
                })
                .transpose()?
                .unwrap_or(0);
            let members = self.members.lock().expect("members mutex");
            if offset > members.len() {
                return Err(KnowledgeAccessControlError::InvalidRequest(
                    "offset cursor exceeds member count".to_string(),
                ));
            }
            let end = (offset + page_size).min(members.len());
            Ok(KnowledgeSpaceMemberList {
                members: members[offset..end].to_vec(),
                next_cursor: (end < members.len()).then(|| format!("offset:{end}")),
            })
        }
    }

    fn archive_scope() -> GroupKnowledgeSpaceScope {
        GroupKnowledgeSpaceScope {
            tenant_id: 1,
            organization_id: 2,
        }
    }

    fn archive_space() -> KnowledgeSpace {
        KnowledgeSpace {
            id: 1,
            uuid: "space-uuid".to_string(),
            name: "group-space".to_string(),
            description: None,
            drive_space_id: Some("drive-space-1".to_string()),
            status: sdkwork_knowledgebase_contract::space::KnowledgeSpaceStatus::Active,
            okf_bundle_initialized: true,
            knowledge_mode: Default::default(),
        }
    }

    fn direct_user(subject_id: impl Into<String>) -> KnowledgeSpaceMember {
        KnowledgeSpaceMember {
            subject_type: KnowledgeSubjectType::User,
            subject_id: subject_id.into(),
            role: KnowledgeAccessRole::Reader,
            inherited: false,
        }
    }

    #[test]
    fn ensure_allows_current_owner_or_admin_only() {
        let members = members();

        for actor_id in ["owner", "admin"] {
            assert!(validate_group_command_actor(
                actor_id,
                "kb-operator",
                &members,
                GroupKnowledgeSpaceCommandAuthorization::OwnerOrAdmin,
            )
            .is_ok());
        }
        for actor_id in ["member", "guest"] {
            assert!(matches!(
                validate_group_command_actor(
                    actor_id,
                    "kb-operator",
                    &members,
                    GroupKnowledgeSpaceCommandAuthorization::OwnerOrAdmin,
                ),
                Err(KnowledgeGroupKnowledgeSpaceServiceError::Denied(_))
            ));
        }
    }

    #[test]
    fn membership_snapshot_size_is_bounded_before_projection() {
        let maximum = sdkwork_knowledgebase_contract::group_space::GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS;
        assert!(validate_group_member_snapshot_size(maximum).is_ok());
        assert!(matches!(
            validate_group_member_snapshot_size(maximum + 1),
            Err(KnowledgeGroupKnowledgeSpaceServiceError::InvalidRequest(message))
                if message.contains("no more than")
        ));
    }

    #[test]
    fn trusted_im_lifecycle_entrypoints_reject_non_im_service_actors() {
        assert!(require_trusted_im_service_actor(TRUSTED_IM_SERVICE_ACTOR_ID).is_ok());
        assert!(matches!(
            require_trusted_im_service_actor("sdkwork-untrusted"),
            Err(KnowledgeGroupKnowledgeSpaceServiceError::Denied(_))
        ));
    }

    #[tokio::test]
    async fn archive_acl_cleanup_restarts_offset_pages_until_all_direct_users_are_removed() {
        let mut members = (0..401)
            .map(|index| direct_user(format!("user-{index}")))
            .collect::<Vec<_>>();
        members.push(KnowledgeSpaceMember {
            subject_type: KnowledgeSubjectType::App,
            subject_id: "binding-owner".to_string(),
            role: KnowledgeAccessRole::Owner,
            inherited: false,
        });
        members.push(KnowledgeSpaceMember {
            subject_type: KnowledgeSubjectType::User,
            subject_id: "inherited-user".to_string(),
            role: KnowledgeAccessRole::Reader,
            inherited: true,
        });
        let access_control = OffsetDriveAccessControl::with_members(members);
        let mut cursor = None;
        let mut steps = 0usize;

        loop {
            steps += 1;
            match revoke_direct_group_space_access_page_with_access_control(
                &access_control,
                "kb-worker",
                archive_scope(),
                &archive_space(),
                cursor,
            )
            .await
            .expect("bounded cleanup step")
            {
                GroupArchiveAclCleanupProgress::Complete => break,
                GroupArchiveAclCleanupProgress::Pending { next_cursor } => {
                    // Direct removals always restart offset pagination at zero. Advancing the
                    // old offset here would skip the shifted members on the next page.
                    assert!(next_cursor.is_none());
                    cursor = next_cursor;
                }
            }
            assert!(
                steps < 8,
                "cleanup must converge with bounded offset restarts"
            );
        }

        assert_eq!(steps, 4);
        assert_eq!(access_control.direct_user_count(), 0);
        let remaining = access_control.members.lock().expect("members mutex");
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|member| member.inherited));
        assert!(remaining
            .iter()
            .any(|member| member.subject_type == KnowledgeSubjectType::App));
    }

    #[tokio::test]
    async fn archive_reconciliation_retries_after_a_post_grant_compensation_revoke_failure() {
        let access_control = OffsetDriveAccessControl::with_members(Vec::new());
        let space = archive_space();

        assert!(matches!(
            revoke_direct_group_space_access_page_with_access_control(
                &access_control,
                "kb-worker",
                archive_scope(),
                &space,
                None,
            )
            .await
            .expect("initial empty sweep"),
            GroupArchiveAclCleanupProgress::Complete
        ));

        // Deterministic interleave: a membership worker passed its pre-grant fence, archive
        // began after the empty sweep, and the stale worker granted a direct Drive membership.
        access_control
            .grant_space_access(GrantKnowledgeSpaceAccessRequest {
                tenant_id: "1".to_string(),
                drive_space_id: "drive-space-1".to_string(),
                drive_node_id: None,
                subject_type: KnowledgeSubjectType::User,
                subject_id: "late-member".to_string(),
                role: KnowledgeAccessRole::Reader,
                operator_id: "membership-worker".to_string(),
            })
            .await
            .expect("inject stale direct grant");
        access_control
            .fail_next_revoke
            .store(true, Ordering::SeqCst);

        let compensation = revoke_direct_group_space_access_page_with_access_control(
            &access_control,
            "membership-worker",
            archive_scope(),
            &space,
            None,
        )
        .await;
        assert!(matches!(
            compensation,
            Err(KnowledgeGroupKnowledgeSpaceServiceError::AccessControl(
                KnowledgeAccessControlError::Upstream(_)
            ))
        ));
        assert_eq!(access_control.direct_user_count(), 1);

        // The archive worker's mandatory fresh sweep does not trust its earlier completed
        // marker. It deletes the surviving grant, then confirms an empty page before terminal
        // archival is permitted.
        assert!(matches!(
            revoke_direct_group_space_access_page_with_access_control(
                &access_control,
                "kb-worker",
                archive_scope(),
                &space,
                None,
            )
            .await
            .expect("reconciliation sweep"),
            GroupArchiveAclCleanupProgress::Pending { next_cursor: None }
        ));
        assert_eq!(access_control.direct_user_count(), 0);
        assert!(matches!(
            revoke_direct_group_space_access_page_with_access_control(
                &access_control,
                "kb-worker",
                archive_scope(),
                &space,
                None,
            )
            .await
            .expect("final empty verification sweep"),
            GroupArchiveAclCleanupProgress::Complete
        ));
    }
}
