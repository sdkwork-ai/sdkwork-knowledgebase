use crate::ports::knowledge_drive_storage::KnowledgeObjectRef;
use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeDriveWorkspace: Send + Sync {
    async fn ensure_nodes(
        &self,
        request: EnsureKnowledgeDriveNodesRequest,
    ) -> Result<(), KnowledgeDriveWorkspaceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureKnowledgeDriveNodesRequest {
    pub drive_space_id: String,
    pub nodes: Vec<EnsureKnowledgeDriveNodeRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureKnowledgeDriveNodeRequest {
    pub logical_path: String,
    pub kind: EnsureKnowledgeDriveNodeKind,
    pub object_ref: Option<KnowledgeObjectRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureKnowledgeDriveNodeKind {
    Folder,
    File,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeDriveWorkspaceError {
    #[error("knowledge drive workspace invalid request: {0}")]
    InvalidRequest(String),
    #[error("knowledge drive workspace not found: {0}")]
    NotFound(String),
    #[error("knowledge drive workspace conflict: {0}")]
    Conflict(String),
    #[error("knowledge drive workspace permission denied: {0}")]
    PermissionDenied(String),
    #[error("knowledge drive workspace upstream error: {0}")]
    Upstream(String),
    #[error("knowledge drive workspace internal error: {0}")]
    Internal(String),
}
