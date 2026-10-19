use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeGitImportRequest {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    #[serde(rename = "repoUrl")]
    pub repo_url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,

    #[serde(rename = "gitAccessToken")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_access_token: Option<String>,
}
