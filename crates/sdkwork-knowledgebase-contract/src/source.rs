use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeSourceRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub source_type: KnowledgeSourceType,
    pub provider: Option<String>,
    pub drive_bucket: Option<String>,
    pub drive_prefix: Option<String>,
    /// Non-authoritative content-association metadata. Provider selection, credentials, and
    /// remote resources are owned exclusively by the active Provider binding.
    pub connector_metadata_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSourceList {
    pub items: Vec<KnowledgeSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSource {
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
    pub source_type: KnowledgeSourceType,
    pub provider: Option<String>,
    pub drive_bucket: Option<String>,
    pub drive_prefix: Option<String>,
    /// Non-authoritative content-association metadata. Provider selection, credentials, and
    /// remote resources are owned exclusively by the active Provider binding.
    pub connector_metadata_json: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSourceType {
    Upload,
    DriveObject,
    DriveFolder,
    Url,
    Connector,
    Api,
}

impl KnowledgeSourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::DriveObject => "drive_object",
            Self::DriveFolder => "drive_folder",
            Self::Url => "url",
            Self::Connector => "connector",
            Self::Api => "api",
        }
    }
}
