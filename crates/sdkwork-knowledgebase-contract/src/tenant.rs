use serde::{Deserialize, Serialize};
use crate::serde_int64::{
    deserialize_u64_from_string_or_number,
    serialize_u64_as_string,
};

// ============================================================================
// Tenant contract types
// ============================================================================
//
// DESIGN PRINCIPLES (from SDKWork tenant isolation spec):
// 1. Tenant identity is NEVER accepted from request parameters — always derived
//    from the authenticated access token via KnowledgeBackendRequestContext.
// 2. Tenant creation/management is handled by the IAM layer — knowledgebase only
//    reports tenant-level statistics (space count, document count, status).
// 3. All tenant-scoped operations derive tenant info from token claims.
// ============================================================================

/// Summary representation of the **caller's own tenant** knowledgebase status.
///
/// **Security constraint**: Only the caller's own tenant is returned.
/// No `tenant_id` is exposed — the token already encodes the tenant identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeTenantStatus {
    /// Human-readable tenant name (derived from IAM context if available)
    pub tenant_name: Option<String>,
    /// Current lifecycle status (derived from IAM context)
    pub status: KnowledgeTenantStatusEnum,
    /// Number of knowledge spaces owned by this tenant
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub space_count: u64,
    /// Number of documents across all spaces
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub document_count: u64,
    /// ISO 8601 creation timestamp (first space created)
    pub created_at: Option<String>,
    /// Current usage versus configured tenant business quotas
    pub quota: Option<KnowledgeTenantQuotaStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeTenantQuotaStatus {
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub max_documents: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub document_count: u64,
    pub max_concurrent_ingest_jobs: u32,
    pub inflight_ingest_jobs: u32,
    pub max_retrievals_per_minute: u32,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub max_storage_bytes: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub storage_bytes_used: u64,
}

/// Tenant lifecycle status enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeTenantStatusEnum {
    Active = 1,
    Suspended = 2,
    Archived = 3,
}
