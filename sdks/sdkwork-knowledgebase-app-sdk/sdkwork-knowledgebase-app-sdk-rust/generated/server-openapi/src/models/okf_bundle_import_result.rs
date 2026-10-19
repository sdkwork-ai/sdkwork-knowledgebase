use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfBundleImportResult {
    #[serde(rename = "importedConceptCount")]
    pub imported_concept_count: i64,

    #[serde(rename = "skippedFiles")]
    pub skipped_files: Vec<String>,
}
