use async_trait::async_trait;
use sdkwork_drive_workspace_service::application::permission_service::{
    CheckDriveNodePermissionCommand, GrantDriveNodePermissionCommand,
    ListDriveNodePermissionsCommand, RevokeDriveNodePermissionCommand, SqlDrivePermissionService,
};
use sdkwork_drive_workspace_service::DriveServiceError;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_permission::{
    CheckDrivePermissionRequest, DrivePermissionCheck, DrivePermissionGrant, DrivePermissionList,
    GrantDrivePermissionRequest, KnowledgeDrivePermissionError, KnowledgeDrivePermissionProvider,
    ListDrivePermissionsRequest, RevokeDrivePermissionRequest,
};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct KnowledgebaseDrivePermissionAdapter {
    service: SqlDrivePermissionService,
}

impl KnowledgebaseDrivePermissionAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self {
            service: SqlDrivePermissionService::new(pool),
        }
    }
}

fn map_drive_error(error: DriveServiceError) -> KnowledgeDrivePermissionError {
    match error {
        DriveServiceError::Validation(message) => {
            KnowledgeDrivePermissionError::InvalidRequest(message)
        }
        DriveServiceError::Conflict(message) => KnowledgeDrivePermissionError::Conflict(message),
        DriveServiceError::NotFound(message) => KnowledgeDrivePermissionError::NotFound(message),
        DriveServiceError::PermissionDenied(message) => {
            KnowledgeDrivePermissionError::PermissionDenied(message)
        }
        DriveServiceError::Internal(message) => KnowledgeDrivePermissionError::Internal(message),
    }
}

#[async_trait]
impl KnowledgeDrivePermissionProvider for KnowledgebaseDrivePermissionAdapter {
    async fn grant_space_access(
        &self,
        request: GrantDrivePermissionRequest,
    ) -> Result<DrivePermissionGrant, KnowledgeDrivePermissionError> {
        let node_id = self
            .service
            .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
            .await
            .map_err(map_drive_error)?;

        let grant = self
            .service
            .grant_node_permission(GrantDriveNodePermissionCommand {
                tenant_id: request.tenant_id,
                node_id,
                subject_type: request.subject_type,
                subject_id: request.subject_id,
                role: request.role,
                operator_id: request.operator_id,
            })
            .await
            .map_err(map_drive_error)?;

        Ok(DrivePermissionGrant {
            id: grant.id,
            node_id: grant.node_id,
            subject_type: grant.subject_type,
            subject_id: grant.subject_id,
            role: grant.role,
        })
    }

    async fn revoke_space_access(
        &self,
        request: RevokeDrivePermissionRequest,
    ) -> Result<(), KnowledgeDrivePermissionError> {
        let node_id = self
            .service
            .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
            .await
            .map_err(map_drive_error)?;

        self.service
            .revoke_node_permission(RevokeDriveNodePermissionCommand {
                tenant_id: request.tenant_id,
                node_id,
                subject_type: request.subject_type,
                subject_id: request.subject_id,
                operator_id: request.operator_id,
            })
            .await
            .map_err(map_drive_error)
    }

    async fn list_space_permissions(
        &self,
        request: ListDrivePermissionsRequest,
    ) -> Result<DrivePermissionList, KnowledgeDrivePermissionError> {
        let node_id = self
            .service
            .resolve_space_permission_anchor_node(&request.tenant_id, &request.drive_space_id)
            .await
            .map_err(map_drive_error)?;

        let list = self
            .service
            .list_node_permissions(ListDriveNodePermissionsCommand {
                tenant_id: request.tenant_id,
                node_id,
                page_size: request.page_size,
                page_token: request.page_token,
            })
            .await
            .map_err(map_drive_error)?;

        Ok(DrivePermissionList {
            items: list
                .items
                .into_iter()
                .map(|item| DrivePermissionGrant {
                    id: item.id,
                    node_id: item.node_id,
                    subject_type: item.subject_type,
                    subject_id: item.subject_id,
                    role: item.role,
                })
                .collect(),
            next_page_token: list.next_page_token,
        })
    }

    async fn check_space_access(
        &self,
        request: CheckDrivePermissionRequest,
    ) -> Result<DrivePermissionCheck, KnowledgeDrivePermissionError> {
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
                subject_type: request.subject_type,
                subject_id: request.subject_id,
                required_role: request.required_role,
            })
            .await
            .map_err(map_drive_error)?;

        Ok(DrivePermissionCheck {
            allowed: check.allowed,
            effective_role: check.effective_role,
        })
    }
}
