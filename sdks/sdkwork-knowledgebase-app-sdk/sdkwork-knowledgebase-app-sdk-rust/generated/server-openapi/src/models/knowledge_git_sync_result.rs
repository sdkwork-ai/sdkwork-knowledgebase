use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeGitSyncResult {
    pub accepted: bool,

    pub status: String,

    pub hash: String,

    #[serde(rename = "syncedCount")]
    pub synced_count: i64,
}
