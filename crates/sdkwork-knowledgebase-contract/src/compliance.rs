use crate::serde_int64::{deserialize_u64_from_string_or_number, serialize_u64_as_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportKnowledgeAuditEventsRequest {
    pub actor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAuditEventItem {
    pub id: String,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub result: String,
    pub trace_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAuditEventExport {
    pub items: Vec<KnowledgeAuditEventItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymizeKnowledgeAuditSubjectRequest {
    pub actor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymizeKnowledgeAuditSubjectResult {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub anonymized_count: u64,
}

/// Subject dimension frozen by an audit legal hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeAuditLegalHoldSubjectType {
    Actor,
    Resource,
}

impl KnowledgeAuditLegalHoldSubjectType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Resource => "resource",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeAuditLegalHoldStatus {
    Active,
    Released,
}

impl KnowledgeAuditLegalHoldStatus {
    #[must_use]
    pub fn from_persisted(status: i64) -> Self {
        if status == 0 {
            Self::Released
        } else {
            Self::Active
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeAuditLegalHoldRequest {
    pub subject_type: KnowledgeAuditLegalHoldSubjectType,
    pub subject_id: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAuditLegalHoldItem {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub hold_id: u64,
    pub uuid: String,
    pub subject_type: KnowledgeAuditLegalHoldSubjectType,
    pub subject_id: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_reference: Option<String>,
    pub status: KnowledgeAuditLegalHoldStatus,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
    pub created_at: String,
}
