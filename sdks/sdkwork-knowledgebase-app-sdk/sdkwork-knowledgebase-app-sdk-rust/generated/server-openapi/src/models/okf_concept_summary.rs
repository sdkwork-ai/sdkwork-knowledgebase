use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfConceptSummary {
    pub title: String,

    #[serde(rename = "conceptId")]
    pub concept_id: String,

    #[serde(rename = "conceptType")]
    pub concept_type: String,

    #[serde(rename = "logicalPath")]
    pub logical_path: String,

    pub description: String,

    #[serde(rename = "sourceCount")]
    pub source_count: i64,

    #[serde(rename = "updatedAt")]
    pub updated_at: String,

    pub tags: Vec<String>,
}
