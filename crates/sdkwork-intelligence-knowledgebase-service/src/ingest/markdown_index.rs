use crate::ingest::payload_limits::{
    split_oversized_paragraph, validate_markdown_payload, PayloadLimitError, MAX_MARKDOWN_CHUNKS,
    MAX_MARKDOWN_CHUNK_CHARS,
};
use crate::ports::knowledge_chunk_store::CreateKnowledgeChunkRecord;
use crate::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope,
};
use crate::ports::knowledge_document_version_store::CreateKnowledgeDocumentVersionRecord;
use crate::ports::knowledge_drive_object_ref_store::managed_drive_object_ref_record;
use crate::ports::knowledge_drive_storage::KnowledgeObjectRef;
use crate::ports::knowledge_ingestion_job_store::DriveImportJobLinkage;
use crate::ports::markdown_index_metadata_store::{
    MarkdownIndexMetadataStore, MarkdownIndexMetadataStoreError, MarkdownIndexSourceBinding,
    PrepareMarkdownIndexMetadataRecord,
};
use sdkwork_utils_rust::{is_blank, sha256_hash};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownIndexResult {
    pub document_version_id: u64,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedMarkdownIndex {
    pub document_version_id: u64,
    pub chunk_records: Vec<CreateKnowledgeChunkRecord>,
    pub ingest_linkage: Option<DriveImportJobLinkage>,
}

pub struct KnowledgeApiMarkdownIndexService<'a> {
    metadata: &'a dyn MarkdownIndexMetadataStore,
}

impl<'a> KnowledgeApiMarkdownIndexService<'a> {
    pub fn new(metadata: &'a dyn MarkdownIndexMetadataStore) -> Self {
        Self { metadata }
    }

    pub async fn prepare_payload_markdown_index(
        &self,
        space_id: u64,
        source: MarkdownIndexSourceBinding,
        title: &str,
        payload_markdown: &str,
        payload_object_ref: &KnowledgeObjectRef,
        drive_space_id: Option<&str>,
    ) -> Result<PreparedMarkdownIndex, KnowledgeApiMarkdownIndexServiceError> {
        if space_id == 0 {
            return Err(KnowledgeApiMarkdownIndexServiceError::InvalidRequest(
                "space_id is required".to_string(),
            ));
        }
        if is_blank(Some(title)) {
            return Err(KnowledgeApiMarkdownIndexServiceError::InvalidRequest(
                "title is required".to_string(),
            ));
        }

        let prepared_metadata = self
            .metadata
            .create_or_prepare_markdown_index_metadata(PrepareMarkdownIndexMetadataRecord {
                source,
                object_ref: managed_drive_object_ref_record(
                    space_id,
                    payload_object_ref,
                    drive_space_id,
                    None,
                ),
                document: CreateKnowledgeDocumentRecord {
                    space_id,
                    source_id: None,
                    identity_scope: KnowledgeDocumentIdentityScope::SourceOnly,
                    original_file_drive_node_id: None,
                    title: title.to_string(),
                    mime_type: Some("text/markdown".to_string()),
                    language: None,
                },
                version: CreateKnowledgeDocumentVersionRecord {
                    document_id: 0,
                    version_no: 1,
                    original_object_ref_id: 0,
                    checksum_sha256_hex: payload_object_ref.checksum_sha256_hex.clone(),
                    size_bytes: payload_object_ref.size_bytes,
                    mime_type: Some("text/markdown".to_string()),
                },
            })
            .await?;

        let chunk_records = split_markdown_chunks(
            space_id,
            prepared_metadata.document.id,
            prepared_metadata.version.id,
            payload_markdown,
        );
        Ok(PreparedMarkdownIndex {
            document_version_id: prepared_metadata.version.id,
            chunk_records,
            ingest_linkage: Some(DriveImportJobLinkage {
                source_id: prepared_metadata.source_id,
                document_id: prepared_metadata.document.id,
                document_version_id: prepared_metadata.version.id,
                original_object_ref: prepared_metadata.object_ref,
            }),
        })
    }

    pub async fn prepare_existing_document_version_index(
        &self,
        space_id: u64,
        document_id: u64,
        document_version_id: u64,
        payload_markdown: &str,
    ) -> Result<PreparedMarkdownIndex, KnowledgeApiMarkdownIndexServiceError> {
        if space_id == 0 || document_id == 0 || document_version_id == 0 {
            return Err(KnowledgeApiMarkdownIndexServiceError::InvalidRequest(
                "space_id, document_id, and document_version_id are required".to_string(),
            ));
        }

        if is_blank(Some(payload_markdown)) {
            return Err(KnowledgeApiMarkdownIndexServiceError::InvalidRequest(
                "payload content must not be empty".to_string(),
            ));
        }
        validate_markdown_payload(payload_markdown).map_err(|error| match error {
            PayloadLimitError::PayloadEmpty => {
                KnowledgeApiMarkdownIndexServiceError::InvalidRequest(
                    "payload content must not be empty".to_string(),
                )
            }
            PayloadLimitError::PayloadTooLarge { max_bytes } => {
                KnowledgeApiMarkdownIndexServiceError::InvalidRequest(format!(
                    "payload content exceeds maximum allowed size of {max_bytes} bytes"
                ))
            }
        })?;

        let chunk_records =
            split_markdown_chunks(space_id, document_id, document_version_id, payload_markdown);
        Ok(PreparedMarkdownIndex {
            document_version_id,
            chunk_records,
            ingest_linkage: None,
        })
    }
}

pub fn split_markdown_chunks(
    space_id: u64,
    document_id: u64,
    document_version_id: u64,
    markdown: &str,
) -> Vec<CreateKnowledgeChunkRecord> {
    let mut records = Vec::new();
    for (index, segment) in markdown
        .split("\n\n")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .flat_map(|segment| split_oversized_paragraph(segment, MAX_MARKDOWN_CHUNK_CHARS))
        .enumerate()
    {
        if records.len() >= MAX_MARKDOWN_CHUNKS {
            break;
        }
        let content_hash = format!("sha256:{}", sha256_hash(segment.as_bytes()));
        records.push(CreateKnowledgeChunkRecord {
            space_id,
            document_id,
            document_version_id,
            chunk_index: (index + 1) as u32,
            content_text: segment.clone(),
            content_hash,
            token_count: Some(estimate_token_count(&segment)),
            locator: Some(format!("paragraph:{}", index + 1)),
        });
    }

    records
}

#[cfg(test)]
mod split_markdown_chunk_tests {
    use super::*;
    use crate::ingest::payload_limits::MAX_MARKDOWN_CHUNKS;

    #[test]
    fn caps_chunk_count_for_large_documents() {
        let markdown = (0..MAX_MARKDOWN_CHUNKS + 10)
            .map(|index| format!("paragraph {index}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = split_markdown_chunks(1, 2, 3, &markdown);
        assert_eq!(chunks.len(), MAX_MARKDOWN_CHUNKS);
    }
}

fn estimate_token_count(content: &str) -> u32 {
    ((content.len() as u32) / 4).max(1)
}

#[derive(Debug, Error)]
pub enum KnowledgeApiMarkdownIndexServiceError {
    #[error("invalid markdown index request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Metadata(#[from] MarkdownIndexMetadataStoreError),
}
