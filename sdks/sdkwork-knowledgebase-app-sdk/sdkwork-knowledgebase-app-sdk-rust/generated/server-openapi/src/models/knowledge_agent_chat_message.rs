use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentChatMessage {
    pub role: String,

    pub content: String,
}
