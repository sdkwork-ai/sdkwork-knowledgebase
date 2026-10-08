use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeDriveSpaceProvisioner: Send + Sync {
    async fn create_knowledge_drive_space(
        &self,
        request: CreateKnowledgeDriveSpaceRequest,
    ) -> Result<KnowledgeDriveSpaceBinding, KnowledgeDriveSpaceProvisionerError>;

    async fn delete_knowledge_drive_space(
        &self,
        request: DeleteKnowledgeDriveSpaceRequest,
    ) -> Result<(), KnowledgeDriveSpaceProvisionerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeDriveSpaceRequest {
    pub tenant_id: String,
    pub knowledge_space_id: u64,
    pub knowledge_space_uuid: String,
    pub display_name: String,
    pub owner_subject_type: String,
    pub owner_subject_id: String,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDriveSpaceBinding {
    pub drive_space_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteKnowledgeDriveSpaceRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub owner_subject_type: String,
    pub owner_subject_id: String,
    pub operator_id: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeDriveSpaceProvisionerError {
    #[error("knowledge drive space invalid request: {0}")]
    InvalidRequest(String),
    #[error("knowledge drive space not found: {0}")]
    NotFound(String),
    #[error("knowledge drive space conflict: {0}")]
    Conflict(String),
    #[error("knowledge drive space permission denied: {0}")]
    PermissionDenied(String),
    #[error("knowledge drive space upstream error: {0}")]
    Upstream(String),
    #[error("knowledge drive space internal error: {0}")]
    Internal(String),
}
