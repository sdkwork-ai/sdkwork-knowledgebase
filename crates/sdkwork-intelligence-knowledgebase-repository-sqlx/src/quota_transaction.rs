use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::KNOWLEDGE_UPLOAD_SESSION_TTL;
use sdkwork_intelligence_knowledgebase_service::tenant_quota::{
    TenantQuotaExceeded, TenantQuotaKind,
};
use sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits;
use sqlx::{Any, AnyConnection, AnyPool, Transaction};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const KNOWLEDGEBASE_QUOTA_LOCK_NAMESPACE: i64 = 0x4B42_5155_4F54_4100;

pub(crate) async fn begin_tenant_quota_transaction(
    pool: &AnyPool,
    tenant_id: i64,
    organization_id: i64,
) -> Result<Transaction<'static, Any>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(CAST($1 AS TEXT) || ':' || CAST($2 AS TEXT), $3))",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(KNOWLEDGEBASE_QUOTA_LOCK_NAMESPACE)
    .execute(&mut *transaction)
    .await?;
    Ok(transaction)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TenantQuotaTransactionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Quota(#[from] TenantQuotaExceeded),
    #[error("tenant quota transaction error: {0}")]
    Invalid(String),
}

/// O(1) active-document count read from the trigger-maintained usage row in
/// `kb_tenant_quota_usage`, falling back to the exact aggregate scan only when
/// the counter row has not been materialized yet (cold scope or drift repair).
/// The row triggers on `kb_document` keep the counter exact within the caller's
/// transaction, so the advisory-locked quota check stays atomic without paying
/// one full COUNT scan per write.
pub(crate) async fn tenant_active_document_count(
    connection: &mut AnyConnection,
    tenant_id: i64,
    organization_id: i64,
) -> Result<u64, sqlx::Error> {
    let counter: Option<i64> = sqlx::query_scalar(
        "SELECT document_count FROM kb_tenant_quota_usage \
         WHERE tenant_id = $1 AND organization_id = $2",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(document_count) = counter {
        return Ok(document_count.max(0) as u64);
    }
    let document_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_document WHERE tenant_id = $1 AND organization_id = $2 AND status = 1",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .fetch_one(&mut *connection)
    .await?;
    Ok(document_count.max(0) as u64)
}

/// O(1) active storage-bytes total read from the trigger-maintained usage row
/// in `kb_tenant_quota_usage`, falling back to the exact aggregate scan only
/// when the counter row has not been materialized yet. The row triggers on
/// `kb_drive_object_ref` keep the counter exact within the caller's
/// transaction, so the advisory-locked quota check stays atomic without paying
/// one full SUM scan per write.
pub(crate) async fn tenant_active_storage_bytes(
    connection: &mut AnyConnection,
    tenant_id: i64,
    organization_id: i64,
) -> Result<u64, sqlx::Error> {
    let counter: Option<i64> = sqlx::query_scalar(
        "SELECT storage_bytes FROM kb_tenant_quota_usage \
         WHERE tenant_id = $1 AND organization_id = $2",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(storage_bytes) = counter {
        return Ok(storage_bytes.max(0) as u64);
    }
    let storage_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM kb_drive_object_ref WHERE tenant_id = $1 AND organization_id = $2 AND status = 1",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .fetch_one(&mut *connection)
    .await?;
    Ok(storage_bytes.max(0) as u64)
}

pub(crate) async fn enforce_tenant_quotas_after_write(
    connection: &mut AnyConnection,
    tenant_id: i64,
    organization_id: i64,
    limits: KnowledgebaseTenantQuotaLimits,
) -> Result<(), TenantQuotaTransactionError> {
    let document_count =
        tenant_active_document_count(connection, tenant_id, organization_id).await?;
    if document_count > limits.max_documents {
        return Err(TenantQuotaExceeded {
            kind: TenantQuotaKind::Documents,
            usage: document_count,
            limit: limits.max_documents,
        }
        .into());
    }

    let cutoff = OffsetDateTime::now_utc()
        .checked_sub(KNOWLEDGE_UPLOAD_SESSION_TTL)
        .ok_or_else(|| {
            TenantQuotaTransactionError::Invalid(
                "upload session quota cutoff is outside the supported timestamp range".to_string(),
            )
        })?
        .format(&Rfc3339)
        .map_err(|error| TenantQuotaTransactionError::Invalid(error.to_string()))?;
    let cutoff_expr = "CAST($7 AS TIMESTAMP)";
    let query = format!(
        r#"
        SELECT COUNT(*)
        FROM kb_ingestion_job
        WHERE tenant_id = $1
          AND organization_id = $2
          AND status = $3
          AND state IN ($4, $5)
          AND NOT (job_type = $6 AND created_at <= {cutoff_expr})
        "#,
    );
    let inflight_count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(query.as_str()))
        .bind(tenant_id)
        .bind(organization_id)
        .bind(1_i64)
        .bind(0_i64)
        .bind(1_i64)
        .bind("upload_session")
        .bind(cutoff)
        .fetch_one(&mut *connection)
        .await?;
    let inflight_count = u64::try_from(inflight_count.max(0))
        .map_err(|error| TenantQuotaTransactionError::Invalid(error.to_string()))?;
    if inflight_count > u64::from(limits.max_concurrent_ingest_jobs) {
        return Err(TenantQuotaExceeded {
            kind: TenantQuotaKind::IngestConcurrency,
            usage: inflight_count,
            limit: u64::from(limits.max_concurrent_ingest_jobs),
        }
        .into());
    }

    let storage_bytes =
        tenant_active_storage_bytes(connection, tenant_id, organization_id).await?;
    if storage_bytes > limits.max_storage_bytes {
        return Err(TenantQuotaExceeded {
            kind: TenantQuotaKind::StorageBytes,
            usage: storage_bytes,
            limit: limits.max_storage_bytes,
        }
        .into());
    }

    Ok(())
}
