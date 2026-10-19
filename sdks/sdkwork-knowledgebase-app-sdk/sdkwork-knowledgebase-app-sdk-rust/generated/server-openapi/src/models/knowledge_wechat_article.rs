use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatArticle {
    pub id: String,

    pub title: String,

    pub author: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#abstract: Option<String>,
}
