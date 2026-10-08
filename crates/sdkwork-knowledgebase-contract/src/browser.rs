use sdkwork_utils_rust::{PageInfo, PageMode};
use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_option_u64_from_string_or_number,
    deserialize_u64_from_string_or_number,
    serialize_option_u64_as_string,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBrowserView {
    Files,
    OkfBundle,
    Outputs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeBrowserNodeType {
    Folder,
    Document,
    OkfConcept,
    Candidate,
    Answer,
    Report,
    VirtualFolder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKnowledgeBrowserRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub parent_id: Option<String>,
    pub view: KnowledgeBrowserView,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBrowserPage {
    pub space_id: u64,
    pub drive_space_id: String,
    pub parent_id: Option<String>,
    pub view: KnowledgeBrowserView,
    pub page_size: u32,
    pub items: Vec<KnowledgeBrowserNode>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBrowserListData {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub drive_space_id: String,
    pub parent_id: Option<String>,
    pub view: KnowledgeBrowserView,
    pub page_size: u32,
    pub items: Vec<KnowledgeBrowserNode>,
    pub page_info: PageInfo,
}

impl From<KnowledgeBrowserPage> for KnowledgeBrowserListData {
    fn from(page: KnowledgeBrowserPage) -> Self {
        let next_cursor = page.next_cursor;
        Self {
            space_id: page.space_id,
            drive_space_id: page.drive_space_id,
            parent_id: page.parent_id,
            view: page.view,
            page_size: page.page_size,
            items: page.items,
            page_info: PageInfo {
                mode: PageMode::Cursor,
                page: None,
                page_size: Some(page.page_size as i32),
                total_items: None,
                total_pages: None,
                has_more: Some(next_cursor.is_some()),
                next_cursor,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBrowserNode {
    pub id: String,
    pub node_type: KnowledgeBrowserNodeType,
    pub name: String,
    pub parent_id: Option<String>,
    pub path: String,
    pub drive_space_id: Option<String>,
    pub drive_node_id: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub document_id: Option<u64>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub document_version_id: Option<u64>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub concept_id: Option<u64>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub concept_revision_id: Option<u64>,
    pub mime_type: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub size_bytes: Option<u64>,
    pub ingest_state: Option<String>,
    pub parse_state: Option<String>,
    pub index_state: Option<String>,
    pub okf_state: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub children_count: Option<u64>,
    pub updated_at: String,
    pub permissions: KnowledgeBrowserNodePermissions,
    pub drive_storage_provider_id: Option<String>,
    pub drive_bucket: Option<String>,
    pub drive_object_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBrowserNodePermissions {
    pub can_read: bool,
    pub can_upload: bool,
    pub can_rename: bool,
    pub can_move: bool,
    pub can_delete: bool,
    pub can_review: bool,
    pub can_publish: bool,
}

impl KnowledgeBrowserNodePermissions {
    pub const fn read_only() -> Self {
        Self {
            can_read: true,
            can_upload: false,
            can_rename: false,
            can_move: false,
            can_delete: false,
            can_review: false,
            can_publish: false,
        }
    }

    pub const fn file_manager() -> Self {
        Self {
            can_read: true,
            can_upload: true,
            can_rename: true,
            can_move: true,
            can_delete: true,
            can_review: false,
            can_publish: false,
        }
    }
}
