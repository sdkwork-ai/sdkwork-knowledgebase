use serde::{Deserialize, Serialize};

use crate::models::WikiPublicPageMetadata;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WikiRouteResolution {
    pub disposition: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<WikiPublicPageMetadata>,

    #[serde(rename = "contentHandle")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_handle: Option<String>,

    #[serde(rename = "requestedRoute")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_route: Option<String>,

    #[serde(rename = "canonicalRoute")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_route: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<i64>,

    #[serde(rename = "pagePublicVersion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_public_version: Option<String>,
}
