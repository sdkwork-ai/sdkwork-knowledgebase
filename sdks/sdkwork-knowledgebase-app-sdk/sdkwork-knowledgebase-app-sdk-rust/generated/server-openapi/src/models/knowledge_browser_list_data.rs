use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeBrowserNode, PageInfo};

/// Standard browser list response data. It follows SDKWork list semantics with items and pageInfo, and also returns the resolved Drive view context needed by clients.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeBrowserListData {
    #[serde(rename = "spaceId")]
    pub space_id: String,

    /// Drive space id bound to the knowledge space.
    #[serde(rename = "driveSpaceId")]
    pub drive_space_id: String,

    /// Resolved Drive folder id for the current browser view page. When request parentId is omitted, this is the view root folder id; OKF files view resolves to sources/raw.
    #[serde(rename = "parentId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Browser view selector. files lists original source files; for OKF spaces this is sources/raw. okf_bundle lists generated OKF bundle files under okf. outputs lists generated output files under output.
    pub view: String,

    #[serde(rename = "pageSize")]
    pub page_size: i64,

    pub items: Vec<KnowledgeBrowserNode>,

    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}
