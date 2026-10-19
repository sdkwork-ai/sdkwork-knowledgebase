use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfQualityRunRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}
