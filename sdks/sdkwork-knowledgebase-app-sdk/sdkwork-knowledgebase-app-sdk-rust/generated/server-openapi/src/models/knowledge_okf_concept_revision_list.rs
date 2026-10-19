use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeOkfConceptRevision, PageInfo};

/// One bounded cursor page of OKF concept revisions.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeOkfConceptRevisionList {
    pub items: Vec<KnowledgeOkfConceptRevision>,

    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}
