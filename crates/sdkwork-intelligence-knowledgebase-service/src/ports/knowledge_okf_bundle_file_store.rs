use async_trait::async_trait;
use sdkwork_knowledgebase_contract::okf_bundle_file::{KnowledgeOkfBundleFile, OkfBundleFileKind};
use thiserror::Error;

#[async_trait]
pub trait KnowledgeOkfBundleFileStore: Send + Sync {
    async fn create_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError>;

    async fn upsert_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        self.create_file_entry(record).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateKnowledgeOkfBundleFileRecord {
    pub space_id: u64,
    pub logical_path: String,
    pub file_kind: OkfBundleFileKind,
    pub artifact_role: String,
    pub drive_bucket: String,
    pub drive_object_key: String,
    pub checksum_sha256_hex: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeOkfBundleFileStoreError {
    #[error("okf bundle file store internal error: {0}")]
    Internal(String),
    #[error("okf bundle file not found: {0}")]
    NotFound(u64),
}
