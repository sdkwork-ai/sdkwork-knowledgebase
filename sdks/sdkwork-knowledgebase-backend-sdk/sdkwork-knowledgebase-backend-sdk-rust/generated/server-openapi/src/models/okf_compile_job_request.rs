use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfCompileJobRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "sourceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<i64>,
}
