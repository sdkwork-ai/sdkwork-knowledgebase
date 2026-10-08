use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeSpace {
    pub id: String,

    pub uuid: String,

    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "driveSpaceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_space_id: Option<String>,

    pub status: String,

    #[serde(rename = "okfBundleInitialized")]
    pub okf_bundle_initialized: bool,
}
