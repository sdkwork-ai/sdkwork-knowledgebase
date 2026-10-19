use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeBrowserNodePermissions {
    #[serde(rename = "canRead")]
    pub can_read: bool,

    #[serde(rename = "canUpload")]
    pub can_upload: bool,

    #[serde(rename = "canRename")]
    pub can_rename: bool,

    #[serde(rename = "canMove")]
    pub can_move: bool,

    #[serde(rename = "canDelete")]
    pub can_delete: bool,

    #[serde(rename = "canReview")]
    pub can_review: bool,

    #[serde(rename = "canPublish")]
    pub can_publish: bool,
}
