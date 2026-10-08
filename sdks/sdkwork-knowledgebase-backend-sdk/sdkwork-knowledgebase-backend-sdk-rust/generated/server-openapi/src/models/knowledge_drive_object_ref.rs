use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeDriveObjectRef {
    pub id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "driveSpaceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_space_id: Option<String>,

    #[serde(rename = "driveNodeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_node_id: Option<String>,

    #[serde(rename = "logicalPath")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_path: Option<String>,

    #[serde(rename = "driveProviderKind")]
    pub drive_provider_kind: String,

    #[serde(rename = "driveBucket")]
    pub drive_bucket: String,

    #[serde(rename = "driveObjectKey")]
    pub drive_object_key: String,

    #[serde(rename = "driveObjectVersion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_object_version: Option<String>,

    #[serde(rename = "driveEtag")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_etag: Option<String>,

    #[serde(rename = "contentType")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    #[serde(rename = "sizeBytes")]
    pub size_bytes: String,

    #[serde(rename = "checksumSha256Hex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum_sha256_hex: Option<String>,

    #[serde(rename = "objectRole")]
    pub object_role: String,

    #[serde(rename = "accessMode")]
    pub access_mode: String,

    #[serde(rename = "driveStorageProviderId")]
    pub drive_storage_provider_id: String,
}
