use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeCitation {
    #[serde(rename = "documentId")]
    pub document_id: String,

    #[serde(rename = "documentVersionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_version_id: Option<String>,

    #[serde(rename = "chunkId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,

    pub title: String,

    #[serde(rename = "sourceUri")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}
