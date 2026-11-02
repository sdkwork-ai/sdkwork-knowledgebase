use async_trait::async_trait;
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::KnowledgeSpace;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeSpaceStore: Send + Sync {
    async fn create_space(
        &self,
        record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError>;

    async fn get_space(&self, space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError>;

    /// Returns a group-managed active space only after the caller has performed group snapshot
    /// authorization. This must not be used by generic knowledge-space routes.
    async fn get_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(format!(
            "group-managed space access is unsupported for space {space_id}"
        )))
    }

    /// Returns a group-managed provisioning space for trusted provisioning orchestration only.
    async fn get_group_provisioning_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(format!(
            "group-managed provisioning access is unsupported for space {space_id}"
        )))
    }

    async fn mark_drive_space_bound(
        &self,
        space_id: u64,
        record: BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError>;

    async fn mark_okf_bundle_initialized(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError>;

    /// Makes a successfully initialized group-managed space visible to its specialized access
    /// path. Generic routes still exclude group-managed spaces.
    async fn activate_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(format!(
            "group-managed activation is unsupported for space {space_id}"
        )))
    }

    /// Idempotently archives a group-managed space for the durable group archive saga. An
    /// already archived space is a successful convergence result; content is retained.
    async fn archive_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(format!(
            "group-managed archive is unsupported for space {space_id}"
        )))
    }

    async fn update_space(
        &self,
        space_id: u64,
        record: UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError>;

    /// Updates only Knowledgebase-owned description metadata for an active group-managed space.
    /// The caller must first establish the current IM owner snapshot and projected Drive owner
    /// grant. Generic space updates intentionally remain unavailable for group-managed spaces.
    async fn update_group_managed_space_description(
        &self,
        space_id: u64,
        _description: String,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        Err(KnowledgeSpaceStoreError::Internal(format!(
            "group-managed description update is unsupported for space {space_id}"
        )))
    }

    async fn mark_space_deleted(&self, space_id: u64) -> Result<(), KnowledgeSpaceStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeSpaceRecord {
    pub name: String,
    pub description: Option<String>,
    pub okf_bundle_initialized: bool,
    pub knowledge_mode: KnowledgeAgentKnowledgeMode,
    /// Authenticated actor that created the space, or `0` when the creating path
    /// has no authenticated actor (for example a trusted provisioning reservation).
    /// Persisted so the creator keeps object-level access without depending on the
    /// separately projected Drive ACL.
    pub created_by: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateKnowledgeSpaceRecord {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindKnowledgeDriveSpaceRecord {
    pub drive_space_id: String,
    pub actor_id: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeSpaceStoreError {
    #[error("knowledge space store conflict: {0}")]
    Conflict(String),
    #[error("knowledge space was not found: {0}")]
    NotFound(String),
    #[error("knowledge space store internal error: {0}")]
    Internal(String),
}
