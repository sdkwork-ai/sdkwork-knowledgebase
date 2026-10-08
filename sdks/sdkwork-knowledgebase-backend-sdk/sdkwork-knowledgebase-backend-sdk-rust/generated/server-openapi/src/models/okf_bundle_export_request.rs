use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfBundleExportRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "exportType")]
    pub export_type: String,

    #[serde(rename = "stageForImport")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_for_import: Option<bool>,

    #[serde(rename = "importId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,
}
