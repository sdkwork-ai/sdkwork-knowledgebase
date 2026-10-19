use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeWechatOfficialAccount};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatOfficialAccountList {
    pub accounts: Vec<KnowledgeWechatOfficialAccount>,
}
