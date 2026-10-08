use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeCitation};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeContextFragment {
    #[serde(rename = "chunkId")]
    pub chunk_id: String,

    #[serde(rename = "documentId")]
    pub document_id: String,

    #[serde(rename = "documentVersionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_version_id: Option<String>,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    pub title: String,

    pub content: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    pub rank: i64,

    #[serde(rename = "tokenCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i64>,

    #[serde(rename = "retrievalMethod")]
    pub retrieval_method: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<KnowledgeCitation>,
}
