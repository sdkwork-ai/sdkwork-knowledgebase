use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeWechatArticle};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatArticlesPublishRequest {
    #[serde(rename = "accountIds")]
    pub account_ids: Vec<String>,

    pub articles: Vec<KnowledgeWechatArticle>,

    #[serde(rename = "sendNotification")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_notification: Option<bool>,

    #[serde(rename = "groupNotification")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_notification: Option<bool>,

    #[serde(rename = "selectedGroupId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_group_id: Option<String>,

    #[serde(rename = "scheduleTime")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_time: Option<String>,
}
