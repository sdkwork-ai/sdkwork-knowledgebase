use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeContextFragment, KnowledgeRetrievalTrace};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeRetrievalResult {
    #[serde(rename = "retrievalId")]
    pub retrieval_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<KnowledgeRetrievalTrace>,

    pub hits: Vec<KnowledgeContextFragment>,
}
