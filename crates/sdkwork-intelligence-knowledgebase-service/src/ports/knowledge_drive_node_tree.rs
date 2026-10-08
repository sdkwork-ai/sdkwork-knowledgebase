use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeDriveNodeTree: Send + Sync {
    async fn resolve_path(
        &self,
        request: ResolveKnowledgeDriveNodePathRequest,
    ) -> Result<Option<KnowledgeDriveNodeSummary>, KnowledgeDriveNodeTreeError>;

    async fn get_node(
        &self,
        request: GetKnowledgeDriveNodeRequest,
    ) -> Result<Option<KnowledgeDriveNodeSummary>, KnowledgeDriveNodeTreeError>;

    async fn list_children(
        &self,
        request: ListKnowledgeDriveNodeChildrenRequest,
    ) -> Result<KnowledgeDriveNodePage, KnowledgeDriveNodeTreeError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveKnowledgeDriveNodePathRequest {
    pub drive_space_id: String,
    pub logical_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetKnowledgeDriveNodeRequest {
    pub drive_space_id: String,
    pub drive_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListKnowledgeDriveNodeChildrenRequest {
    pub drive_space_id: String,
    pub parent_drive_node_id: Option<String>,
    pub cursor: Option<String>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDriveNodePage {
    pub nodes: Vec<KnowledgeDriveNodeSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDriveNodeObjectLocator {
    pub storage_provider_id: String,
    pub bucket: String,
    pub object_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDriveNodeSummary {
    pub drive_node_id: String,
    pub parent_drive_node_id: Option<String>,
    pub kind: DriveNodeKind,
    pub name: String,
    pub path: String,
    pub content_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub children_count: Option<u64>,
    pub updated_at: String,
    pub object_locator: Option<KnowledgeDriveNodeObjectLocator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveNodeKind {
    Folder,
    File,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeDriveNodeTreeError {
    #[error("knowledge drive node tree invalid request: {0}")]
    InvalidRequest(String),
    #[error("knowledge drive node tree not found: {0}")]
    NotFound(String),
    #[error("knowledge drive node tree conflict: {0}")]
    Conflict(String),
    #[error("knowledge drive node tree permission denied: {0}")]
    PermissionDenied(String),
    #[error("knowledge drive node tree upstream error: {0}")]
    Upstream(String),
    #[error("knowledge drive node tree internal error: {0}")]
    Internal(String),
}
