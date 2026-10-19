use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdateKnowledgeSpaceContextBindingRequest {
    #[serde(rename = "contextName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_name: Option<String>,

    #[serde(rename = "accessLevel")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_level: Option<String>,
}
