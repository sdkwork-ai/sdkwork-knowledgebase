use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CreateKnowledgeSpaceContextBindingRequest {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_level: Option<String>,
}
