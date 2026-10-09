use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use crate::enums::{OkfBundleFileKind, OkfCandidateType, OkfLogEventType};
use crate::serde_int64::{
    deserialize_option_u64_from_string_or_number,
    deserialize_u64_from_string_or_number,
    serialize_option_u64_as_string,
    serialize_u64_as_string,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfBundlePaths {
    pub agents_md: &'static str,
    pub profile_yaml: &'static str,
    pub index_md: &'static str,
    pub log_md: &'static str,
    pub governance_root: &'static str,
    pub local_mirror_agents_md: &'static str,
    pub local_mirror_profile: &'static str,
    pub local_mirror_raw_root: &'static str,
    pub local_mirror_bundle_root: &'static str,
}

impl Default for OkfBundlePaths {
    fn default() -> Self {
        Self {
            agents_md: "okf/schema/AGENTS.md",
            profile_yaml: "okf/schema/okf_profile.yaml",
            index_md: "okf/index.md",
            log_md: "okf/log.md",
            governance_root: ".sdkwork/governance",
            local_mirror_agents_md: "schema/AGENTS.md",
            local_mirror_profile: "schema/okf_profile.yaml",
            local_mirror_raw_root: "raw/",
            local_mirror_bundle_root: ".",
        }
    }
}

impl OkfBundlePaths {
    pub fn concept_logical_path(concept_id: &str) -> String {
        format!("okf/{concept_id}.md")
    }

    pub fn concept_id_from_logical_path(logical_path: &str) -> Option<String> {
        let path = logical_path.trim();
        let path = path.strip_prefix("okf/")?;
        let path = path.strip_suffix(".md")?;
        if path.is_empty() || path.contains("..") {
            return None;
        }
        Some(path.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfConceptSummary {
    pub title: String,
    pub concept_id: String,
    pub concept_type: String,
    pub logical_path: String,
    pub bundle_relative_path: String,
    pub description: String,
    pub source_count: u32,
    pub updated_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfConceptSummaryList {
    pub items: Vec<OkfConceptSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOkfConceptsQuery {
    // Wire name follows the app-api contract's snake_case query canon
    // (`space_id`), matching `documents.list` and the materialized OpenAPI.
    #[serde(
        rename = "space_id",
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub cursor: Option<String>,
    #[serde(rename = "page_size")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfIndexDocument {
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfLogDocument {
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfProfileDocument {
    pub agents_markdown: String,
    pub profile_yaml: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfQueryRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfQueryResult {
    pub answer_markdown: String,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfFileAnswerRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub title: String,
    pub answer_markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfContextPackRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfCompileJobRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub source_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfCandidateResult {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfCandidateResultList {
    pub items: Vec<OkfCandidateResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfCandidateReviewRequest {
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub reviewer_id: Option<u64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfConceptPublishRequest {
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub publisher_id: Option<u64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfProfileRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub profile_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfIndexRebuildRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfBundleExportRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub export_type: String,
    #[serde(default)]
    pub stage_for_import: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfBundleImportRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub import_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfBundleImportResult {
    pub imported_concept_count: u32,
    pub skipped_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfQualityRunRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfQualityRun {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfBundleLintResult {
    pub conformance: String,
    pub issues: Vec<OkfLintIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfLintIssue {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub concept_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OkfConceptPublishState {
    Draft,
    CandidateReady,
    NeedsReview,
    Published,
    Stale,
    Rejected,
    Failed,
}

impl OkfConceptPublishState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::CandidateReady => "candidate_ready",
            Self::NeedsReview => "needs_review",
            Self::Published => "published",
            Self::Stale => "stale",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OkfRevisionReviewState {
    Pending,
    Approved,
    Rejected,
}

impl OkfRevisionReviewState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfConcept {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub concept_id: String,
    pub title: String,
    pub concept_type: String,
    pub logical_path: String,
    pub bundle_relative_path: String,
    pub description: String,
    pub source_count: u32,
    pub tags: Vec<String>,
    #[serde(
        default,
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string_or_number"
    )]
    pub current_revision_id: Option<u64>,
    pub publish_state: OkfConceptPublishState,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfConceptRevision {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub concept_row_id: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub revision_no: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub markdown_object_ref_id: u64,
    pub content_hash: String,
    pub review_state: OkfRevisionReviewState,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfConceptRevisionList {
    pub items: Vec<KnowledgeOkfConceptRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOkfConceptPublication {
    pub concept: KnowledgeOkfConcept,
    pub revision: KnowledgeOkfConceptRevision,
    pub published_logical_path: String,
    pub governance_revision_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishKnowledgeOkfConceptRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub concept_id: String,
    pub title: String,
    pub concept_type: String,
    pub description: String,
    pub markdown: String,
    pub source_count: u32,
    pub tags: Vec<String>,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub frontmatter_extensions: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfConceptUpsertRequest {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_id: u64,
    pub concept_id: String,
    pub markdown: String,
    pub actor: String,
    pub publish: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkfLogEntry {
    pub occurred_at: String,
    pub event_type: OkfLogEventType,
    pub title: String,
    pub actor: String,
    pub affected_concepts: Vec<String>,
    pub audit_event_id: Option<String>,
    pub warnings: Vec<String>,
}

pub const OKF_KNOWLEDGE_PROVIDER_ID: &str = "provider.knowledge.okf";

pub fn okf_document_id(space_id: u64, concept_id: &str) -> String {
    format!("okf:{space_id}:{concept_id}")
}
