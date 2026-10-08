use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfBundleFile {
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
    pub logical_path: String,
    pub file_kind: OkfBundleFileKind,
    pub artifact_role: String,
    pub drive_bucket: String,
    pub drive_object_key: String,
    pub checksum_sha256_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_import_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfBundleFileList {
    pub items: Vec<KnowledgeOkfBundleFile>,
}

pub use crate::enums::OkfBundleFileKind;
