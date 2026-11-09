use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CreateKnowledgeAuditLegalHoldRequest {
    #[serde(rename = "subjectType")]
    pub subject_type: String,

    #[serde(rename = "subjectId")]
    pub subject_id: String,

    pub reason: String,

    #[serde(rename = "caseReference")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_reference: Option<String>,
}
