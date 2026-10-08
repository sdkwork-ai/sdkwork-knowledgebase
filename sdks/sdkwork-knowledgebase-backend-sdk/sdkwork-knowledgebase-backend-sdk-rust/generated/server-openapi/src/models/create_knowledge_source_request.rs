use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CreateKnowledgeSourceRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "sourceType")]
    pub source_type: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    #[serde(rename = "driveBucket")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_bucket: Option<String>,

    #[serde(rename = "drivePrefix")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_prefix: Option<String>,

    /// JSON connector config for external knowledge engines (for example Dify datasetId).
    #[serde(rename = "connectorMetadataJson")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector_metadata_json: Option<String>,
}
