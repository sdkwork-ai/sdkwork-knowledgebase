use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeWechatApplet};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWechatAppletList {
    pub applets: Vec<KnowledgeWechatApplet>,
}
