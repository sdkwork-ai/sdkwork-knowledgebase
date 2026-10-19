use serde::{Deserialize, Serialize};

use crate::models::{OkfConceptSummary, PageInfo};

/// One bounded cursor page of published OKF concept summaries.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OkfConceptSummaryList {
    pub items: Vec<OkfConceptSummary>,

    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}
