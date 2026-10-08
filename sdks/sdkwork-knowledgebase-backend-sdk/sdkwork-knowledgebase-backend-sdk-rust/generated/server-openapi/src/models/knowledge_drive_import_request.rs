use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeDriveImportRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    pub title: String,

    #[serde(rename = "driveBucket")]
    pub drive_bucket: String,

    #[serde(rename = "driveObjectKey")]
    pub drive_object_key: String,

    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(rename = "driveSpaceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_space_id: Option<String>,

    #[serde(rename = "driveNodeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_node_id: Option<String>,

    #[serde(rename = "driveStorageProviderId")]
    pub drive_storage_provider_id: String,
}
