use serde::{Deserialize, Serialize};

/// Canonical Wiki publication state for one Knowledgebase.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWikiPublication {
    pub uuid: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "driveSpaceUuid")]
    pub drive_space_uuid: String,

    #[serde(rename = "sourceRootNodeUuid")]
    pub source_root_node_uuid: String,

    pub status: String,

    pub title: String,

    #[serde(rename = "homepageSourcePath")]
    pub homepage_source_path: String,

    #[serde(rename = "publicationMode")]
    pub publication_mode: String,

    #[serde(rename = "defaultVisibility")]
    pub default_visibility: String,

    #[serde(rename = "updatePolicy")]
    pub update_policy: String,

    #[serde(rename = "providerGeneration")]
    pub provider_generation: String,

    #[serde(rename = "navigationGeneration")]
    pub navigation_generation: String,

    #[serde(rename = "searchGeneration")]
    pub search_generation: String,

    #[serde(rename = "lastProjectedDriveCheckpoint")]
    pub last_projected_drive_checkpoint: String,

    pub version: String,
}
