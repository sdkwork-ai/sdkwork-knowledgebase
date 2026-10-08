use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDriveObjectRef {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub drive_space_id: Option<String>,
    pub drive_node_id: Option<String>,
    pub logical_path: Option<String>,
    #[serde(skip_serializing)]
    pub drive_provider_kind: String,
    #[serde(skip_serializing)]
    pub drive_storage_provider_id: String,
    #[serde(skip_serializing)]
    pub drive_bucket: String,
    #[serde(skip_serializing)]
    pub drive_object_key: String,
    #[serde(skip_serializing)]
    pub drive_object_version: Option<String>,
    #[serde(skip_serializing)]
    pub drive_etag: Option<String>,
    pub content_type: Option<String>,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub size_bytes: u64,
    pub checksum_sha256_hex: Option<String>,
    pub object_role: String,
    pub access_mode: String,
}
