use crate::rag::KnowledgeAgentKnowledgeMode;
use crate::serde_int64::{deserialize_u64_from_string_or_number, serialize_u64_as_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeSpaceRequest {
    pub name: String,
    pub description: Option<String>,
    pub owner_subject_type: Option<String>,
    pub owner_subject_id: Option<String>,
    #[serde(default)]
    pub knowledge_mode: KnowledgeAgentKnowledgeMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateKnowledgeSpaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSpace {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    pub uuid: String,
    /// Server-side ownership metadata used for object-level authorization.
    ///
    /// The creating actor is recorded when the space is created and is never
    /// serialized on the wire, so the public response shape and generated SDK
    /// models are unchanged. A space always remains reachable by its creator,
    /// even when the separate Drive ACL projection for it is missing or stale;
    /// `0` marks a space created before ownership was recorded (or by a
    /// server-side path without an authenticated actor), which only keeps the
    /// Drive ACL decision.
    #[serde(default, skip_serializing)]
    pub created_by: u64,
    pub name: String,
    pub description: Option<String>,
    pub drive_space_id: Option<String>,
    pub status: KnowledgeSpaceStatus,
    pub okf_bundle_initialized: bool,
    pub knowledge_mode: KnowledgeAgentKnowledgeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSpaceStatus {
    /// Internal-only state used while a managed space is being initialized. Generic app routes
    /// must not list or retrieve it.
    Provisioning,
    Active,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSpaceList {
    pub items: Vec<KnowledgeSpace>,
}
