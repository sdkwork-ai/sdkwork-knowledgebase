use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAuditLegalHoldItem {
    #[serde(rename = "holdId")]
    pub hold_id: String,

    pub uuid: String,

    #[serde(rename = "subjectType")]
    pub subject_type: String,

    #[serde(rename = "subjectId")]
    pub subject_id: String,

    pub reason: String,

    #[serde(rename = "caseReference")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_reference: Option<String>,

    pub status: String,

    #[serde(rename = "createdBy")]
    pub created_by: String,

    #[serde(rename = "releasedBy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_by: Option<String>,

    #[serde(rename = "releasedAt")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,

    #[serde(rename = "createdAt")]
    pub created_at: String,
}
