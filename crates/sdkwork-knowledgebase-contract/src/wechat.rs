use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeWechatOfficialAccount {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub avatar: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub app_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_aes_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypt_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub js_secure_domains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_auth_domains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_domains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeWechatOfficialAccountList {
    pub accounts: Vec<KnowledgeWechatOfficialAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeWechatReplaceOfficialAccountsRequest {
    pub accounts: Vec<KnowledgeWechatOfficialAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeWechatApplet {
    pub id: String,
    pub name: String,
    pub app_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_secret: Option<String>,
    pub path: String,
    pub avatar: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udp_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_encoding_aes_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_data_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_encrypt_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeWechatAppletList {
    pub applets: Vec<KnowledgeWechatApplet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeWechatReplaceAppletsRequest {
    pub applets: Vec<KnowledgeWechatApplet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeWechatArticlesPublishRequest {
    pub account_ids: Vec<String>,
    pub articles: Vec<KnowledgeWechatArticle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_notification: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_notification: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeWechatArticlesPreviewRequest {
    pub account_id: String,
    pub wechat_ids: Vec<String>,
    pub articles: Vec<KnowledgeWechatArticle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeWechatOperationResult {
    pub accepted: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeWechatFanTag {
    pub id: String,
    pub name: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub fan_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeWechatFanTagList {
    pub tags: Vec<KnowledgeWechatFanTag>,
}

#[cfg(test)]
mod tests {
    use super::{
        KnowledgeWechatReplaceAppletsRequest, KnowledgeWechatReplaceOfficialAccountsRequest,
    };
    use serde_json::json;

    #[test]
    fn replacement_contracts_reject_unknown_fields() {
        let official_account = json!({
            "accounts": [{
                "id": "oa-1",
                "name": "Account",
                "type": "subscription",
                "avatar": "OA",
                "appId": "wx-account",
                "unexpected": true
            }]
        });
        assert!(
            serde_json::from_value::<KnowledgeWechatReplaceOfficialAccountsRequest>(
                official_account
            )
            .is_err()
        );

        let applet = json!({
            "applets": [],
            "unexpected": true
        });
        assert!(serde_json::from_value::<KnowledgeWechatReplaceAppletsRequest>(applet).is_err());
    }
}
