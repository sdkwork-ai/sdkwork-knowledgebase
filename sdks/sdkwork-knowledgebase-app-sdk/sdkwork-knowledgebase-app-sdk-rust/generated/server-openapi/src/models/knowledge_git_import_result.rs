use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeGitImportResult {
    #[serde(rename = "importedCount")]
    pub imported_count: i64,

    #[serde(rename = "skippedCount")]
    pub skipped_count: i64,
}
