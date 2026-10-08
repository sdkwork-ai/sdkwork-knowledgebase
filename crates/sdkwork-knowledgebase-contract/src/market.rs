use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMarketCatalogItem {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub subscribers_count: u32,
    pub documents_count: u32,
    pub provider: String,
    pub model_name: String,
    pub is_subscribed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMarketCatalogList {
    pub items: Vec<KnowledgeMarketCatalogItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMarketSubscriptionRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub listing_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMarketSubscriptionResult {
    pub accepted: bool,
    pub status: String,
}
