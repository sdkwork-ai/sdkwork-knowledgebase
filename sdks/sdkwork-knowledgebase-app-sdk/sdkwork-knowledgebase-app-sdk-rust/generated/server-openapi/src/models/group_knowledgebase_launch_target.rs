use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GroupKnowledgebaseLaunchTarget {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,

    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "spaceUuid")]
    pub space_uuid: String,

    #[serde(rename = "groupName")]
    pub group_name: String,

    #[serde(rename = "lifecycleState")]
    pub lifecycle_state: String,
}
