use serde::{Deserialize, Serialize};

/// Projected sources/raw file state and its pinned public version.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWikiSourceFile {
    pub uuid: String,

    #[serde(rename = "driveNodeUuid")]
    pub drive_node_uuid: String,

    #[serde(rename = "driveVersionUuid")]
    pub drive_version_uuid: String,

    #[serde(rename = "sourcePath")]
    pub source_path: String,

    #[serde(rename = "canonicalRoute")]
    pub canonical_route: String,

    #[serde(rename = "fileKind")]
    pub file_kind: String,

    #[serde(rename = "mediaType")]
    pub media_type: String,

    #[serde(rename = "sizeBytes")]
    pub size_bytes: String,

    #[serde(rename = "contentSha256")]
    pub content_sha256: String,

    #[serde(rename = "sourceState")]
    pub source_state: String,

    #[serde(rename = "publicationState")]
    pub publication_state: String,

    pub visibility: String,

    #[serde(rename = "indexState")]
    pub index_state: String,

    #[serde(rename = "publicDriveVersionUuid")]
    pub public_drive_version_uuid: String,

    #[serde(rename = "pagePublicVersion")]
    pub page_public_version: String,

    pub version: String,
}
