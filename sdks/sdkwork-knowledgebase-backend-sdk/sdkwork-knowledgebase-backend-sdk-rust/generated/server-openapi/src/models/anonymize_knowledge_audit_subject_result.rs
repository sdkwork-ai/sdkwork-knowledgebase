use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AnonymizeKnowledgeAuditSubjectResult {
    #[serde(rename = "anonymizedCount")]
    pub anonymized_count: String,
}
