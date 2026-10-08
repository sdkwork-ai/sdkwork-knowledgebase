use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeFilter};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentBinding {
    #[serde(rename = "bindingId")]
    pub binding_id: String,

    #[serde(rename = "profileId")]
    pub profile_id: String,

    #[serde(rename = "tenantId")]
    pub tenant_id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "sourceFilter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_filter: Option<Vec<KnowledgeFilter>>,

    #[serde(rename = "documentFilter")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_filter: Option<Vec<KnowledgeFilter>>,

    pub priority: i64,

    #[serde(rename = "topK")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,

    #[serde(rename = "minScore")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,

    pub enabled: bool,
}
