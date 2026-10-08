use async_trait::async_trait;
use sdkwork_drive_workspace_service::application::permission_service::{
    CheckDriveNodePermissionCommand, GrantDriveNodePermissionCommand,
    ListDriveNodePermissionsCommand, RevokeDriveNodePermissionCommand, SqlDrivePermissionService,
};
use sdkwork_drive_workspace_service::DriveServiceError;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_access_control::{
    GrantKnowledgeSpaceAccessRequest, KnowledgeAccessCheckRequest, KnowledgeAccessControl,
    KnowledgeAccessControlError, KnowledgeAccessGrant, KnowledgeAccessRole,
    KnowledgeNodeAccessCheckRequest, KnowledgeSpaceMember, KnowledgeSpaceMemberList,
    KnowledgeSubjectType, ListKnowledgeSpaceMembersRequest, RevokeKnowledgeSpaceAccessRequest,
};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct KnowledgebaseKnowledgeAccessControlAdapter {
    service: SqlDrivePermissionService,
}

impl KnowledgebaseKnowledgeAccessControlAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self {
            service: SqlDrivePermissionService::new(pool),
        }
    }
}

fn map_drive_error(error: DriveServiceError) -> KnowledgeAccessControlError {
    match error {
        DriveServiceError::Validation(message) => {
            KnowledgeAccessControlError::InvalidRequest(message)
        }
        DriveServiceError::Conflict(message) => {
            KnowledgeAccessControlError::InvalidRequest(message)
        }
        DriveServiceError::NotFound(message) => {
            // A missing drive-side resource (space anchor node, binding) is definitive:
            // preserving NotFound lets callers retire the knowledge-space reference
            // instead of surfacing a permanent 500 for a zombie space.
            KnowledgeAccessControlError::NotFound(message)
        }
        DriveServiceError::PermissionDenied(message) => {
            KnowledgeAccessControlError::Denied(message)
        }
        DriveServiceError::Internal(message) => KnowledgeAccessControlError::Internal(message),
    }
}

fn map_access_grant(allowed: bool, effective_role: Option<String>) -> KnowledgeAccessGrant {
    KnowledgeAccessGrant {
        allowed,
        effective_role: effective_role
            .as_deref()
            .and_then(KnowledgeAccessRole::from_drive_role),
    }
}

fn map_subject_type(subject_type: KnowledgeSubjectType) -> String {
    subject_type.to_drive_subject_type().to_string()
}

#[async_trait]
impl KnowledgeAccessControl for KnowledgebaseKnowledgeAccessControlAdapter {
    async fn check_space_access(
        &self,
        request: KnowledgeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
        let node_id = self
            .service
            .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
            .await
            .map_err(map_drive_error)?;

        let check = self
            .service
            .check_node_permission(CheckDriveNodePermissionCommand {
                tenant_id: request.tenant_id,
                node_id,
                subject_type: "user".to_string(),
                subject_id: request.actor_id,
                required_role: request.required_role.to_drive_role().to_string(),
            })
            .await
            .map_err(map_drive_error)?;

        Ok(map_access_grant(check.allowed, check.effective_role))
    }

    async fn check_node_access(
        &self,
        request: KnowledgeNodeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError> {
        let check = self
            .service
            .check_node_permission(CheckDriveNodePermissionCommand {
                tenant_id: request.tenant_id,
                node_id: request.drive_node_id,
                subject_type: "user".to_string(),
                subject_id: request.actor_id,
                required_role: request.required_role.to_drive_role().to_string(),
            })
            .await
            .map_err(map_drive_error)?;

        Ok(map_access_grant(check.allowed, check.effective_role))
    }

    async fn grant_space_access(
        &self,
        request: GrantKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError> {
        let node_id = match request.drive_node_id.as_deref() {
            Some(drive_node_id) => drive_node_id.to_string(),
            None => self
                .service
                .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
                .await
                .map_err(map_drive_error)?,
        };

        self.service
            .grant_node_permission(GrantDriveNodePermissionCommand {
                tenant_id: request.tenant_id,
                node_id,
                subject_type: map_subject_type(request.subject_type),
                subject_id: request.subject_id,
                role: request.role.to_drive_role().to_string(),
                operator_id: request.operator_id,
            })
            .await
            .map_err(map_drive_error)?;

        Ok(())
    }

    async fn revoke_space_access(
        &self,
        request: RevokeKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError> {
        let node_id = match request.drive_node_id.as_deref() {
            Some(drive_node_id) => drive_node_id.to_string(),
            None => self
                .service
                .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
                .await
                .map_err(map_drive_error)?,
        };

        self.service
            .revoke_node_permission(RevokeDriveNodePermissionCommand {
                tenant_id: request.tenant_id,
                node_id,
                subject_type: map_subject_type(request.subject_type),
                subject_id: request.subject_id,
                operator_id: request.operator_id,
            })
            .await
            .map_err(map_drive_error)?;

        Ok(())
    }

    async fn list_space_members(
        &self,
        request: ListKnowledgeSpaceMembersRequest,
    ) -> Result<KnowledgeSpaceMemberList, KnowledgeAccessControlError> {
        let node_id = match request.drive_node_id.as_deref() {
            Some(drive_node_id) => drive_node_id.to_string(),
            None => self
                .service
                .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
                .await
                .map_err(map_drive_error)?,
        };

        let list = self
            .service
            .list_node_permissions(ListDriveNodePermissionsCommand {
                tenant_id: request.tenant_id,
                node_id,
                page_size: request.page_size,
                page_token: request.cursor,
            })
            .await
            .map_err(map_drive_error)?;
        // Drive permission pagination uses offset page_token; cursor is forwarded as-is.
        let mut members = Vec::with_capacity(list.items.len());
        for item in list.items {
            // Unmapped subject types or roles are skipped loudly instead of being
            // silently dropped by a filter_map, so drift between the Drive and
            // Knowledge vocabularies is observable in logs.
            let Some(subject_type) = KnowledgeSubjectType::from_drive_subject_type(
                &item.subject_type,
            ) else {
                tracing::warn!(
                    subject_id = %item.subject_id,
                    subject_type = %item.subject_type,
                    "skipping drive permission with unmapped subject type in knowledge space member list"
                );
                continue;
            };
            let Some(role) = KnowledgeAccessRole::from_drive_role(&item.role) else {
                tracing::warn!(
                    subject_id = %item.subject_id,
                    subject_type = %item.subject_type,
                    role = %item.role,
                    "skipping drive permission with unmapped role in knowledge space member list"
                );
                continue;
            };
            members.push(KnowledgeSpaceMember {
                subject_type,
                subject_id: item.subject_id,
                role,
                inherited: false,
            });
        }

        Ok(KnowledgeSpaceMemberList {
            members,
            next_cursor: list.next_page_token,
        })
    }
}
