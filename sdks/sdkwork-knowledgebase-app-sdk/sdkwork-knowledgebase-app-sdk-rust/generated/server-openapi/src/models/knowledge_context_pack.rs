use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeCitation, KnowledgeContextFragment, KnowledgeMemoryContextFragment};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeContextPack {
    #[serde(rename = "contextPackId")]
    pub context_pack_id: String,

    #[serde(rename = "retrievalId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_id: Option<String>,

    pub query: String,

    pub fragments: Vec<KnowledgeContextFragment>,

    #[serde(rename = "estimatedTokens")]
    pub estimated_tokens: i64,

    pub citations: Vec<KnowledgeCitation>,

    pub truncated: bool,

    #[serde(rename = "memoryFragments")]
    pub memory_fragments: Vec<KnowledgeMemoryContextFragment>,
}
