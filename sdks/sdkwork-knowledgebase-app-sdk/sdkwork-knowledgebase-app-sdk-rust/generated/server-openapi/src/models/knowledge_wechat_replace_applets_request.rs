use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeWechatApplet};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatReplaceAppletsRequest {
    pub applets: Vec<KnowledgeWechatApplet>,
}
