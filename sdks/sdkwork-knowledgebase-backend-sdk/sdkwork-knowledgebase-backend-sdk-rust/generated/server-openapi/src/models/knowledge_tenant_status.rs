use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeTenantQuotaStatus};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeTenantStatus {
    #[serde(rename = "tenantName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_name: Option<String>,

    pub status: String,

    #[serde(rename = "spaceCount")]
    pub space_count: String,

    #[serde(rename = "documentCount")]
    pub document_count: String,

    #[serde(rename = "createdAt")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<KnowledgeTenantQuotaStatus>,
}
