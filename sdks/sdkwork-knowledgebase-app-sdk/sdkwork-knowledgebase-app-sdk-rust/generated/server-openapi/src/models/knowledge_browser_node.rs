use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeBrowserNodePermissions};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeBrowserNode {
    pub id: String,

    #[serde(rename = "nodeType")]
    pub node_type: String,

    pub name: String,

    #[serde(rename = "parentId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    pub path: String,

    #[serde(rename = "driveSpaceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_space_id: Option<String>,

    #[serde(rename = "driveNodeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_node_id: Option<String>,

    #[serde(rename = "documentId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    #[serde(rename = "documentVersionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_version_id: Option<String>,

    #[serde(rename = "conceptId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_id: Option<String>,

    #[serde(rename = "conceptRevisionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_revision_id: Option<String>,

    #[serde(rename = "mimeType")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(rename = "sizeBytes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<String>,

    #[serde(rename = "ingestState")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest_state: Option<String>,

    #[serde(rename = "parseState")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_state: Option<String>,

    #[serde(rename = "indexState")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_state: Option<String>,

    #[serde(rename = "okfState")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub okf_state: Option<String>,

    #[serde(rename = "childrenCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children_count: Option<String>,

    #[serde(rename = "updatedAt")]
    pub updated_at: String,

    #[serde(rename = "driveStorageProviderId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_storage_provider_id: Option<String>,

    #[serde(rename = "driveBucket")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_bucket: Option<String>,

    #[serde(rename = "driveObjectKey")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_object_key: Option<String>,

    pub permissions: KnowledgeBrowserNodePermissions,
}
