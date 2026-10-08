use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeIngestRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    pub title: String,

    #[serde(rename = "payloadMarkdown")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_markdown: Option<String>,

    #[serde(rename = "sourceUrl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,

    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,
}
