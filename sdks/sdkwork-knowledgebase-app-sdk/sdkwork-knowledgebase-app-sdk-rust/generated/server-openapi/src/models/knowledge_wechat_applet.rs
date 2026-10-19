use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatApplet {
    pub id: String,

    pub name: String,

    #[serde(rename = "appId")]
    pub app_id: String,

    #[serde(rename = "originalId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_id: Option<String>,

    #[serde(rename = "appSecret")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_secret: Option<String>,

    pub path: String,

    pub avatar: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "requestDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_domain: Option<Vec<String>>,

    #[serde(rename = "socketDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_domain: Option<Vec<String>>,

    #[serde(rename = "uploadDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload_domain: Option<Vec<String>>,

    #[serde(rename = "downloadDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_domain: Option<Vec<String>>,

    #[serde(rename = "udpDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udp_domain: Option<Vec<String>>,

    #[serde(rename = "tcpDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_domain: Option<Vec<String>>,

    #[serde(rename = "businessDomain")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_domain: Option<Vec<String>>,

    #[serde(rename = "domainVerifyFileName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_name: Option<String>,

    /// WeChat domain verification text. The service also enforces a 65,536 UTF-8 byte limit.
    #[serde(rename = "domainVerifyFileContent")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_verify_file_content: Option<String>,

    #[serde(rename = "msgToken")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_token: Option<String>,

    #[serde(rename = "msgEncodingAESKey")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_encoding_aeskey: Option<String>,

    #[serde(rename = "msgDataFormat")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_data_format: Option<String>,

    #[serde(rename = "msgEncryptMode")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg_encrypt_mode: Option<String>,
}
