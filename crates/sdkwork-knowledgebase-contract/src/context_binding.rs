use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSpaceContextBinding {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub tenant_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub context_type: KnowledgeContextType,
    pub context_id: String,
    pub context_name: Option<String>,
    pub access_level: KnowledgeAccessLevel,
    pub status: KnowledgeContextBindingStatus,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeSpaceContextBindingRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub context_type: KnowledgeContextType,
    pub context_id: String,
    pub context_name: Option<String>,
    pub access_level: Option<KnowledgeAccessLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateKnowledgeSpaceContextBindingRequest {
    pub context_name: Option<String>,
    pub access_level: Option<KnowledgeAccessLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSpaceContextBindingList {
    pub items: Vec<KnowledgeSpaceContextBinding>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKnowledgeSpaceContextBindingsRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub context_type: Option<KnowledgeContextType>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListContextBoundSpacesRequest {
    pub context_type: KnowledgeContextType,
    pub context_id: String,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeContextType {
    ChatGroup,
    Organization,
    Circle,
    Channel,
    Team,
    Project,
}

impl KnowledgeContextType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChatGroup => "chat_group",
            Self::Organization => "organization",
            Self::Circle => "circle",
            Self::Channel => "channel",
            Self::Team => "team",
            Self::Project => "project",
        }
    }
}

impl FromStr for KnowledgeContextType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat_group" => Ok(Self::ChatGroup),
            "organization" => Ok(Self::Organization),
            "circle" => Ok(Self::Circle),
            "channel" => Ok(Self::Channel),
            "team" => Ok(Self::Team),
            "project" => Ok(Self::Project),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAccessLevel {
    Reader,
    Writer,
}

impl KnowledgeAccessLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Writer => "writer",
        }
    }
}

impl FromStr for KnowledgeAccessLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "reader" => Ok(Self::Reader),
            "writer" => Ok(Self::Writer),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeContextBindingStatus {
    Active,
    Deleted,
}
