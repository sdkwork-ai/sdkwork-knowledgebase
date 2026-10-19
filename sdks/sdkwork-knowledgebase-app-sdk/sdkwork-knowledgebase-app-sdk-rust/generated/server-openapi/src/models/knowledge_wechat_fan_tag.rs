use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatFanTag {
    pub id: String,

    pub name: String,

    #[serde(rename = "fanCount")]
    pub fan_count: String,
}
