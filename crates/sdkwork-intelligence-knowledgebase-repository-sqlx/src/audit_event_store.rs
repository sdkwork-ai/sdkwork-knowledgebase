use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use serde_json::Value;
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use sdkwork_utils_rust::is_blank;

use crate::db::sql_timestamp::{utc_sql_timestamp_text, SqlTimestampDialect};
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const INITIAL_VERSION: i64 = 0;
const MAX_AUDIT_EVENT_TYPE_CHARS: usize = 128;
const MAX_AUDIT_ACTOR_TYPE_CHARS: usize = 64;
const MAX_AUDIT_ACTOR_ID_CHARS: usize = 128;
const MAX_AUDIT_RESOURCE_TYPE_CHARS: usize = 64;
const MAX_AUDIT_RESULT_CHARS: usize = 64;
const MAX_AUDIT_REQUEST_ID_CHARS: usize = 64;
const MAX_AUDIT_TRACE_ID_CHARS: usize = 128;
const MAX_AUDIT_UUID_CHARS: usize = 64;
const MAX_AUDIT_CREATED_AT_CHARS: usize = 64;
const MAX_AUDIT_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_AUDIT_EXPORT_EVENTS: u32 = 5_000;
const MAX_AUDIT_LEGAL_HOLD_REASON_CHARS: usize = 512;
const MAX_AUDIT_LEGAL_HOLD_CASE_REFERENCE_CHARS: usize = 128;
const MAX_AUDIT_LEGAL_HOLD_SUBJECT_ID_CHARS: usize = 128;
const MAX_AUDIT_LEGAL_HOLD_ACTOR_CHARS: usize = 128;
const MAX_AUDIT_RETENTION_BATCH: u32 = 1_000;
const AUDIT_LEGAL_HOLD_SUBJECT_TYPES: &[&str] = &["actor", "resource"];
const AUDIT_LEGAL_HOLD_ACTIVE_STATUS: i64 = 1;
const AUDIT_LEGAL_HOLD_RELEASED_STATUS: i64 = 0;

#[derive(Debug, Clone)]
pub struct KnowledgeAuditEventRecord {
    pub id: Option<i64>,
    pub uuid: Option<String>,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: String,
    pub resource_type: String,
    pub resource_id: Option<u64>,
    pub result: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub payload: Option<Value>,
    pub created_at: Option<String>,
}

/// An active or released subject-scoped legal hold over audit events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeAuditLegalHold {
    pub hold_id: u64,
    pub uuid: String,
    pub tenant_id: u64,
    pub organization_id: u64,
    /// `actor` holds every event with this actor id; `resource` holds events for this resource id.
    pub subject_type: String,
    pub subject_id: String,
    pub reason: String,
    pub case_reference: Option<String>,
    pub status: i64,
    pub created_by: String,
    pub released_by: Option<String>,
    pub released_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateKnowledgeAuditLegalHold {
    pub subject_type: String,
    pub subject_id: String,
    pub reason: String,
    pub case_reference: Option<String>,
    pub created_by: String,
}

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeAuditEventStoreError {
    #[error("invalid audit event: {0}")]
    InvalidRequest(String),
    #[error("audit event not found: {0}")]
    NotFound(String),
    #[error("audit event conflict: {0}")]
    Conflict(String),
    #[error("audit export exceeds the maximum of {max_events} events")]
    ExportLimitExceeded { max_events: u32 },
    #[error("audit event data integrity error: {0}")]
    DataIntegrity(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("id generation error: {0}")]
    IdGeneration(String),
}

#[async_trait]
pub trait KnowledgeAuditEventStore: Send + Sync {
    async fn record(
        &self,
        event: KnowledgeAuditEventRecord,
    ) -> Result<(), KnowledgeAuditEventStoreError>;
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeAuditEventStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeAuditEventStore {
    pub fn new(pool: AnyPool, tenant_id: u64, organization_id: u64) -> Self {
        Self::with_id_generator(
            pool,
            tenant_id,
            organization_id,
            default_knowledge_id_generator(),
        )
    }

    pub fn with_id_generator(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        id_generator: Arc<dyn KnowledgeIdGenerator>,
    ) -> Self {
        Self {
            pool,
            tenant_id,
            organization_id,
            id_generator,
            timestamp_dialect: SqlTimestampDialect::default(),
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }

    pub async fn append_event(
        &self,
        event: KnowledgeAuditEventRecord,
    ) -> Result<(), KnowledgeAuditEventStoreError> {
        validate_required_audit_text("event_type", &event.event_type, MAX_AUDIT_EVENT_TYPE_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_required_audit_text("actor_type", &event.actor_type, MAX_AUDIT_ACTOR_TYPE_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_required_audit_text("actor_id", &event.actor_id, MAX_AUDIT_ACTOR_ID_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_required_audit_text(
            "resource_type",
            &event.resource_type,
            MAX_AUDIT_RESOURCE_TYPE_CHARS,
        )
        .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_required_audit_text("result", &event.result, MAX_AUDIT_RESULT_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_optional_audit_text(
            "request_id",
            event.request_id.as_deref(),
            MAX_AUDIT_REQUEST_ID_CHARS,
        )
        .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_optional_audit_text(
            "trace_id",
            event.trace_id.as_deref(),
            MAX_AUDIT_TRACE_ID_CHARS,
        )
        .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;

        let id = next_i64_id(&self.id_generator)
            .map_err(|error| KnowledgeAuditEventStoreError::IdGeneration(error.to_string()))?;
        let tenant_id = i64::try_from(self.tenant_id)
            .map_err(|_| KnowledgeAuditEventStoreError::InvalidRequest("tenant_id".to_string()))?;
        let organization_id = i64::try_from(self.organization_id).map_err(|_| {
            KnowledgeAuditEventStoreError::InvalidRequest("organization_id".to_string())
        })?;
        let resource_id = event
            .resource_id
            .map(i64::try_from)
            .transpose()
            .map_err(|_| {
                KnowledgeAuditEventStoreError::InvalidRequest("resource_id".to_string())
            })?;
        let payload = event.payload.as_ref().map(ToString::to_string);
        if payload
            .as_ref()
            .is_some_and(|value| value.len() > MAX_AUDIT_PAYLOAD_BYTES)
        {
            return Err(KnowledgeAuditEventStoreError::InvalidRequest(format!(
                "payload exceeds {MAX_AUDIT_PAYLOAD_BYTES} bytes"
            )));
        }
        let now =
            utc_sql_timestamp_text().map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;

        let payload_expr = self.timestamp_dialect.sql_json_expr("$13");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$14");
        let query = format!(
            r#"
            INSERT INTO kb_audit_event (
                id, uuid, tenant_id, organization_id, event_type, actor_type, actor_id,
                resource_type, resource_id, result, request_id, trace_id,
                payload, created_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {payload_expr}, {created_at_expr}, $15)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(event.event_type)
            .bind(event.actor_type)
            .bind(event.actor_id)
            .bind(event.resource_type)
            .bind(resource_id)
            .bind(event.result)
            .bind(event.request_id)
            .bind(event.trace_id)
            .bind(payload)
            .bind(now)
            .bind(INITIAL_VERSION)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_events_by_actor(
        &self,
        actor_id: &str,
        limit: u32,
    ) -> Result<Vec<KnowledgeAuditEventRecord>, KnowledgeAuditEventStoreError> {
        validate_required_audit_text("actor_id", actor_id, MAX_AUDIT_ACTOR_ID_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        if !(1..=MAX_AUDIT_EXPORT_EVENTS).contains(&limit) {
            return Err(KnowledgeAuditEventStoreError::InvalidRequest(format!(
                "limit must be between 1 and {MAX_AUDIT_EXPORT_EVENTS}"
            )));
        }
        let tenant_id = i64::try_from(self.tenant_id)
            .map_err(|_| KnowledgeAuditEventStoreError::InvalidRequest("tenant_id".to_string()))?;
        let organization_id = i64::try_from(self.organization_id).map_err(|_| {
            KnowledgeAuditEventStoreError::InvalidRequest("organization_id".to_string())
        })?;
        let requested_limit = usize::try_from(limit).map_err(|_| {
            KnowledgeAuditEventStoreError::InvalidRequest("limit is unsupported".to_string())
        })?;
        let query_limit = i64::from(limit) + 1;
        let created_at_expr = self.timestamp_dialect.sql_timestamp_text_expr("created_at");
        let query = format!(
            r#"
            SELECT id, uuid, event_type, actor_type, actor_id, resource_type, resource_id,
                   result, request_id, trace_id, {created_at_expr} AS created_at
            FROM kb_audit_event
            WHERE tenant_id = $1 AND organization_id = $2 AND actor_id = $3
            ORDER BY created_at DESC, id DESC
            LIMIT $4
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(actor_id)
            .bind(query_limit)
            .fetch_all(&self.pool)
            .await?;

        if rows.len() > requested_limit {
            return Err(KnowledgeAuditEventStoreError::ExportLimitExceeded { max_events: limit });
        }

        rows.into_iter()
            .map(|row| {
                let id = row
                    .try_get::<i64, _>("id")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                if id <= 0 {
                    return Err(KnowledgeAuditEventStoreError::DataIntegrity(
                        "id must be positive".to_string(),
                    ));
                }
                let uuid = row
                    .try_get::<String, _>("uuid")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text("uuid", &uuid, MAX_AUDIT_UUID_CHARS)
                    .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let created_at = row
                    .try_get::<String, _>("created_at")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text("created_at", &created_at, MAX_AUDIT_CREATED_AT_CHARS)
                    .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let event_type = row
                    .try_get::<String, _>("event_type")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text("event_type", &event_type, MAX_AUDIT_EVENT_TYPE_CHARS)
                    .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let actor_type = row
                    .try_get::<String, _>("actor_type")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text("actor_type", &actor_type, MAX_AUDIT_ACTOR_TYPE_CHARS)
                    .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let decoded_actor_id = row
                    .try_get::<String, _>("actor_id")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text(
                    "actor_id",
                    &decoded_actor_id,
                    MAX_AUDIT_ACTOR_ID_CHARS,
                )
                .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let resource_type = row
                    .try_get::<String, _>("resource_type")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text(
                    "resource_type",
                    &resource_type,
                    MAX_AUDIT_RESOURCE_TYPE_CHARS,
                )
                .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let result = row
                    .try_get::<String, _>("result")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_required_audit_text("result", &result, MAX_AUDIT_RESULT_CHARS)
                    .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let request_id = row
                    .try_get::<Option<String>, _>("request_id")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_optional_audit_text(
                    "request_id",
                    request_id.as_deref(),
                    MAX_AUDIT_REQUEST_ID_CHARS,
                )
                .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let trace_id = row
                    .try_get::<Option<String>, _>("trace_id")
                    .map_err(KnowledgeAuditEventStoreError::Database)?;
                validate_optional_audit_text(
                    "trace_id",
                    trace_id.as_deref(),
                    MAX_AUDIT_TRACE_ID_CHARS,
                )
                .map_err(KnowledgeAuditEventStoreError::DataIntegrity)?;
                let resource_id = row
                    .try_get::<Option<i64>, _>("resource_id")
                    .map_err(KnowledgeAuditEventStoreError::Database)?
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| {
                        KnowledgeAuditEventStoreError::DataIntegrity(
                            "resource_id must not be negative".to_string(),
                        )
                    })?;
                Ok(KnowledgeAuditEventRecord {
                    id: Some(id),
                    uuid: Some(uuid),
                    event_type,
                    actor_type,
                    actor_id: decoded_actor_id,
                    resource_type,
                    resource_id,
                    result,
                    request_id,
                    trace_id,
                    payload: None,
                    created_at: Some(created_at),
                })
            })
            .collect()
    }

    pub async fn anonymize_actor(
        &self,
        actor_id: &str,
    ) -> Result<u64, KnowledgeAuditEventStoreError> {
        validate_required_audit_text("actor_id", actor_id, MAX_AUDIT_ACTOR_ID_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        let tenant_id = i64::try_from(self.tenant_id)
            .map_err(|_| KnowledgeAuditEventStoreError::InvalidRequest("tenant_id".to_string()))?;
        let organization_id = i64::try_from(self.organization_id).map_err(|_| {
            KnowledgeAuditEventStoreError::InvalidRequest("organization_id".to_string())
        })?;
        let result = sqlx::query(
            r#"
            UPDATE kb_audit_event
            SET actor_id = 'gdpr-redacted', actor_type = 'system'
            WHERE tenant_id = $1 AND organization_id = $2 AND actor_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(actor_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Creates a subject-scoped legal hold. Held events are exempt from retention purge.
    pub async fn create_legal_hold(
        &self,
        request: CreateKnowledgeAuditLegalHold,
    ) -> Result<KnowledgeAuditLegalHold, KnowledgeAuditEventStoreError> {
        validate_legal_hold_subject(&request.subject_type, &request.subject_id)?;
        validate_required_audit_text("reason", &request.reason, MAX_AUDIT_LEGAL_HOLD_REASON_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_optional_audit_text(
            "case_reference",
            request.case_reference.as_deref(),
            MAX_AUDIT_LEGAL_HOLD_CASE_REFERENCE_CHARS,
        )
        .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        validate_required_audit_text(
            "created_by",
            &request.created_by,
            MAX_AUDIT_LEGAL_HOLD_ACTOR_CHARS,
        )
        .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;

        let id = next_i64_id(&self.id_generator)
            .map_err(|error| KnowledgeAuditEventStoreError::IdGeneration(error.to_string()))?;
        let tenant_id = audit_scope_i64("tenant_id", self.tenant_id)?;
        let organization_id = audit_scope_i64("organization_id", self.organization_id)?;
        let uuid = Uuid::new_v4().to_string();
        let now =
            utc_sql_timestamp_text().map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$11");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$12");
        let query = format!(
            r#"
            INSERT INTO kb_audit_legal_hold (
                id, uuid, tenant_id, organization_id, subject_type, subject_id,
                reason, case_reference, status, created_by, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, {created_at_expr}, {updated_at_expr}, $13)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(&uuid)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(&request.subject_type)
            .bind(&request.subject_id)
            .bind(&request.reason)
            .bind(request.case_reference.as_deref())
            .bind(AUDIT_LEGAL_HOLD_ACTIVE_STATUS)
            .bind(&request.created_by)
            .bind(now.clone())
            .bind(now.clone())
            .bind(INITIAL_VERSION)
            .execute(&self.pool)
            .await?;

        Ok(KnowledgeAuditLegalHold {
            hold_id: id as u64,
            uuid,
            tenant_id: self.tenant_id,
            organization_id: self.organization_id,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            reason: request.reason,
            case_reference: request.case_reference,
            status: AUDIT_LEGAL_HOLD_ACTIVE_STATUS,
            created_by: request.created_by,
            released_by: None,
            released_at: None,
            created_at: now,
        })
    }

    /// Releases an active legal hold. Releasing an already released or unknown hold is a
    /// conflict/not-found rather than a silent success.
    pub async fn release_legal_hold(
        &self,
        hold_id: u64,
        released_by: &str,
    ) -> Result<KnowledgeAuditLegalHold, KnowledgeAuditEventStoreError> {
        validate_required_audit_text("released_by", released_by, MAX_AUDIT_LEGAL_HOLD_ACTOR_CHARS)
            .map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        let tenant_id = audit_scope_i64("tenant_id", self.tenant_id)?;
        let organization_id = audit_scope_i64("organization_id", self.organization_id)?;
        let hold_id_value = audit_scope_i64("hold_id", hold_id)?;
        let now =
            utc_sql_timestamp_text().map_err(KnowledgeAuditEventStoreError::InvalidRequest)?;
        let released_at_expr = self.timestamp_dialect.sql_timestamp_expr("$5");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$6");
        let query = format!(
            r#"
            UPDATE kb_audit_legal_hold
            SET status = $4,
                released_by = $3,
                released_at = {released_at_expr},
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $7 AND status = $8
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(released_by)
            .bind(AUDIT_LEGAL_HOLD_RELEASED_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(hold_id_value)
            .bind(AUDIT_LEGAL_HOLD_ACTIVE_STATUS)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeAuditEventStoreError::Conflict(format!(
                "audit legal hold {hold_id} is not active or does not exist"
            )));
        }
        self.retrieve_legal_hold(hold_id).await
    }

    pub async fn retrieve_legal_hold(
        &self,
        hold_id: u64,
    ) -> Result<KnowledgeAuditLegalHold, KnowledgeAuditEventStoreError> {
        let tenant_id = audit_scope_i64("tenant_id", self.tenant_id)?;
        let organization_id = audit_scope_i64("organization_id", self.organization_id)?;
        let hold_id = audit_scope_i64("hold_id", hold_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, uuid, tenant_id, organization_id, subject_type, subject_id,
                   reason, case_reference, status, created_by, released_by,
                   TO_CHAR(released_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS released_at,
                   TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS created_at
            FROM kb_audit_legal_hold
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(hold_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            KnowledgeAuditEventStoreError::NotFound(format!("audit legal hold {hold_id}"))
        })?;
        legal_hold_from_row(&row)
    }

    pub async fn list_active_legal_holds(
        &self,
        limit: u32,
    ) -> Result<Vec<KnowledgeAuditLegalHold>, KnowledgeAuditEventStoreError> {
        if limit == 0 || limit > MAX_AUDIT_RETENTION_BATCH {
            return Err(KnowledgeAuditEventStoreError::InvalidRequest(format!(
                "legal hold limit must be between 1 and {MAX_AUDIT_RETENTION_BATCH}"
            )));
        }
        let tenant_id = audit_scope_i64("tenant_id", self.tenant_id)?;
        let organization_id = audit_scope_i64("organization_id", self.organization_id)?;
        let rows = sqlx::query(
            r#"
            SELECT id, uuid, tenant_id, organization_id, subject_type, subject_id,
                   reason, case_reference, status, created_by, released_by,
                   TO_CHAR(released_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS released_at,
                   TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS created_at
            FROM kb_audit_legal_hold
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
            ORDER BY created_at DESC, id DESC
            LIMIT $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(AUDIT_LEGAL_HOLD_ACTIVE_STATUS)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| legal_hold_from_row(&row))
            .collect()
    }

    /// Keyset page of active holds ordered by hold id. The extra row distinguishes a full
    /// last page from a page with more entries without a COUNT.
    pub async fn list_active_legal_holds_page(
        &self,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeAuditLegalHold>, Option<u64>), KnowledgeAuditEventStoreError> {
        if page_size == 0 || page_size > MAX_AUDIT_RETENTION_BATCH {
            return Err(KnowledgeAuditEventStoreError::InvalidRequest(format!(
                "page_size must be between 1 and {MAX_AUDIT_RETENTION_BATCH}"
            )));
        }
        let tenant_id = audit_scope_i64("tenant_id", self.tenant_id)?;
        let organization_id = audit_scope_i64("organization_id", self.organization_id)?;
        let after_id = cursor
            .map(|value| audit_scope_i64("cursor", value))
            .transpose()?
            .unwrap_or(0);
        let rows = sqlx::query(
            r#"
            SELECT id, uuid, tenant_id, organization_id, subject_type, subject_id,
                   reason, case_reference, status, created_by, released_by,
                   TO_CHAR(released_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS released_at,
                   TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS created_at
            FROM kb_audit_legal_hold
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3 AND id > $4
            ORDER BY id ASC
            LIMIT $5
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(AUDIT_LEGAL_HOLD_ACTIVE_STATUS)
        .bind(after_id)
        .bind(i64::from(page_size) + 1)
        .fetch_all(&self.pool)
        .await?;
        let mut holds = rows
            .into_iter()
            .map(|row| legal_hold_from_row(&row))
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = holds.len() > page_size as usize;
        if has_more {
            holds.truncate(page_size as usize);
        }
        let next_cursor = if has_more {
            holds.last().map(|hold| hold.hold_id)
        } else {
            None
        };
        Ok((holds, next_cursor))
    }

    /// Deletes at most `limit` audit events older than `cutoff`, skipping events whose actor or
    /// resource is under an active legal hold. The delete is a single statement so a crash cannot
    /// leave the batch half-selected; a rerun simply continues with the next oldest window.
    pub async fn purge_expired_events(
        &self,
        cutoff: &str,
        limit: u32,
    ) -> Result<u64, KnowledgeAuditEventStoreError> {
        if limit == 0 || limit > MAX_AUDIT_RETENTION_BATCH {
            return Err(KnowledgeAuditEventStoreError::InvalidRequest(format!(
                "purge limit must be between 1 and {MAX_AUDIT_RETENTION_BATCH}"
            )));
        }
        if is_blank(Some(cutoff)) {
            return Err(KnowledgeAuditEventStoreError::InvalidRequest(
                "cutoff is required".to_string(),
            ));
        }
        let tenant_id = audit_scope_i64("tenant_id", self.tenant_id)?;
        let organization_id = audit_scope_i64("organization_id", self.organization_id)?;
        let query = r#"
            DELETE FROM kb_audit_event event
            WHERE event.tenant_id = $1
              AND event.organization_id = $2
              AND event.created_at < CAST($3 AS TIMESTAMPTZ)
              AND event.id IN (
                  SELECT candidate.id
                  FROM kb_audit_event candidate
                  WHERE candidate.tenant_id = $1
                    AND candidate.organization_id = $2
                    AND candidate.created_at < CAST($3 AS TIMESTAMPTZ)
                  ORDER BY candidate.created_at ASC, candidate.id ASC
                  LIMIT $4
              )
              AND NOT EXISTS (
                  SELECT 1
                  FROM kb_audit_legal_hold hold
                  WHERE hold.tenant_id = event.tenant_id
                    AND hold.organization_id = event.organization_id
                    AND hold.status = $5
                    AND (
                        (hold.subject_type = 'actor' AND hold.subject_id = event.actor_id)
                        OR (
                            hold.subject_type = 'resource'
                            AND event.resource_id IS NOT NULL
                            AND hold.subject_id = event.resource_id::text
                        )
                    )
              )
        "#;
        let result = sqlx::query(sqlx::AssertSqlSafe(query))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(cutoff)
            .bind(i64::from(limit))
            .bind(AUDIT_LEGAL_HOLD_ACTIVE_STATUS)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

fn audit_scope_i64(field: &str, value: u64) -> Result<i64, KnowledgeAuditEventStoreError> {
    i64::try_from(value)
        .map_err(|_| KnowledgeAuditEventStoreError::InvalidRequest(field.to_string()))
}

fn validate_legal_hold_subject(
    subject_type: &str,
    subject_id: &str,
) -> Result<(), KnowledgeAuditEventStoreError> {
    if !AUDIT_LEGAL_HOLD_SUBJECT_TYPES.contains(&subject_type) {
        return Err(KnowledgeAuditEventStoreError::InvalidRequest(format!(
            "subject_type must be one of {AUDIT_LEGAL_HOLD_SUBJECT_TYPES:?}"
        )));
    }
    validate_required_audit_text(
        "subject_id",
        subject_id,
        MAX_AUDIT_LEGAL_HOLD_SUBJECT_ID_CHARS,
    )
    .map_err(KnowledgeAuditEventStoreError::InvalidRequest)
}

fn legal_hold_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<KnowledgeAuditLegalHold, KnowledgeAuditEventStoreError> {
    let hold_id: i64 = row
        .try_get("id")
        .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?;
    Ok(KnowledgeAuditLegalHold {
        hold_id: u64::try_from(hold_id).map_err(|_| {
            KnowledgeAuditEventStoreError::DataIntegrity("negative hold id".to_string())
        })?,
        uuid: row
            .try_get("uuid")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        tenant_id: u64::try_from(
            row.try_get::<i64, _>("tenant_id")
                .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        )
        .map_err(|_| {
            KnowledgeAuditEventStoreError::DataIntegrity("negative tenant id".to_string())
        })?,
        organization_id: u64::try_from(
            row.try_get::<i64, _>("organization_id")
                .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        )
        .map_err(|_| {
            KnowledgeAuditEventStoreError::DataIntegrity("negative organization id".to_string())
        })?,
        subject_type: row
            .try_get("subject_type")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        subject_id: row
            .try_get("subject_id")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        reason: row
            .try_get("reason")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        case_reference: row
            .try_get("case_reference")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        status: row
            .try_get("status")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        created_by: row
            .try_get("created_by")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        released_by: row
            .try_get("released_by")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        released_at: row
            .try_get("released_at")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|error| KnowledgeAuditEventStoreError::DataIntegrity(error.to_string()))?,
    })
}

fn validate_required_audit_text(field: &str, value: &str, max_chars: usize) -> Result<(), String> {
    if is_blank(Some(value)) {
        return Err(format!("{field} is required"));
    }
    if value.chars().nth(max_chars).is_some() {
        return Err(format!("{field} exceeds {max_chars} characters"));
    }
    Ok(())
}

fn validate_optional_audit_text(
    field: &str,
    value: Option<&str>,
    max_chars: usize,
) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.chars().nth(max_chars).is_some() {
        return Err(format!("{field} exceeds {max_chars} characters"));
    }
    Ok(())
}

#[async_trait]
impl KnowledgeAuditEventStore for PostgresKnowledgeAuditEventStore {
    async fn record(
        &self,
        event: KnowledgeAuditEventRecord,
    ) -> Result<(), KnowledgeAuditEventStoreError> {
        self.append_event(event).await
    }
}
