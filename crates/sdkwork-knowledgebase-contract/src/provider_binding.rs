use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::knowledge_engine::{KnowledgeEngineCapability, KnowledgeEngineProviderErrorCategory};
use crate::serde_int64::{
    deserialize_option_u64_from_string_or_number,
    deserialize_u64_from_string_or_number,
    serialize_option_u64_as_string,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEngineProviderBindingState {
    Draft,
    Testing,
    Active,
    Degraded,
    Disabled,
    Failed,
}

impl KnowledgeEngineProviderBindingState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Testing => "testing",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Disabled => "disabled",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for KnowledgeEngineProviderBindingState {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "testing" => Ok(Self::Testing),
            "active" => Ok(Self::Active),
            "degraded" => Ok(Self::Degraded),
            "disabled" => Ok(Self::Disabled),
            "failed" => Ok(Self::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineProviderBinding {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    pub uuid: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub tenant_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub organization_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub implementation_id: String,
    pub remote_resource_type: String,
    pub remote_resource_id: String,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub credential_reference_id: Option<u64>,
    pub lifecycle_state: KnowledgeEngineProviderBindingState,
    pub capability_snapshot: Vec<KnowledgeEngineCapability>,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub capability_snapshot_version: u64,
    pub last_tested_at: Option<String>,
    pub activated_at: Option<String>,
    pub disabled_at: Option<String>,
    pub last_error_category: Option<KnowledgeEngineProviderErrorCategory>,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeEngineProviderBindingRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub implementation_id: String,
    pub remote_resource_type: String,
    pub remote_resource_id: String,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub credential_reference_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateKnowledgeEngineProviderBindingRequest {
    pub remote_resource_type: Option<String>,
    pub remote_resource_id: Option<String>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub credential_reference_id: Option<u64>,
    pub clear_credential_reference: bool,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKnowledgeEngineProviderBindingsRequest {
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub space_id: Option<u64>,
    pub lifecycle_state: Option<KnowledgeEngineProviderBindingState>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineProviderBindingList {
    pub items: Vec<KnowledgeEngineProviderBinding>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEngineProviderCredentialRotationState {
    Current,
    RotationDue,
    Revoked,
}

impl KnowledgeEngineProviderCredentialRotationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::RotationDue => "rotation_due",
            Self::Revoked => "revoked",
        }
    }
}

impl FromStr for KnowledgeEngineProviderCredentialRotationState {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "current" => Ok(Self::Current),
            "rotation_due" => Ok(Self::RotationDue),
            "revoked" => Ok(Self::Revoked),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineProviderCredentialReference {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    pub uuid: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub tenant_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub organization_id: u64,
    pub implementation_id: String,
    pub display_name: String,
    pub rotation_state: KnowledgeEngineProviderCredentialRotationState,
    pub last_rotated_at: Option<String>,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeEngineProviderCredentialReferenceRequest {
    pub implementation_id: String,
    pub display_name: String,
    /// Write-only locator owned by the approved secret provider. It is never returned by reads.
    pub reference_locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKnowledgeEngineProviderCredentialReferencesRequest {
    pub implementation_id: Option<String>,
    pub rotation_state: Option<KnowledgeEngineProviderCredentialRotationState>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineProviderCredentialReferenceList {
    pub items: Vec<KnowledgeEngineProviderCredentialReference>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateKnowledgeEngineProviderCredentialReferenceRequest {
    /// Write-only locator owned by the approved secret provider. It is never returned by reads.
    pub reference_locator: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeKnowledgeEngineProviderCredentialReferenceRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBindingVersionCommandRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEngineProviderMigrationState {
    DryRun,
    Preparing,
    Validating,
    Cutover,
    Observing,
    Completed,
    RollingBack,
    RolledBack,
    Failed,
}

impl KnowledgeEngineProviderMigrationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry_run",
            Self::Preparing => "preparing",
            Self::Validating => "validating",
            Self::Cutover => "cutover",
            Self::Observing => "observing",
            Self::Completed => "completed",
            Self::RollingBack => "rolling_back",
            Self::RolledBack => "rolled_back",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for KnowledgeEngineProviderMigrationState {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dry_run" => Ok(Self::DryRun),
            "preparing" => Ok(Self::Preparing),
            "validating" => Ok(Self::Validating),
            "cutover" => Ok(Self::Cutover),
            "observing" => Ok(Self::Observing),
            "completed" => Ok(Self::Completed),
            "rolling_back" => Ok(Self::RollingBack),
            "rolled_back" => Ok(Self::RolledBack),
            "failed" => Ok(Self::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineProviderMigrationOperation {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    pub uuid: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub tenant_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub organization_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub source_binding_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub target_binding_id: u64,
    pub operation_state: KnowledgeEngineProviderMigrationState,
    pub requested_by: String,
    pub attempt_count: u32,
    pub cutover_at: Option<String>,
    pub observation_until: Option<String>,
    pub completed_at: Option<String>,
    pub last_error_category: Option<KnowledgeEngineProviderErrorCategory>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeEngineProviderMigrationOperationRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub source_binding_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub target_binding_id: u64,
    pub idempotency_key: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_source_version: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_target_version: u64,
    pub observation_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKnowledgeEngineProviderMigrationOperationsRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub operation_state: Option<KnowledgeEngineProviderMigrationState>,
    pub cursor: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineProviderMigrationOperationList {
    pub items: Vec<KnowledgeEngineProviderMigrationOperation>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMigrationVersionCommandRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub expected_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineDataScope {
    pub allowed_space_ids: Vec<u64>,
    pub allowed_source_ids: Vec<u64>,
    pub allowed_document_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEngineExecutionContext {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub actor_id: String,
    pub permission_scope: Vec<String>,
    pub data_scope: KnowledgeEngineDataScope,
    pub space_id: u64,
    pub binding_id: Option<u64>,
    pub trace_id: String,
    pub deadline_unix_ms: u64,
}
