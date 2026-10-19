use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeFilter, KnowledgeRetrievalBinding};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeRetrievalRequest {
    #[serde(rename = "actorId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    pub query: String,

    #[serde(rename = "retrievalProfileId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_profile_id: Option<String>,

    pub bindings: Vec<KnowledgeRetrievalBinding>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methods: Option<Vec<String>>,

    #[serde(rename = "topK")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,

    #[serde(rename = "includeCitations")]
    pub include_citations: bool,

    #[serde(rename = "includeTrace")]
    pub include_trace: bool,

    #[serde(rename = "contextBudgetTokens")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_budget_tokens: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<KnowledgeFilter>>,
}
