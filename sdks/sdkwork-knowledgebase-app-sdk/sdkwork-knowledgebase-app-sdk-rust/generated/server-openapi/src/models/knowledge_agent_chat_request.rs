use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeAgentChatMessage};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentChatRequest {
    #[serde(rename = "actorId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    pub message: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<KnowledgeAgentChatMessage>>,

    /// Knowledge execution mode. okf_bundle uses native OKF bundle knowledge, rag uses native RAG retrieval, and external delegates to an external knowledge engine integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    #[serde(rename = "sessionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    #[serde(rename = "modelProviderId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provider_id: Option<String>,

    #[serde(rename = "modelId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    #[serde(rename = "agentImplementationId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_implementation_id: Option<String>,
}
