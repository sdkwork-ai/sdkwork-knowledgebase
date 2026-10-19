use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfBundleImportRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "importType")]
    pub import_type: String,

    #[serde(rename = "importId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,
}
