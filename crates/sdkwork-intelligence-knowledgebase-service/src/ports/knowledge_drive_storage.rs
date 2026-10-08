use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_MAX_KNOWLEDGE_OBJECT_READ_BYTES: u64 = 32 * 1024 * 1024;

#[async_trait]
pub trait KnowledgeDriveStorage: Send + Sync {
    async fn put_object(
        &self,
        request: PutKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError>;

    async fn head_object(
        &self,
        request: HeadKnowledgeObjectRequest,
    ) -> Result<KnowledgeObjectRef, KnowledgeStorageError>;

    async fn delete_object(
        &self,
        _object_ref: &KnowledgeObjectRef,
    ) -> Result<(), KnowledgeStorageError> {
        Err(KnowledgeStorageError::Internal(
            "knowledge storage does not support object deletion".to_string(),
        ))
    }

    async fn get_object_text(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<String, KnowledgeStorageError>;

    async fn get_object_text_bounded(
        &self,
        object_ref: &KnowledgeObjectRef,
        max_bytes: u64,
    ) -> Result<String, KnowledgeStorageError> {
        let bytes = self.get_object_bytes_bounded(object_ref, max_bytes).await?;
        String::from_utf8(bytes)
            .map_err(|error| KnowledgeStorageError::InvalidRequest(error.to_string()))
    }

    async fn get_object_bytes(
        &self,
        object_ref: &KnowledgeObjectRef,
    ) -> Result<Vec<u8>, KnowledgeStorageError> {
        self.get_object_text(object_ref)
            .await
            .map(|text| text.into_bytes())
    }

    async fn get_object_bytes_bounded(
        &self,
        object_ref: &KnowledgeObjectRef,
        max_bytes: u64,
    ) -> Result<Vec<u8>, KnowledgeStorageError> {
        validate_object_read_size(object_ref, max_bytes)?;
        let bytes = self.get_object_bytes(object_ref).await?;
        if bytes.len() as u64 > max_bytes {
            return Err(object_read_limit_error(bytes.len() as u64, max_bytes));
        }
        Ok(bytes)
    }
}

pub fn validate_object_read_size(
    object_ref: &KnowledgeObjectRef,
    max_bytes: u64,
) -> Result<(), KnowledgeStorageError> {
    if max_bytes == 0 {
        return Err(KnowledgeStorageError::InvalidRequest(
            "knowledge object read limit must be greater than zero".to_string(),
        ));
    }
    if object_ref.size_bytes > max_bytes {
        return Err(object_read_limit_error(object_ref.size_bytes, max_bytes));
    }
    Ok(())
}

pub fn object_read_limit_error(actual_bytes: u64, max_bytes: u64) -> KnowledgeStorageError {
    KnowledgeStorageError::InvalidRequest(format!(
        "knowledge object size {actual_bytes} exceeds read limit {max_bytes} bytes"
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutKnowledgeObjectRequest {
    pub logical_path: String,
    pub object_role: String,
    pub content_type: String,
    pub body: Vec<u8>,
    pub checksum_sha256_hex: Option<String>,
    /// Knowledge space UUID for per-space object key planning (`knowledge/{tenant}/{space}/...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_uuid: Option<String>,
}

impl PutKnowledgeObjectRequest {
    pub fn text(
        logical_path: impl Into<String>,
        object_role: impl Into<String>,
        body: impl Into<String>,
        checksum_sha256_hex: Option<String>,
    ) -> Self {
        Self {
            logical_path: logical_path.into(),
            object_role: object_role.into(),
            content_type: "text/markdown; charset=utf-8".to_string(),
            body: body.into().into_bytes(),
            checksum_sha256_hex,
            space_uuid: None,
        }
    }

    pub fn with_space_uuid(mut self, space_uuid: impl Into<String>) -> Self {
        self.space_uuid = Some(space_uuid.into());
        self
    }

    pub fn with_drive_space_id(mut self, drive_space_id: Option<&str>) -> Self {
        self.space_uuid = drive_space_id.and_then(space_uuid_from_drive_space_id);
        self
    }

    pub fn managed_text(
        logical_path: impl Into<String>,
        object_role: impl Into<String>,
        body: impl Into<String>,
        drive_space_id: Option<&str>,
    ) -> Self {
        Self::text(logical_path, object_role, body, None).with_drive_space_id(drive_space_id)
    }

    pub fn managed_json(
        logical_path: impl Into<String>,
        object_role: impl Into<String>,
        body: Vec<u8>,
        checksum_sha256_hex: impl Into<String>,
        space_uuid: impl Into<String>,
    ) -> Self {
        Self {
            logical_path: logical_path.into(),
            object_role: object_role.into(),
            content_type: "application/json; charset=utf-8".to_string(),
            body,
            checksum_sha256_hex: Some(checksum_sha256_hex.into()),
            space_uuid: Some(space_uuid.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadKnowledgeObjectRequest {
    pub storage_provider_id: Option<String>,
    pub bucket: String,
    pub object_key: String,
    pub logical_path: Option<String>,
    pub object_role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_uuid: Option<String>,
}

impl HeadKnowledgeObjectRequest {
    pub fn original_document(
        storage_provider_id: impl Into<String>,
        bucket: impl Into<String>,
        object_key: impl Into<String>,
    ) -> Self {
        let object_key = object_key.into();
        Self {
            storage_provider_id: Some(storage_provider_id.into()),
            bucket: bucket.into(),
            logical_path: Some(object_key.clone()),
            object_key,
            object_role: "original_document".to_string(),
            space_uuid: None,
        }
    }

    pub fn managed_artifact(
        logical_path: impl Into<String>,
        object_role: impl Into<String>,
    ) -> Self {
        let logical_path = logical_path.into();
        Self {
            storage_provider_id: None,
            bucket: String::new(),
            object_key: logical_path.clone(),
            logical_path: Some(logical_path),
            object_role: object_role.into(),
            space_uuid: None,
        }
    }

    pub fn with_space_uuid(mut self, space_uuid: impl Into<String>) -> Self {
        self.space_uuid = Some(space_uuid.into());
        self
    }

    pub fn with_drive_space_id(mut self, drive_space_id: Option<&str>) -> Self {
        self.space_uuid = drive_space_id.and_then(space_uuid_from_drive_space_id);
        self
    }
}

pub fn space_uuid_from_drive_space_id(drive_space_id: &str) -> Option<String> {
    drive_space_id
        .strip_prefix("kb-")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeObjectRef {
    pub storage_provider_id: String,
    pub bucket: String,
    pub object_key: String,
    pub logical_path: String,
    pub object_role: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub checksum_sha256_hex: Option<String>,
    pub etag: Option<String>,
    pub version_id: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeStorageError {
    #[error("knowledge storage not found: {0}")]
    NotFound(String),
    #[error("knowledge storage invalid request: {0}")]
    InvalidRequest(String),
    #[error("knowledge storage integrity failed: {0}")]
    IntegrityFailed(String),
    #[error("knowledge storage permission denied: {0}")]
    PermissionDenied(String),
    #[error("knowledge storage conflict: {0}")]
    Conflict(String),
    #[error("knowledge storage upstream error: {0}")]
    Upstream(String),
    #[error("knowledge storage internal error: {0}")]
    Internal(String),
}

impl KnowledgeStorageError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }
}
