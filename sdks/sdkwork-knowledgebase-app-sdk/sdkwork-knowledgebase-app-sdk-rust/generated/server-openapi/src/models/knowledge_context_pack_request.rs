use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeRetrievalBinding};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeContextPackRequest {
    #[serde(rename = "actorId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    pub query: String,

    #[serde(rename = "retrievalProfileId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_profile_id: Option<String>,

    pub bindings: Vec<KnowledgeRetrievalBinding>,

    #[serde(rename = "contextBudgetTokens")]
    pub context_budget_tokens: i64,

    #[serde(rename = "includeCitations")]
    pub include_citations: bool,

    #[serde(rename = "memoryPolicyRef")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_policy_ref: Option<String>,
}
