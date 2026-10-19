use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeAgentChatCitation};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentChatResponse {
    #[serde(rename = "chatId")]
    pub chat_id: String,

    pub answer: String,

    /// Knowledge execution mode. okf_bundle uses native OKF bundle knowledge, rag uses native RAG retrieval, and external delegates to an external knowledge engine integration.
    pub mode: String,

    #[serde(rename = "agentImplementationId")]
    pub agent_implementation_id: String,

    #[serde(rename = "modelProviderId")]
    pub model_provider_id: String,

    #[serde(rename = "modelId")]
    pub model_id: String,

    pub citations: Vec<KnowledgeAgentChatCitation>,

    #[serde(rename = "retrievalId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_id: Option<String>,

    #[serde(rename = "sessionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}
