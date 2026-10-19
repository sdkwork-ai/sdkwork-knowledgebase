use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeMemoryContextFragment {
    #[serde(rename = "memoryId")]
    pub memory_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    pub content: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    pub rank: i64,

    #[serde(rename = "tokenCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i64>,

    #[serde(rename = "sourceUri")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,

    #[serde(rename = "policyRef")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_ref: Option<String>,
}
