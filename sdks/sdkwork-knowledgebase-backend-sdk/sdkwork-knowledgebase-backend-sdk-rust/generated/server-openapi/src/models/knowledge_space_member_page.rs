use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeSpaceMember, PageInfo};

/// One bounded cursor page of knowledge space members.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeSpaceMemberPage {
    pub items: Vec<KnowledgeSpaceMember>,

    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}
