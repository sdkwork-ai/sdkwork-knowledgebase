use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeIndexRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "indexKind")]
    pub index_kind: String,

    #[serde(rename = "embeddingProviderId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_provider_id: Option<String>,

    #[serde(rename = "embeddingModel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
}
