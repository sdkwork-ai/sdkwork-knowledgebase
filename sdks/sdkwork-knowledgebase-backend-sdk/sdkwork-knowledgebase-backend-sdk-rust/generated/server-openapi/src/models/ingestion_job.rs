use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IngestionJob {
    pub id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "sourceType")]
    pub source_type: String,

    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,

    pub state: String,

    #[serde(rename = "errorMessage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}
