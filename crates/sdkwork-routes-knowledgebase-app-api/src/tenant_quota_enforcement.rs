//! Tenant business quota enforcement at the app-api boundary.

use sdkwork_intelligence_knowledgebase_service::tenant_quota::{
    build_quota_status, ensure_document_capacity, TenantQuotaExceeded, TenantQuotaKind,
};
use sdkwork_knowledgebase_contract::KnowledgeTenantQuotaStatus;
use sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits;
use sdkwork_routes_knowledgebase_backend_api::knowledgebase_rate_limit_store;
use std::time::Duration;

use crate::{runtime::KnowledgebaseRuntime, ApiError, ApiResult};

pub(crate) fn map_tenant_quota_error(error: TenantQuotaExceeded) -> ApiError {
    let detail = match error.kind {
        TenantQuotaKind::Documents => format!(
            "tenant document quota exceeded ({}/{})",
            error.usage, error.limit
        ),
        TenantQuotaKind::IngestConcurrency => format!(
            "tenant ingest concurrency quota exceeded ({}/{})",
            error.usage, error.limit
        ),
        TenantQuotaKind::RetrievalRate => format!(
            "tenant retrieval rate quota exceeded ({}/{})",
            error.usage, error.limit
        ),
        TenantQuotaKind::StorageBytes => format!(
            "tenant storage quota exceeded ({}/{})",
            error.usage, error.limit
        ),
    };
    ApiError::quota_exceeded(detail)
}

async fn load_storage_bytes_used(runtime: &KnowledgebaseRuntime) -> ApiResult<u64> {
    runtime
        .object_ref_store()
        .sum_active_storage_bytes()
        .await
        .map_err(|error| ApiError::internal("knowledgebase_store_failed", error.to_string()))
}

pub(crate) async fn load_tenant_quota_status(
    runtime: &KnowledgebaseRuntime,
) -> ApiResult<KnowledgeTenantQuotaStatus> {
    let limits = KnowledgebaseTenantQuotaLimits::from_env();
    let summary = runtime
        .space_store()
        .summarize_tenant_knowledgebase()
        .await
        .map_err(|error| ApiError::internal("knowledgebase_store_failed", error.to_string()))?;
    let inflight = runtime
        .ingestion_job_store()
        .count_inflight_jobs()
        .await
        .map_err(|error| ApiError::internal("knowledgebase_store_failed", error.to_string()))?;
    let storage_bytes_used = load_storage_bytes_used(runtime).await?;
    Ok(build_quota_status(
        limits,
        summary.document_count,
        inflight,
        storage_bytes_used,
    ))
}

pub(crate) async fn ensure_tenant_can_create_document(
    runtime: &KnowledgebaseRuntime,
) -> ApiResult<()> {
    let limits = KnowledgebaseTenantQuotaLimits::from_env();
    let summary = runtime
        .space_store()
        .summarize_tenant_knowledgebase()
        .await
        .map_err(|error| ApiError::internal("knowledgebase_store_failed", error.to_string()))?;
    ensure_document_capacity(summary.document_count, &limits).map_err(map_tenant_quota_error)
}

pub(crate) async fn ensure_tenant_retrieval_rate(tenant_id: u64) -> ApiResult<()> {
    let limits = KnowledgebaseTenantQuotaLimits::from_env();
    let store = knowledgebase_rate_limit_store();
    let key = format!("kb:tenant:{tenant_id}:retrieval:minute");
    if store
        .check_and_record(
            &key,
            limits.max_retrievals_per_minute,
            Duration::from_secs(60),
        )
        .await
        .is_err()
    {
        return Err(map_tenant_quota_error(TenantQuotaExceeded {
            kind: TenantQuotaKind::RetrievalRate,
            usage: u64::from(limits.max_retrievals_per_minute),
            limit: u64::from(limits.max_retrievals_per_minute),
        }));
    }
    Ok(())
}
