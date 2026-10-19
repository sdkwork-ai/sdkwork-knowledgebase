use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatOperationResult {
    pub accepted: bool,

    pub status: String,
}
