use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeDocument {
    pub id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "sourceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<i64>,

    #[serde(rename = "originalFileDriveNodeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_file_drive_node_id: Option<String>,

    pub title: String,

    #[serde(rename = "mimeType")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(rename = "currentVersionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<i64>,

    pub visibility: String,

    #[serde(rename = "contentState")]
    pub content_state: String,

    #[serde(rename = "indexState")]
    pub index_state: String,
}
