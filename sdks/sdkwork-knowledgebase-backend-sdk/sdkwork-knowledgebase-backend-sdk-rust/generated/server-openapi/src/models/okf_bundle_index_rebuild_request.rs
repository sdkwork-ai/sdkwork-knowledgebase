use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfBundleIndexRebuildRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,
}
