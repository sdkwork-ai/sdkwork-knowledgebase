use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeMarketCatalogItem {
    pub id: String,

    pub title: String,

    pub icon: String,

    pub description: String,

    pub author: String,

    pub tags: Vec<String>,

    #[serde(rename = "subscribersCount")]
    pub subscribers_count: i64,

    #[serde(rename = "documentsCount")]
    pub documents_count: i64,

    pub provider: String,

    #[serde(rename = "modelName")]
    pub model_name: String,

    #[serde(rename = "isSubscribed")]
    pub is_subscribed: bool,
}
