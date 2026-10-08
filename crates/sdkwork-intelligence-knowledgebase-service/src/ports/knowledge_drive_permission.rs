use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeDrivePermissionProvider: Send + Sync {
    async fn grant_space_access(
        &self,
        request: GrantDrivePermissionRequest,
    ) -> Result<DrivePermissionGrant, KnowledgeDrivePermissionError>;

    async fn revoke_space_access(
        &self,
        request: RevokeDrivePermissionRequest,
    ) -> Result<(), KnowledgeDrivePermissionError>;

    async fn list_space_permissions(
        &self,
        request: ListDrivePermissionsRequest,
    ) -> Result<DrivePermissionList, KnowledgeDrivePermissionError>;

    async fn check_space_access(
        &self,
        request: CheckDrivePermissionRequest,
    ) -> Result<DrivePermissionCheck, KnowledgeDrivePermissionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantDrivePermissionRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub role: String,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeDrivePermissionRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDrivePermissionsRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckDrivePermissionRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub required_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivePermissionGrant {
    pub id: String,
    pub node_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivePermissionCheck {
    pub allowed: bool,
    pub effective_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivePermissionList {
    pub items: Vec<DrivePermissionGrant>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeDrivePermissionError {
    #[error("drive permission invalid request: {0}")]
    InvalidRequest(String),
    #[error("drive permission conflict: {0}")]
    Conflict(String),
    #[error("drive permission not found: {0}")]
    NotFound(String),
    #[error("drive permission denied: {0}")]
    PermissionDenied(String),
    #[error("drive permission upstream error: {0}")]
    Upstream(String),
    #[error("drive permission internal error: {0}")]
    Internal(String),
}
