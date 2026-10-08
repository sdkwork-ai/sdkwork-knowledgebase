use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeOkfProfileRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "profileVersion")]
    pub profile_version: String,
}
