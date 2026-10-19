use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeWechatArticle};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatArticlesPreviewRequest {
    #[serde(rename = "accountId")]
    pub account_id: String,

    #[serde(rename = "wechatIds")]
    pub wechat_ids: Vec<String>,

    pub articles: Vec<KnowledgeWechatArticle>,
}
