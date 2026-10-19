use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeMarketSubscriptionResult {
    pub accepted: bool,

    pub status: String,
}
