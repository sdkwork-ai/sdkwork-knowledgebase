use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeSpaceContextBinding {
    pub id: String,

    #[serde(rename = "tenantId")]
    pub tenant_id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "contextType")]
    pub context_type: String,

    #[serde(rename = "contextId")]
    pub context_id: String,

    #[serde(rename = "contextName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_name: Option<String>,

    #[serde(rename = "accessLevel")]
    pub access_level: String,

    pub status: String,

    #[serde(rename = "createdBy")]
    pub created_by: String,

    #[serde(rename = "createdAt")]
    pub created_at: String,

    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}
