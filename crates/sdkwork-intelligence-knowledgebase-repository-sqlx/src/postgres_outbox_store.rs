use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_store::{
    AppendOutboxEventRecord, ClaimedOutboxEvent, KnowledgeOutboxStore, KnowledgeOutboxStoreError,
    OutboxClaim, OutboxRequeueResult, PendingOutboxEvent, MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES,
};
use sdkwork_utils_rust::{is_blank, truncate};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const OUTBOX_STATUS_PENDING: i64 = 0;
const OUTBOX_STATUS_PUBLISHED: i64 = 1;
const OUTBOX_STATUS_FAILED: i64 = 2;
const OUTBOX_STATUS_CLAIMED: i64 = 3;
const OUTBOX_STATUS_DEAD_LETTER: i64 = 4;
const INITIAL_VERSION: i64 = 0;
const DEFAULT_STALE_CLAIM_SECS: u64 = 300;
const MAX_CLAIM_OWNER_BYTES: usize = 128;

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeOutboxStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    claim_owner: String,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    use_postgres_skip_locked_claim: bool,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeOutboxStore {
    pub fn new(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        claim_owner: impl Into<String>,
    ) -> Self {
        Self::with_id_generator(
            pool,
            tenant_id,
            organization_id,
            claim_owner,
            default_knowledge_id_generator(),
        )
    }

    pub fn with_postgres_skip_locked_claim(mut self, enabled: bool) -> Self {
        self.use_postgres_skip_locked_claim = enabled;
        self
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }

    pub fn with_id_generator(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        claim_owner: impl Into<String>,
        id_generator: Arc<dyn KnowledgeIdGenerator>,
    ) -> Self {
        Self {
            pool,
            tenant_id,
            organization_id,
            claim_owner: claim_owner.into(),
            id_generator,
            use_postgres_skip_locked_claim: false,
            timestamp_dialect: SqlTimestampDialect::default(),
        }
    }
}

#[async_trait]
impl KnowledgeOutboxStore for PostgresKnowledgeOutboxStore {
    async fn append_event(
        &self,
        record: AppendOutboxEventRecord,
    ) -> Result<(), KnowledgeOutboxStoreError> {
        if is_blank(Some(record.aggregate_type.as_str())) {
            return Err(KnowledgeOutboxStoreError::InvalidRequest(
                "aggregate_type is required".to_string(),
            ));
        }
        if is_blank(Some(record.event_type.as_str())) {
            return Err(KnowledgeOutboxStoreError::InvalidRequest(
                "event_type is required".to_string(),
            ));
        }
        if is_blank(Some(record.payload_json.as_str())) {
            return Err(KnowledgeOutboxStoreError::InvalidRequest(
                "payload_json is required".to_string(),
            ));
        }
        if record.payload_json.len() > MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES {
            return Err(KnowledgeOutboxStoreError::InvalidRequest(format!(
                "payload_json exceeds {MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES} bytes"
            )));
        }
        serde_json::from_str::<serde_json::Value>(&record.payload_json).map_err(|_| {
            KnowledgeOutboxStoreError::InvalidRequest(
                "payload_json must contain valid JSON".to_string(),
            )
        })?;

        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let aggregate_id = to_i64("aggregate_id", record.aggregate_id)?;
        let now = now_rfc3339()?;
        let payload_expr = self.timestamp_dialect.sql_json_expr("$8");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$10");

        let query = format!(
            r#"
            INSERT INTO kb_outbox_event (
                id, uuid, tenant_id, organization_id, aggregate_type, aggregate_id, event_type,
                payload, status, created_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, {payload_expr}, $9, {created_at_expr}, $11)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(record.aggregate_type)
            .bind(aggregate_id)
            .bind(record.event_type)
            .bind(record.payload_json)
            .bind(OUTBOX_STATUS_PENDING)
            .bind(now)
            .bind(INITIAL_VERSION)
            .execute(&self.pool)
            .await
            .map_err(sqlx_error)?;

        Ok(())
    }

    async fn list_pending_events(
        &self,
        limit: u32,
    ) -> Result<Vec<PendingOutboxEvent>, KnowledgeOutboxStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 200));
        let rows = sqlx::query(
            r#"
            SELECT id, uuid, aggregate_type, aggregate_id, event_type, retry_count,
                   CAST(payload AS TEXT) AS payload
            FROM kb_outbox_event
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
            ORDER BY created_at ASC, id ASC
            LIMIT $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(OUTBOX_STATUS_PENDING)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(PendingOutboxEvent {
                    id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
                    event_uuid: row.try_get("uuid").map_err(sqlx_error)?,
                    aggregate_type: row.try_get("aggregate_type").map_err(sqlx_error)?,
                    aggregate_id: from_i64(
                        "aggregate_id",
                        row.try_get("aggregate_id").map_err(sqlx_error)?,
                    )?,
                    event_type: row.try_get("event_type").map_err(sqlx_error)?,
                    retry_count: from_i64_u32(
                        "retry_count",
                        row.try_get("retry_count").map_err(sqlx_error)?,
                    )?,
                    payload_json: row.try_get("payload").map_err(sqlx_error)?,
                })
            })
            .collect()
    }

    async fn claim_pending_events(
        &self,
        limit: u32,
    ) -> Result<Vec<ClaimedOutboxEvent>, KnowledgeOutboxStoreError> {
        let _ = self
            .release_stale_claimed_events(DEFAULT_STALE_CLAIM_SECS)
            .await?;

        validate_claim_owner(&self.claim_owner)?;
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 200));
        let now = now_rfc3339()?;
        let claim_token = Uuid::new_v4().to_string();
        let claimed_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");

        let rows = if self.use_postgres_skip_locked_claim {
            let query = format!(
                r#"
                UPDATE kb_outbox_event
                SET status = $1, claimed_at = {claimed_at_expr}, claim_owner = $3,
                    claim_token = $4, dead_lettered_at = NULL, version = version + 1
                WHERE id IN (
                    SELECT id
                    FROM kb_outbox_event
                    WHERE tenant_id = $5 AND organization_id = $6 AND status = $7
                    ORDER BY created_at ASC, id ASC
                    LIMIT $8
                    FOR UPDATE SKIP LOCKED
                )
                RETURNING id, uuid, aggregate_type, aggregate_id, event_type, retry_count,
                          CAST(payload AS TEXT) AS payload
                "#,
            );
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(OUTBOX_STATUS_CLAIMED)
                .bind(&now)
                .bind(&self.claim_owner)
                .bind(&claim_token)
                .bind(tenant_id)
                .bind(organization_id)
                .bind(OUTBOX_STATUS_PENDING)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(sqlx_error)?
        } else {
            let query = format!(
                r#"
                UPDATE kb_outbox_event
                SET status = $1, claimed_at = {claimed_at_expr}, claim_owner = $3,
                    claim_token = $4, dead_lettered_at = NULL, version = version + 1
                WHERE id IN (
                    SELECT id
                    FROM kb_outbox_event
                    WHERE tenant_id = $5 AND organization_id = $6 AND status = $7
                    ORDER BY created_at ASC, id ASC
                    LIMIT $8
                )
                RETURNING id, uuid, aggregate_type, aggregate_id, event_type, retry_count,
                          CAST(payload AS TEXT) AS payload
                "#,
            );
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(OUTBOX_STATUS_CLAIMED)
                .bind(&now)
                .bind(&self.claim_owner)
                .bind(&claim_token)
                .bind(tenant_id)
                .bind(organization_id)
                .bind(OUTBOX_STATUS_PENDING)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(sqlx_error)?
        };

        let claim = OutboxClaim {
            owner: self.claim_owner.clone(),
            token: claim_token,
        };
        rows.into_iter()
            .map(|row| {
                Ok(ClaimedOutboxEvent {
                    event: pending_event_from_row(&row)?,
                    claim: claim.clone(),
                })
            })
            .collect()
    }

    async fn release_stale_claimed_events(
        &self,
        stale_after_secs: u64,
    ) -> Result<usize, KnowledgeOutboxStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let cutoff = OffsetDateTime::now_utc()
            - time::Duration::seconds(i64::try_from(stale_after_secs).unwrap_or(300));
        let cutoff = cutoff
            .format(&Rfc3339)
            .map_err(|error| KnowledgeOutboxStoreError::Internal(error.to_string()))?;
        let cutoff_expr = self.timestamp_dialect.sql_timestamp_expr("$5");

        let query = format!(
            r#"
            UPDATE kb_outbox_event
            SET status = $1, claimed_at = NULL, claim_owner = NULL, claim_token = NULL,
                version = version + 1
            WHERE tenant_id = $2
              AND organization_id = $3
              AND status = $4
              AND claimed_at IS NOT NULL
              AND claimed_at < {cutoff_expr}
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(OUTBOX_STATUS_PENDING)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(OUTBOX_STATUS_CLAIMED)
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(sqlx_error)?;

        Ok(updated.rows_affected() as usize)
    }

    async fn mark_published(
        &self,
        claimed: &ClaimedOutboxEvent,
    ) -> Result<(), KnowledgeOutboxStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let event_id = to_i64("event_id", claimed.event.id)?;
        let now = now_rfc3339()?;
        let published_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_outbox_event
            SET status = $1, published_at = {published_at_expr}, claimed_at = NULL,
                claim_owner = NULL, claim_token = NULL, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
              AND claim_owner = $7 AND claim_token = $8
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(OUTBOX_STATUS_PUBLISHED)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(event_id)
            .bind(OUTBOX_STATUS_CLAIMED)
            .bind(&claimed.claim.owner)
            .bind(&claimed.claim.token)
            .execute(&self.pool)
            .await
            .map_err(sqlx_error)?;

        if updated.rows_affected() == 0 {
            return Err(KnowledgeOutboxStoreError::InvalidRequest(format!(
                "outbox event claim is stale or invalid: {event_id}"
            )));
        }
        Ok(())
    }

    async fn mark_failed(
        &self,
        claimed: &ClaimedOutboxEvent,
        error_message: &str,
    ) -> Result<(), KnowledgeOutboxStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let event_id = to_i64("event_id", claimed.event.id)?;
        let truncated_error = truncate_outbox_error(error_message);
        // Schedule the next attempt with exponential backoff based on the NEW
        // retry count so dead webhooks are not hammered every poll interval.
        let new_retry_count = claimed.event.retry_count.saturating_add(1);
        let next_attempt_at = OffsetDateTime::now_utc()
            .checked_add(time::Duration::seconds(outbox_retry_backoff_seconds(
                new_retry_count,
            )))
            .ok_or_else(|| {
                KnowledgeOutboxStoreError::Internal(
                    "outbox retry backoff timestamp overflow".to_string(),
                )
            })?
            .format(&Rfc3339)
            .map_err(|error| {
                KnowledgeOutboxStoreError::Internal(format!(
                    "outbox retry backoff timestamp format error: {error}"
                ))
            })?;
        let updated = sqlx::query(
            r#"
            UPDATE kb_outbox_event
            SET status = $1,
                last_error = $2,
                claimed_at = NULL,
                claim_owner = NULL,
                claim_token = NULL,
                retry_count = retry_count + 1,
                next_attempt_at = $9,
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
              AND claim_owner = $7 AND claim_token = $8
            "#,
        )
        .bind(OUTBOX_STATUS_FAILED)
        .bind(truncated_error)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(event_id)
        .bind(OUTBOX_STATUS_CLAIMED)
        .bind(&claimed.claim.owner)
        .bind(&claimed.claim.token)
        .bind(next_attempt_at)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error)?;

        if updated.rows_affected() == 0 {
            return Err(KnowledgeOutboxStoreError::InvalidRequest(format!(
                "outbox event claim is stale or invalid: {event_id}"
            )));
        }
        Ok(())
    }

    async fn requeue_failed_events(
        &self,
        limit: u32,
        max_retry_count: u32,
    ) -> Result<OutboxRequeueResult, KnowledgeOutboxStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 200));
        let max_retry_count = i64::from(max_retry_count);
        let now = now_rfc3339()?;
        let dead_lettered_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let mut transaction = self.pool.begin().await.map_err(sqlx_error)?;
        let dead_letter_query = format!(
            r#"
            UPDATE kb_outbox_event
            SET status = $1, dead_lettered_at = {dead_lettered_at_expr}, version = version + 1
            WHERE tenant_id = $3
              AND organization_id = $4
              AND status = $5
              AND retry_count >= $6
              AND id IN (
                SELECT id
                FROM kb_outbox_event
                WHERE tenant_id = $3
                  AND organization_id = $4
                  AND status = $5
                  AND retry_count >= $6
                ORDER BY created_at ASC, id ASC
                LIMIT $7
              )
            "#,
        );
        let dead_lettered = sqlx::query(sqlx::AssertSqlSafe(dead_letter_query.as_str()))
            .bind(OUTBOX_STATUS_DEAD_LETTER)
            .bind(now.clone())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(OUTBOX_STATUS_FAILED)
            .bind(max_retry_count)
            .bind(limit)
            .execute(&mut *transaction)
            .await
            .map_err(sqlx_error)?
            .rows_affected() as usize;

        // Requeue only failed events whose backoff window has elapsed
        // (next_attempt_at is NULL or due); future-dated events stay FAILED
        // until their scheduled retry time.
        let next_attempt_due_expr = self.timestamp_dialect.sql_timestamp_expr("$7");
        let requeue_query = format!(
            r#"
            UPDATE kb_outbox_event
            SET status = $1, dead_lettered_at = NULL, next_attempt_at = NULL, version = version + 1
            WHERE tenant_id = $2
              AND organization_id = $3
              AND status = $4
              AND retry_count < $5
              AND (next_attempt_at IS NULL OR next_attempt_at <= {next_attempt_due_expr})
              AND id IN (
                SELECT id
                FROM kb_outbox_event
                WHERE tenant_id = $2
                  AND organization_id = $3
                  AND status = $4
                  AND retry_count < $5
                  AND (next_attempt_at IS NULL OR next_attempt_at <= {next_attempt_due_expr})
                ORDER BY created_at ASC, id ASC
                LIMIT $6
              )
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(requeue_query.as_str()))
            .bind(OUTBOX_STATUS_PENDING)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(OUTBOX_STATUS_FAILED)
            .bind(max_retry_count)
            .bind(limit)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(sqlx_error)?;
        transaction.commit().await.map_err(sqlx_error)?;

        Ok(OutboxRequeueResult {
            requeued: updated.rows_affected() as usize,
            dead_lettered,
        })
    }
}

/// Exponential retry backoff for outbox delivery: 30s, 60s, 120s, ... capped at
/// one hour, based on the attempt count.
fn outbox_retry_backoff_seconds(retry_count: u32) -> i64 {
    let exponent = i64::from(retry_count.saturating_sub(1)).min(7);
    let backoff = 30i64.saturating_mul(1i64 << exponent);
    backoff.clamp(30, 3_600)
}

fn pending_event_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<PendingOutboxEvent, KnowledgeOutboxStoreError> {
    Ok(PendingOutboxEvent {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        event_uuid: row.try_get("uuid").map_err(sqlx_error)?,
        aggregate_type: row.try_get("aggregate_type").map_err(sqlx_error)?,
        aggregate_id: from_i64(
            "aggregate_id",
            row.try_get("aggregate_id").map_err(sqlx_error)?,
        )?,
        event_type: row.try_get("event_type").map_err(sqlx_error)?,
        retry_count: from_i64_u32(
            "retry_count",
            row.try_get("retry_count").map_err(sqlx_error)?,
        )?,
        payload_json: row.try_get("payload").map_err(sqlx_error)?,
    })
}

fn validate_claim_owner(owner: &str) -> Result<(), KnowledgeOutboxStoreError> {
    if is_blank(Some(owner)) || owner.len() > MAX_CLAIM_OWNER_BYTES {
        return Err(KnowledgeOutboxStoreError::InvalidRequest(format!(
            "claim_owner must contain 1 to {MAX_CLAIM_OWNER_BYTES} bytes"
        )));
    }
    Ok(())
}

fn truncate_outbox_error(error_message: &str) -> String {
    const MAX_OUTBOX_ERROR_LEN: usize = 1024;
    truncate(error_message, MAX_OUTBOX_ERROR_LEN, Some(""))
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeOutboxStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeOutboxStoreError::InvalidRequest(format!("{field} exceeds i64 range: {value}"))
    })
}

fn from_i64(field: &str, value: i64) -> Result<u64, KnowledgeOutboxStoreError> {
    u64::try_from(value).map_err(|_| {
        KnowledgeOutboxStoreError::Internal(format!(
            "persisted {field} must be a non-negative integer"
        ))
    })
}

fn from_i64_u32(field: &str, value: i64) -> Result<u32, KnowledgeOutboxStoreError> {
    u32::try_from(value).map_err(|_| {
        KnowledgeOutboxStoreError::Internal(format!(
            "persisted {field} must be a non-negative 32-bit integer"
        ))
    })
}

fn now_rfc3339() -> Result<String, KnowledgeOutboxStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeOutboxStoreError::Internal(error.to_string()))
}

fn id_error(error: crate::id::KnowledgeIdGeneratorError) -> KnowledgeOutboxStoreError {
    KnowledgeOutboxStoreError::Internal(error.to_string())
}

fn sqlx_error(error: sqlx::Error) -> KnowledgeOutboxStoreError {
    KnowledgeOutboxStoreError::Internal(error.to_string())
}

#[cfg(test)]
mod truncation_tests {
    use super::truncate_outbox_error;

    #[test]
    fn truncates_unicode_error_without_splitting_utf8() {
        let message = "上游错误".repeat(400);
        let truncated = truncate_outbox_error(&message);

        assert_eq!(truncated.chars().count(), 1024);
        assert!(message.starts_with(&truncated));
    }
}
