use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeOkfBundleFile {
    pub id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "logicalPath")]
    pub logical_path: String,

    #[serde(rename = "entryType")]
    pub entry_type: String,

    #[serde(rename = "artifactRole")]
    pub artifact_role: String,

    #[serde(rename = "driveBucket")]
    pub drive_bucket: String,

    #[serde(rename = "driveObjectKey")]
    pub drive_object_key: String,

    #[serde(rename = "checksumSha256Hex")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum_sha256_hex: Option<String>,

    #[serde(rename = "stagedImportRoot")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_import_root: Option<String>,

    #[serde(rename = "importId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,
}
