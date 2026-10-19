use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeAgentBinding};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentProfile {
    #[serde(rename = "profileId")]
    pub profile_id: String,

    #[serde(rename = "tenantId")]
    pub tenant_id: String,

    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "systemInstruction")]
    pub system_instruction: String,

    #[serde(rename = "modelProviderId")]
    pub model_provider_id: String,

    #[serde(rename = "modelId")]
    pub model_id: String,

    #[serde(rename = "modelParameters")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_parameters: Option<String>,

    #[serde(rename = "retrievalProfileId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_profile_id: Option<String>,

    #[serde(rename = "citationPolicy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_policy: Option<String>,

    #[serde(rename = "memoryPolicyRef")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_policy_ref: Option<String>,

    #[serde(rename = "toolPolicyRef")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_policy_ref: Option<String>,

    #[serde(rename = "answerPolicy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer_policy: Option<String>,

    /// Knowledge execution mode. okf_bundle uses native OKF bundle knowledge, rag uses native RAG retrieval, and external delegates to an external knowledge engine integration.
    #[serde(rename = "knowledgeMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_mode: Option<String>,

    #[serde(rename = "agentImplementationId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_implementation_id: Option<String>,

    pub status: String,

    pub bindings: Vec<KnowledgeAgentBinding>,
}
