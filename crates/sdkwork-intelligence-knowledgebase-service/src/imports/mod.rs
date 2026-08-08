mod drive_import_pipeline;
mod git_import;
mod git_sync;
mod github_api;

pub use github_api::GitHubApiError;

pub use drive_import_pipeline::{
    DriveImportPipelineResult, KnowledgeDriveImportPipelineService,
    KnowledgeDriveImportPipelineServiceError,
};
pub use git_import::{
    KnowledgeGitImportRunResult, KnowledgeGitImportService, KnowledgeGitImportServiceError,
};
pub use git_sync::{
    KnowledgeDocumentMarkdownReader, KnowledgeGitSyncService, KnowledgeGitSyncServiceError,
};

use crate::ports::{
    drive_import_metadata_store::{
        DriveImportMetadataStore, DriveImportMetadataStoreError, PrepareDriveImportMetadataRecord,
    },
    knowledge_document_store::{CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope},
    knowledge_document_version_store::CreateKnowledgeDocumentVersionRecord,
    knowledge_drive_object_ref_store::managed_drive_object_ref_record,
    knowledge_drive_storage::{
        HeadKnowledgeObjectRequest, KnowledgeDriveStorage, KnowledgeStorageError,
    },
    knowledge_ingestion_job_store::CreateIngestionJobRecord,
    knowledge_source_store::CreateKnowledgeSourceRecord,
};
use sdkwork_knowledgebase_contract::{
    ingest::{KnowledgeDriveImportRequest, KnowledgeDriveImportResult},
    source::KnowledgeSourceType,
};
use sdkwork_utils_rust::is_blank;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub struct KnowledgeDriveImportService<'a> {
    drive: &'a dyn KnowledgeDriveStorage,
    metadata: &'a dyn DriveImportMetadataStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKnowledgeDriveImportRequest {
    pub request: KnowledgeDriveImportRequest,
    pub drive_storage_provider_id: String,
    pub drive_bucket: String,
    pub drive_object_key: String,
}

impl<'a> KnowledgeDriveImportService<'a> {
    pub fn new(
        drive: &'a dyn KnowledgeDriveStorage,
        metadata: &'a dyn DriveImportMetadataStore,
    ) -> Self {
        Self { drive, metadata }
    }

    pub async fn import_drive_object(
        &self,
        resolved: ResolvedKnowledgeDriveImportRequest,
    ) -> Result<KnowledgeDriveImportResult, KnowledgeDriveImportServiceError> {
        let request = resolved.request;
        if request.space_id == 0 {
            return Err(KnowledgeDriveImportServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        if is_blank(Some(request.title.as_str())) {
            return Err(KnowledgeDriveImportServiceError::InvalidRequest(
                "title is required".to_string(),
            ));
        }
        if is_blank(Some(resolved.drive_bucket.as_str())) {
            return Err(KnowledgeDriveImportServiceError::InvalidRequest(
                "drive_bucket is required".to_string(),
            ));
        }
        if is_blank(Some(resolved.drive_storage_provider_id.as_str())) {
            return Err(KnowledgeDriveImportServiceError::InvalidRequest(
                "drive_storage_provider_id is required".to_string(),
            ));
        }
        if is_blank(Some(resolved.drive_object_key.as_str())) {
            return Err(KnowledgeDriveImportServiceError::InvalidRequest(
                "drive_object_key is required".to_string(),
            ));
        }
        let idempotency_key = normalize_idempotency_key(&request.idempotency_key)?;
        let drive_space_id = normalize_required_drive_id(request.drive_space_id, "drive_space_id")?;
        let drive_node_id = normalize_required_drive_id(request.drive_node_id, "drive_node_id")?;
        let drive_storage_provider_id = normalize_required_drive_id(
            resolved.drive_storage_provider_id,
            "drive_storage_provider_id",
        )?;

        let fingerprint_input = DriveImportFingerprintInput {
            space_id: request.space_id,
            title: &request.title,
            drive_space_id: Some(&drive_space_id),
            drive_node_id: Some(&drive_node_id),
            drive_storage_provider_id: &drive_storage_provider_id,
            drive_bucket: &resolved.drive_bucket,
            drive_object_key: &resolved.drive_object_key,
            language: request.language.as_deref(),
        };
        let fingerprint = drive_import_idempotency_fingerprint_sha256_hex(&fingerprint_input);
        self.metadata
            .validate_drive_import_idempotency(CreateIngestionJobRecord {
                space_id: request.space_id,
                source_type: KnowledgeSourceType::DriveObject.as_str().to_string(),
                idempotency_key: idempotency_key.clone(),
                idempotency_fingerprint_sha256_hex: Some(fingerprint.clone()),
            })
            .await?;

        let original_object_ref = self
            .drive
            .head_object(HeadKnowledgeObjectRequest::original_document(
                drive_storage_provider_id.clone(),
                resolved.drive_bucket.clone(),
                resolved.drive_object_key.clone(),
            ))
            .await?;
        if original_object_ref.storage_provider_id != drive_storage_provider_id {
            return Err(KnowledgeDriveImportServiceError::InvalidRequest(format!(
                "drive_storage_provider_id does not match resolved object provider: {}",
                original_object_ref.storage_provider_id
            )));
        }

        let prepared = self
            .metadata
            .create_or_prepare_drive_import_metadata(PrepareDriveImportMetadataRecord {
                job: CreateIngestionJobRecord {
                    space_id: request.space_id,
                    source_type: KnowledgeSourceType::DriveObject.as_str().to_string(),
                    idempotency_key,
                    idempotency_fingerprint_sha256_hex: Some(fingerprint),
                },
                object_ref: managed_drive_object_ref_record(
                    request.space_id,
                    &original_object_ref,
                    Some(&drive_space_id),
                    Some(drive_node_id.clone()),
                ),
                source: CreateKnowledgeSourceRecord {
                    space_id: request.space_id,
                    source_type: KnowledgeSourceType::DriveObject,
                    provider: Some("sdkwork-drive".to_string()),
                    drive_bucket: Some(resolved.drive_bucket),
                    drive_prefix: Some(resolved.drive_object_key),
                    connector_metadata_json: None,
                },
                document: CreateKnowledgeDocumentRecord {
                    space_id: request.space_id,
                    collection_id: 0,
                    source_id: None,
                    identity_scope: KnowledgeDocumentIdentityScope::SourceOnly,
                    original_file_drive_node_id: Some(drive_node_id),
                    title: request.title,
                    mime_type: Some(original_object_ref.content_type.clone()),
                    language: request.language,
                },
                version: CreateKnowledgeDocumentVersionRecord {
                    document_id: 0,
                    version_no: 1,
                    original_object_ref_id: 0,
                    checksum_sha256_hex: original_object_ref.checksum_sha256_hex.clone(),
                    size_bytes: original_object_ref.size_bytes,
                    mime_type: Some(original_object_ref.content_type.clone()),
                },
            })
            .await?;

        Ok(KnowledgeDriveImportResult {
            source: prepared.source,
            document: prepared.document,
            version: prepared.version,
            original_object_ref: prepared.original_object_ref,
            job: prepared.job,
        })
    }
}

struct DriveImportFingerprintInput<'a> {
    space_id: u64,
    title: &'a str,
    drive_space_id: Option<&'a str>,
    drive_node_id: Option<&'a str>,
    drive_storage_provider_id: &'a str,
    drive_bucket: &'a str,
    drive_object_key: &'a str,
    language: Option<&'a str>,
}

fn drive_import_idempotency_fingerprint_sha256_hex(
    input: &DriveImportFingerprintInput<'_>,
) -> String {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, "kind", Some("drive_object"));
    hash_field(&mut hasher, "space_id", Some(&input.space_id.to_string()));
    hash_field(&mut hasher, "title", Some(input.title));
    hash_field(&mut hasher, "drive_space_id", input.drive_space_id);
    hash_field(&mut hasher, "drive_node_id", input.drive_node_id);
    hash_field(
        &mut hasher,
        "drive_storage_provider_id",
        Some(input.drive_storage_provider_id),
    );
    hash_field(&mut hasher, "drive_bucket", Some(input.drive_bucket));
    hash_field(
        &mut hasher,
        "drive_object_key",
        Some(input.drive_object_key),
    );
    hash_field(&mut hasher, "language", input.language);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn hash_field(hasher: &mut Sha256, field_name: &str, value: Option<&str>) {
    hasher.update(field_name.as_bytes());
    hasher.update([0]);
    match value {
        Some(value) => {
            hasher.update(value.len().to_string().as_bytes());
            hasher.update(b":");
            hasher.update(value.as_bytes());
        }
        None => hasher.update(b"null"),
    }
    hasher.update([0xff]);
}

fn is_safe_idempotency_key(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn normalize_idempotency_key(value: &str) -> Result<String, KnowledgeDriveImportServiceError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(KnowledgeDriveImportServiceError::InvalidRequest(
            "idempotency_key is required".to_string(),
        ));
    }
    if !is_safe_idempotency_key(value) {
        return Err(KnowledgeDriveImportServiceError::InvalidRequest(
            "idempotency_key contains unsafe characters".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn normalize_optional_drive_id(
    value: Option<String>,
    field_name: &str,
) -> Result<Option<String>, KnowledgeDriveImportServiceError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    {
        return Err(KnowledgeDriveImportServiceError::InvalidRequest(format!(
            "{field_name} contains unsafe characters"
        )));
    }
    Ok(Some(value))
}

fn normalize_required_drive_id(
    value: String,
    field_name: &str,
) -> Result<String, KnowledgeDriveImportServiceError> {
    normalize_optional_drive_id(Some(value), field_name)?.ok_or_else(|| {
        KnowledgeDriveImportServiceError::InvalidRequest(format!("{field_name} is required"))
    })
}

#[derive(Debug, Error)]
pub enum KnowledgeDriveImportServiceError {
    #[error("invalid drive import request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Storage(#[from] KnowledgeStorageError),
    #[error(transparent)]
    Metadata(#[from] DriveImportMetadataStoreError),
}
