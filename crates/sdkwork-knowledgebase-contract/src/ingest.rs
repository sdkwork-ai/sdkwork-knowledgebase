use crate::document::{KnowledgeDocument, KnowledgeDocumentVersion};
use crate::drive::KnowledgeDriveObjectRef;
use crate::serde_int64::{deserialize_u64_from_string_or_number, serialize_u64_as_string};
use crate::source::KnowledgeSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIngestionJobRequest {
    pub space_id: u64,
    pub source_type: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeIngestRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub title: String,
    #[serde(default)]
    pub payload_markdown: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDriveImportRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub title: String,
    pub drive_space_id: String,
    pub drive_node_id: String,
    pub idempotency_key: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDriveImportResult {
    pub source: KnowledgeSource,
    pub document: KnowledgeDocument,
    pub version: KnowledgeDocumentVersion,
    pub original_object_ref: KnowledgeDriveObjectRef,
    pub job: IngestionJob,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionJob {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub source_type: String,
    pub idempotency_key: String,
    pub state: IngestionJobState,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionJobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
