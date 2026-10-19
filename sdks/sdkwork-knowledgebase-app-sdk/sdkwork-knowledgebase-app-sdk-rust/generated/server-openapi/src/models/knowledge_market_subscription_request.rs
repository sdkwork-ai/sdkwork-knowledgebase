use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeMarketSubscriptionRequest {
    #[serde(rename = "listingId")]
    pub listing_id: String,
}
