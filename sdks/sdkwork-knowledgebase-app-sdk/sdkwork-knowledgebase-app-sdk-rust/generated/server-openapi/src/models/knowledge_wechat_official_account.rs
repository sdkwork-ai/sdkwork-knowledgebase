use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatOfficialAccount {
    pub id: String,

    pub name: String,

    pub r#type: String,

    pub avatar: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "appId")]
    pub app_id: String,

    #[serde(rename = "appSecret")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_secret: Option<String>,

    #[serde(rename = "serverUrl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    #[serde(rename = "encodingAesKey")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_aes_key: Option<String>,

    #[serde(rename = "encryptMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypt_mode: Option<String>,

    #[serde(rename = "domainVerifyFileName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_name: Option<String>,

    /// WeChat domain verification text. The service also enforces a 65,536 UTF-8 byte limit.
    #[serde(rename = "domainVerifyFileContent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_content: Option<String>,

    #[serde(rename = "jsSecureDomains")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub js_secure_domains: Option<Vec<String>>,

    #[serde(rename = "webAuthDomains")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_auth_domains: Option<Vec<String>>,

    #[serde(rename = "businessDomains")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_domains: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}
