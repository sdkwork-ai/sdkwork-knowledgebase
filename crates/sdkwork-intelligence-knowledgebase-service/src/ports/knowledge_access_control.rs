use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait KnowledgeAccessControl: Send + Sync {
    async fn check_space_access(
        &self,
        request: KnowledgeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError>;

    async fn check_node_access(
        &self,
        request: KnowledgeNodeAccessCheckRequest,
    ) -> Result<KnowledgeAccessGrant, KnowledgeAccessControlError>;

    async fn grant_space_access(
        &self,
        request: GrantKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError>;

    async fn revoke_space_access(
        &self,
        request: RevokeKnowledgeSpaceAccessRequest,
    ) -> Result<(), KnowledgeAccessControlError>;

    async fn list_space_members(
        &self,
        request: ListKnowledgeSpaceMembersRequest,
    ) -> Result<KnowledgeSpaceMemberList, KnowledgeAccessControlError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeAccessCheckRequest {
    pub tenant_id: String,
    pub actor_id: String,
    pub drive_space_id: String,
    pub required_role: KnowledgeAccessRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeNodeAccessCheckRequest {
    pub tenant_id: String,
    pub actor_id: String,
    pub drive_space_id: String,
    pub drive_node_id: String,
    pub required_role: KnowledgeAccessRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantKnowledgeSpaceAccessRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub drive_node_id: Option<String>,
    pub subject_type: KnowledgeSubjectType,
    pub subject_id: String,
    pub role: KnowledgeAccessRole,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeKnowledgeSpaceAccessRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub drive_node_id: Option<String>,
    pub subject_type: KnowledgeSubjectType,
    pub subject_id: String,
    pub operator_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListKnowledgeSpaceMembersRequest {
    pub tenant_id: String,
    pub drive_space_id: String,
    pub drive_node_id: Option<String>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeAccessGrant {
    pub allowed: bool,
    pub effective_role: Option<KnowledgeAccessRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeSpaceMemberList {
    pub members: Vec<KnowledgeSpaceMember>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeSpaceMember {
    pub subject_type: KnowledgeSubjectType,
    pub subject_id: String,
    pub role: KnowledgeAccessRole,
    pub inherited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeAccessRole {
    Reader,
    Writer,
    Owner,
}

impl KnowledgeAccessRole {
    pub fn at_least_reader(&self) -> bool {
        true
    }

    pub fn at_least_writer(&self) -> bool {
        matches!(self, Self::Writer | Self::Owner)
    }

    pub fn is_owner(&self) -> bool {
        matches!(self, Self::Owner)
    }

    pub fn satisfies(&self, required: &KnowledgeAccessRole) -> bool {
        match required {
            KnowledgeAccessRole::Reader => self.at_least_reader(),
            KnowledgeAccessRole::Writer => self.at_least_writer(),
            KnowledgeAccessRole::Owner => self.is_owner(),
        }
    }

    pub fn from_drive_role(role: &str) -> Option<Self> {
        match role {
            "reader" | "commenter" => Some(Self::Reader),
            "writer" => Some(Self::Writer),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }

    pub fn to_drive_role(&self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Writer => "writer",
            Self::Owner => "owner",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeSubjectType {
    User,
    Group,
    Domain,
    App,
}

impl KnowledgeSubjectType {
    pub fn to_drive_subject_type(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Group => "group",
            Self::Domain => "domain",
            Self::App => "app",
        }
    }

    pub fn from_drive_subject_type(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "group" => Some(Self::Group),
            "domain" => Some(Self::Domain),
            "app" => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeAccessControlError {
    #[error("knowledge access control invalid request: {0}")]
    InvalidRequest(String),
    #[error("knowledge access control denied: {0}")]
    Denied(String),
    /// The addressed drive-side resource (for example a space's permission
    /// anchor node or bound drive space) does not exist. Distinct from
    /// `InvalidRequest` so callers can retire a definitively missing binding
    /// instead of surfacing a permanent 5xx.
    #[error("knowledge access control resource not found: {0}")]
    NotFound(String),
    #[error("knowledge access control upstream error: {0}")]
    Upstream(String),
    #[error("knowledge access control internal error: {0}")]
    Internal(String),
}
