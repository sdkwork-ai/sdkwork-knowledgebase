use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeAgentBinding};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeAgentBindingList {
    pub items: Vec<KnowledgeAgentBinding>,
}
