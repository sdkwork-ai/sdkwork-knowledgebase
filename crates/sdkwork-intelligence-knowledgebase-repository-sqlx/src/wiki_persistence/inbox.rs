use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::{
    ApplyWikiDriveEventRequest, ClaimWikiDriveEventsRequest, CompleteWikiDriveEventRequest,
    ReceiveWikiDriveEventRequest, RetryWikiDriveEventRequest, WikiDriveEventApplicationResult,
    WikiDriveEventInboxStore, WikiDriveEventProcessingState, WikiDriveEventReceipt,
    WikiDriveEventReceiveDisposition, WikiDriveInboxEvent, WikiDriveProjectionMutation,
    WikiPersistenceError, WikiPersistenceScope,
};
use sdkwork_utils_rust::{sha256_hash, uuid};
use sqlx::{any::AnyRow, Row};

use super::checkpoint::{checkpoint_from_row, CHECKPOINT_COLUMNS};
use super::{
    claim_limit, from_i32, from_i64, lease_times, new_lease_token, parse_enum, require_id,
    require_sha256, require_text, retry_time, row_error, sql_error, to_i64, validate_scope,
    SqlxWikiPersistenceStore,
};

pub(super) const INBOX_COLUMNS: &str = r#"
    id, uuid, tenant_id, organization_id, site_publication_id, checkpoint_id,
    source_event_id, event_type, sequence_no, drive_node_uuid, drive_version_uuid,
    payload_sha256, CAST(payload_json AS TEXT) AS payload_json,
    TO_CHAR(source_event_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS source_event_time,
    processing_state, attempt_count, lease_token, version
"#;

#[async_trait]
impl WikiDriveEventInboxStore for SqlxWikiPersistenceStore {
    async fn receive_event(
        &self,
        request: ReceiveWikiDriveEventRequest,
    ) -> Result<WikiDriveEventReceipt, WikiPersistenceError> {
        validate_receive_request(&request)?;
        let expected_hash = format!("sha256:{}", sha256_hash(request.payload_json.as_bytes()));
        if request.payload_sha256 != expected_hash {
            return Err(WikiPersistenceError::InvalidRequest(
                "payload_sha256 does not match the exact payload_json bytes".to_string(),
            ));
        }

        let mut transaction = self.pool.begin().await.map_err(sql_error)?;
        let checkpoint_query = format!(
            "SELECT {CHECKPOINT_COLUMNS} FROM kb_drive_source_checkpoint WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        );
        let checkpoint_row = sqlx::query(sqlx::AssertSqlSafe(checkpoint_query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("checkpoint_id", request.checkpoint_id)?)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(sql_error)?
            .ok_or(WikiPersistenceError::NotFound {
                resource: "wiki_drive_checkpoint",
                id: request.checkpoint_id,
            })?;
        let checkpoint = checkpoint_from_row(&checkpoint_row)?;
        if checkpoint.site_publication_id != request.site_publication_id {
            return Err(WikiPersistenceError::Conflict(
                "inbox event publication does not match its checkpoint".to_string(),
            ));
        }

        let existing_query = format!(
            r#"
            SELECT {INBOX_COLUMNS}
            FROM kb_drive_event_inbox
            WHERE tenant_id = $1 AND organization_id = $2 AND checkpoint_id = $3
              AND (source_event_id = $4 OR sequence_no = $5)
            LIMIT 1
            "#,
        );
        if let Some(existing_row) = sqlx::query(sqlx::AssertSqlSafe(existing_query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("checkpoint_id", request.checkpoint_id)?)
            .bind(request.source_event_id.trim())
            .bind(to_i64("sequence_no", request.sequence_no)?)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(sql_error)?
        {
            let existing = inbox_from_row(&existing_row)?;
            if event_matches_replay(&existing, &request) {
                transaction.commit().await.map_err(sql_error)?;
                return Ok(WikiDriveEventReceipt {
                    event: existing,
                    disposition: WikiDriveEventReceiveDisposition::Duplicate,
                });
            }
            return Err(WikiPersistenceError::Conflict(format!(
                "Drive event ID {} or sequence {} was reused with a different payload",
                request.source_event_id, request.sequence_no
            )));
        }

        let next_sequence = checkpoint
            .last_sequence_no
            .checked_add(1)
            .ok_or_else(|| WikiPersistenceError::Internal("Drive sequence overflow".to_string()))?;
        let (processing_state, disposition) = if request.sequence_no <= checkpoint.last_sequence_no
        {
            (
                WikiDriveEventProcessingState::Ignored,
                WikiDriveEventReceiveDisposition::IgnoredStale,
            )
        } else if request.sequence_no == next_sequence {
            (
                WikiDriveEventProcessingState::Received,
                WikiDriveEventReceiveDisposition::Ready,
            )
        } else {
            (
                WikiDriveEventProcessingState::Deferred,
                WikiDriveEventReceiveDisposition::DeferredGap,
            )
        };

        let now = super::now()?;
        let payload_json = self.dialect.sql_json_expr("$13");
        let source_event_time = self.dialect.sql_timestamp_expr("$14");
        let timestamp = self.dialect.sql_timestamp_expr("$16");
        let insert_query = format!(
            r#"
            INSERT INTO kb_drive_event_inbox (
                id, uuid, tenant_id, organization_id, site_publication_id, checkpoint_id,
                source_event_id, event_type, sequence_no, drive_node_uuid, drive_version_uuid,
                payload_sha256, payload_json, source_event_time, processing_state,
                received_at, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12, {payload_json}, {source_event_time}, $15,
                {timestamp}, {timestamp}, {timestamp}
            )
            RETURNING {INBOX_COLUMNS}
            "#,
        );
        let event_row = sqlx::query(sqlx::AssertSqlSafe(insert_query.as_str()))
            .bind(self.next_id()?)
            .bind(uuid())
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id(
                "site_publication_id",
                request.site_publication_id,
            )?)
            .bind(require_id("checkpoint_id", request.checkpoint_id)?)
            .bind(request.source_event_id.trim())
            .bind(request.event_type.as_str())
            .bind(to_i64("sequence_no", request.sequence_no)?)
            .bind(request.drive_node_uuid.trim())
            .bind(request.drive_version_uuid.as_deref().map(str::trim))
            .bind(request.payload_sha256.as_str())
            .bind(request.payload_json.as_str())
            .bind(request.source_event_time.as_str())
            .bind(processing_state.as_str())
            .bind(&now)
            .fetch_one(&mut *transaction)
            .await
            .map_err(sql_error)?;

        if disposition == WikiDriveEventReceiveDisposition::DeferredGap {
            let gap_from = checkpoint
                .gap_from_sequence_no
                .map_or(next_sequence, |existing| existing.min(next_sequence));
            let requested_gap_to = request.sequence_no - 1;
            let gap_to = checkpoint
                .gap_to_sequence_no
                .map_or(requested_gap_to, |existing| existing.max(requested_gap_to));
            let observed_at = self.dialect.sql_timestamp_expr("$7");
            let checkpoint_update = format!(
                r#"
                UPDATE kb_drive_source_checkpoint
                SET stream_state = CASE
                        WHEN stream_state = 'RECONCILING' THEN stream_state
                        ELSE 'GAP_DETECTED'
                    END,
                    gap_from_sequence_no = $4,
                    gap_to_sequence_no = $5,
                    last_observed_at = {observed_at},
                    updated_at = {observed_at},
                    version = version + 1
                WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
                  AND version = $6 AND status = 1
                "#,
            );
            let updated = sqlx::query(sqlx::AssertSqlSafe(checkpoint_update.as_str()))
                .bind(to_i64("tenant_id", request.scope.tenant_id)?)
                .bind(to_i64("organization_id", request.scope.organization_id)?)
                .bind(require_id("checkpoint_id", request.checkpoint_id)?)
                .bind(to_i64("gap_from_sequence_no", gap_from)?)
                .bind(to_i64("gap_to_sequence_no", gap_to)?)
                .bind(to_i64("checkpoint_version", checkpoint.version)?)
                .bind(&now)
                .execute(&mut *transaction)
                .await
                .map_err(sql_error)?;
            if updated.rows_affected() != 1 {
                return Err(WikiPersistenceError::Conflict(
                    "Drive checkpoint changed while recording a sequence gap".to_string(),
                ));
            }
        }

        transaction.commit().await.map_err(sql_error)?;
        Ok(WikiDriveEventReceipt {
            event: inbox_from_row(&event_row)?,
            disposition,
        })
    }

    async fn claim_events(
        &self,
        request: ClaimWikiDriveEventsRequest,
    ) -> Result<Vec<WikiDriveInboxEvent>, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let owner = require_text("claim_owner", &request.claim_owner, 128)?;
        let limit = claim_limit(request.limit)?;
        let (now, lease_expires_at) = lease_times(request.lease_seconds)?;
        let now_expr = self.dialect.sql_timestamp_expr("$5");
        let mut transaction = self.pool.begin().await.map_err(sql_error)?;
        let candidate_query = format!(
            r#"
            SELECT event.id, event.version
            FROM kb_drive_event_inbox event
            JOIN kb_drive_source_checkpoint checkpoint
              ON checkpoint.tenant_id = event.tenant_id
             AND checkpoint.organization_id = event.organization_id
             AND checkpoint.id = event.checkpoint_id
            WHERE event.tenant_id = $1 AND event.organization_id = $2
              AND event.checkpoint_id = $3 AND event.id > COALESCE($4, 0)
              AND event.sequence_no = checkpoint.last_sequence_no + 1
              AND event.processing_state IN ('RECEIVED', 'RETRY', 'DEFERRED')
              AND (event.next_retry_at IS NULL OR event.next_retry_at <= {now_expr})
              AND (event.lease_expires_at IS NULL OR event.lease_expires_at <= {now_expr})
            ORDER BY event.sequence_no ASC, event.id ASC
            LIMIT $6
            "#,
        );
        let candidates = sqlx::query(sqlx::AssertSqlSafe(candidate_query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("checkpoint_id", request.checkpoint_id)?)
            .bind(
                request
                    .after_id
                    .map(|value| to_i64("after_id", value))
                    .transpose()?,
            )
            .bind(&now)
            .bind(limit)
            .fetch_all(&mut *transaction)
            .await
            .map_err(sql_error)?;

        let lease_expires_at_expr = self.dialect.sql_timestamp_expr("$8");
        let updated_at = self.dialect.sql_timestamp_expr("$7");
        let update_query = format!(
            r#"
            UPDATE kb_drive_event_inbox
            SET processing_state = 'RECEIVED',
                attempt_count = attempt_count + 1,
                lease_owner = $5,
                lease_token = $6,
                lease_expires_at = {lease_expires_at_expr},
                updated_at = {updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND version = $4
              AND processing_state IN ('RECEIVED', 'RETRY', 'DEFERRED')
            RETURNING {INBOX_COLUMNS}
            "#,
        );
        let mut claimed = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let event_id: i64 = candidate.try_get("id").map_err(row_error)?;
            let event_version: i64 = candidate.try_get("version").map_err(row_error)?;
            if let Some(row) = sqlx::query(sqlx::AssertSqlSafe(update_query.as_str()))
                .bind(to_i64("tenant_id", request.scope.tenant_id)?)
                .bind(to_i64("organization_id", request.scope.organization_id)?)
                .bind(event_id)
                .bind(event_version)
                .bind(owner)
                .bind(new_lease_token())
                .bind(&now)
                .bind(&lease_expires_at)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(sql_error)?
            {
                claimed.push(inbox_from_row(&row)?);
            }
        }
        transaction.commit().await.map_err(sql_error)?;
        Ok(claimed)
    }

    async fn complete_event(
        &self,
        request: CompleteWikiDriveEventRequest,
    ) -> Result<WikiDriveInboxEvent, WikiPersistenceError> {
        Ok(self
            .apply_event(ApplyWikiDriveEventRequest {
                complete: request,
                mutation: WikiDriveProjectionMutation::None,
            })
            .await?
            .event)
    }

    async fn apply_event(
        &self,
        request: ApplyWikiDriveEventRequest,
    ) -> Result<WikiDriveEventApplicationResult, WikiPersistenceError> {
        let mutation = request.mutation;
        let request = request.complete;
        validate_scope(request.scope)?;
        let lease_token = require_text("lease_token", &request.lease_token, 128)?;
        let mut transaction = self.pool.begin().await.map_err(sql_error)?;
        let event_query = format!(
            "SELECT {INBOX_COLUMNS} FROM kb_drive_event_inbox WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND lease_token = $4",
        );
        let event_row = sqlx::query(sqlx::AssertSqlSafe(event_query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("event_id", request.event_id)?)
            .bind(lease_token)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(sql_error)?
            .ok_or_else(|| {
                WikiPersistenceError::Conflict(format!(
                    "Drive inbox event {} lease is stale",
                    request.event_id
                ))
            })?;
        let event = inbox_from_row(&event_row)?;

        let checkpoint_query = format!(
            "SELECT {CHECKPOINT_COLUMNS} FROM kb_drive_source_checkpoint WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        );
        let checkpoint_row = sqlx::query(sqlx::AssertSqlSafe(checkpoint_query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(to_i64("checkpoint_id", event.checkpoint_id)?)
            .fetch_one(&mut *transaction)
            .await
            .map_err(sql_error)?;
        let checkpoint = checkpoint_from_row(&checkpoint_row)?;
        let expected_sequence = checkpoint
            .last_sequence_no
            .checked_add(1)
            .ok_or_else(|| WikiPersistenceError::Internal("Drive sequence overflow".to_string()))?;
        if event.sequence_no != expected_sequence {
            return Err(WikiPersistenceError::Conflict(format!(
                "Drive inbox event sequence {} cannot advance checkpoint sequence {}",
                event.sequence_no, checkpoint.last_sequence_no
            )));
        }

        let now = super::now()?;
        let (projection, public_route_change) =
            super::event_application::apply_projection_mutation(
                self,
                &mut transaction,
                &event,
                &mutation,
                request.actor_id,
                &now,
            )
            .await?;
        let timestamp = self.dialect.sql_timestamp_expr("$6");
        let event_update = format!(
            r#"
            UPDATE kb_drive_event_inbox
            SET processing_state = 'APPLIED',
                lease_owner = NULL,
                lease_token = NULL,
                lease_expires_at = NULL,
                applied_at = {timestamp},
                updated_at = {timestamp},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND lease_token = $4 AND version = $5
            RETURNING {INBOX_COLUMNS}
            "#,
        );
        let applied_row = sqlx::query(sqlx::AssertSqlSafe(event_update.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("event_id", request.event_id)?)
            .bind(lease_token)
            .bind(to_i64("event_version", event.version)?)
            .bind(&now)
            .fetch_one(&mut *transaction)
            .await
            .map_err(sql_error)?;

        let gap_remains = checkpoint
            .gap_to_sequence_no
            .is_some_and(|gap_to| gap_to > event.sequence_no);
        let gap_from = gap_remains.then(|| event.sequence_no + 1);
        let gap_to = gap_remains
            .then_some(checkpoint.gap_to_sequence_no)
            .flatten();
        let checkpoint_event_time = self.dialect.sql_timestamp_expr("$10");
        let checkpoint_updated_at = self.dialect.sql_timestamp_expr("$11");
        let checkpoint_update = format!(
            r#"
            UPDATE kb_drive_source_checkpoint
            SET last_sequence_no = $4,
                last_event_id = $5,
                stream_state = $6,
                gap_from_sequence_no = $7,
                gap_to_sequence_no = $8,
                last_event_time = {checkpoint_event_time},
                last_observed_at = {checkpoint_updated_at},
                updated_by = $9,
                updated_at = {checkpoint_updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND version = $12 AND status = 1
            "#,
        );
        let checkpoint_updated = sqlx::query(sqlx::AssertSqlSafe(checkpoint_update.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(to_i64("checkpoint_id", event.checkpoint_id)?)
            .bind(to_i64("sequence_no", event.sequence_no)?)
            .bind(event.source_event_id.as_str())
            .bind(if gap_remains {
                "GAP_DETECTED"
            } else {
                "HEALTHY"
            })
            .bind(
                gap_from
                    .map(|value| to_i64("gap_from", value))
                    .transpose()?,
            )
            .bind(gap_to.map(|value| to_i64("gap_to", value)).transpose()?)
            .bind(require_id("actor_id", request.actor_id)?)
            .bind(event.source_event_time.as_str())
            .bind(&now)
            .bind(to_i64("checkpoint_version", checkpoint.version)?)
            .execute(&mut *transaction)
            .await
            .map_err(sql_error)?;
        if checkpoint_updated.rows_affected() != 1 {
            return Err(WikiPersistenceError::Conflict(
                "Drive checkpoint changed while completing an inbox event".to_string(),
            ));
        }

        let publication_time = self.dialect.sql_timestamp_expr("$6");
        let publication_update = format!(
            r#"
            UPDATE kb_site_publication
            SET last_projected_drive_checkpoint = $4,
                updated_by = $5,
                updated_at = {publication_time},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND last_projected_drive_checkpoint < $4 AND status = 1
            "#,
        );
        let publication_updated = sqlx::query(sqlx::AssertSqlSafe(publication_update.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(to_i64("site_publication_id", event.site_publication_id)?)
            .bind(to_i64("sequence_no", event.sequence_no)?)
            .bind(require_id("actor_id", request.actor_id)?)
            .bind(&now)
            .execute(&mut *transaction)
            .await
            .map_err(sql_error)?;
        if publication_updated.rows_affected() != 1 {
            return Err(WikiPersistenceError::Conflict(
                "Wiki publication checkpoint did not advance exactly once".to_string(),
            ));
        }

        transaction.commit().await.map_err(sql_error)?;
        Ok(WikiDriveEventApplicationResult {
            event: inbox_from_row(&applied_row)?,
            projection,
            public_route_change,
        })
    }

    async fn retry_event(
        &self,
        request: RetryWikiDriveEventRequest,
    ) -> Result<WikiDriveInboxEvent, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let lease_token = require_text("lease_token", &request.lease_token, 128)?;
        let error_code = require_text("error_code", &request.error_code, 128)?;
        let error_summary = require_text("error_summary", &request.error_summary, 1_024)?;
        if request.max_attempts == 0 || request.max_attempts > 100 {
            return Err(WikiPersistenceError::InvalidRequest(
                "max_attempts must be between 1 and 100".to_string(),
            ));
        }
        let (now, next_retry_at) = retry_time(request.retry_delay_seconds)?;
        let next_retry_at_expr = self.dialect.sql_timestamp_expr("$8");
        let updated_at = self.dialect.sql_timestamp_expr("$9");
        let query = format!(
            r#"
            UPDATE kb_drive_event_inbox
            SET processing_state = CASE
                    WHEN attempt_count >= $7 THEN 'DEAD_LETTER'
                    ELSE 'RETRY'
                END,
                next_retry_at = CASE
                    WHEN attempt_count >= $7 THEN NULL
                    ELSE {next_retry_at_expr}
                END,
                lease_owner = NULL,
                lease_token = NULL,
                lease_expires_at = NULL,
                last_error_code = $5,
                last_error_summary = $6,
                updated_at = {updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND lease_token = $4
              AND processing_state IN ('RECEIVED', 'RETRY', 'DEFERRED')
            RETURNING {INBOX_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("event_id", request.event_id)?)
            .bind(lease_token)
            .bind(error_code)
            .bind(error_summary)
            .bind(i64::from(request.max_attempts))
            .bind(&next_retry_at)
            .bind(&now)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?
            .ok_or_else(|| {
                WikiPersistenceError::Conflict(format!(
                    "Drive inbox event {} retry lease is stale",
                    request.event_id
                ))
            })?;
        inbox_from_row(&row)
    }
}

fn validate_receive_request(
    request: &ReceiveWikiDriveEventRequest,
) -> Result<(), WikiPersistenceError> {
    validate_scope(request.scope)?;
    require_id("site_publication_id", request.site_publication_id)?;
    require_id("checkpoint_id", request.checkpoint_id)?;
    require_text("source_event_id", &request.source_event_id, 128)?;
    if request.sequence_no == 0 {
        return Err(WikiPersistenceError::InvalidRequest(
            "sequence_no must be greater than zero".to_string(),
        ));
    }
    require_text("drive_node_uuid", &request.drive_node_uuid, 64)?;
    if let Some(value) = request.drive_version_uuid.as_deref() {
        require_text("drive_version_uuid", value, 64)?;
    }
    require_sha256("payload_sha256", &request.payload_sha256)?;
    if request.payload_json.len() > 65_536 {
        return Err(WikiPersistenceError::InvalidRequest(
            "payload_json exceeds 65536 bytes".to_string(),
        ));
    }
    serde_json::from_str::<serde_json::Value>(&request.payload_json).map_err(|error| {
        WikiPersistenceError::InvalidRequest(format!("payload_json is invalid JSON: {error}"))
    })?;
    require_text("source_event_time", &request.source_event_time, 64)?;
    Ok(())
}

fn event_matches_replay(
    event: &WikiDriveInboxEvent,
    request: &ReceiveWikiDriveEventRequest,
) -> bool {
    event.site_publication_id == request.site_publication_id
        && event.checkpoint_id == request.checkpoint_id
        && event.source_event_id == request.source_event_id.trim()
        && event.event_type == request.event_type
        && event.sequence_no == request.sequence_no
        && event.drive_node_uuid == request.drive_node_uuid.trim()
        && event.drive_version_uuid.as_deref()
            == request.drive_version_uuid.as_deref().map(str::trim)
        && event.payload_sha256 == request.payload_sha256
        && json_payloads_match(&event.payload_json, &request.payload_json)
        && timestamps_match(&event.source_event_time, request.source_event_time.trim())
}

/// JSONB round-trips reorder object keys and drop insignificant whitespace, so compare the
/// decoded values rather than the serialized text. The SHA-256 hash still binds the exact bytes.
fn json_payloads_match(left: &str, right: &str) -> bool {
    match (
        serde_json::from_str::<serde_json::Value>(left),
        serde_json::from_str::<serde_json::Value>(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// Compares two RFC 3339 instants by value: the database round-trip renders microseconds
/// (`.000000Z`) while callers may send the same instant without a fractional part.
fn timestamps_match(left: &str, right: &str) -> bool {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    match (
        OffsetDateTime::parse(left, &Rfc3339),
        OffsetDateTime::parse(right, &Rfc3339),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

pub(super) fn inbox_from_row(row: &AnyRow) -> Result<WikiDriveInboxEvent, WikiPersistenceError> {
    Ok(WikiDriveInboxEvent {
        id: from_i64("id", row.try_get("id").map_err(row_error)?)?,
        uuid: row.try_get("uuid").map_err(row_error)?,
        scope: WikiPersistenceScope {
            tenant_id: from_i64("tenant_id", row.try_get("tenant_id").map_err(row_error)?)?,
            organization_id: from_i64(
                "organization_id",
                row.try_get("organization_id").map_err(row_error)?,
            )?,
        },
        site_publication_id: from_i64(
            "site_publication_id",
            row.try_get("site_publication_id").map_err(row_error)?,
        )?,
        checkpoint_id: from_i64(
            "checkpoint_id",
            row.try_get("checkpoint_id").map_err(row_error)?,
        )?,
        source_event_id: row.try_get("source_event_id").map_err(row_error)?,
        event_type: parse_enum("event_type", row.try_get("event_type").map_err(row_error)?)?,
        sequence_no: from_i64(
            "sequence_no",
            row.try_get("sequence_no").map_err(row_error)?,
        )?,
        drive_node_uuid: row.try_get("drive_node_uuid").map_err(row_error)?,
        drive_version_uuid: row.try_get("drive_version_uuid").map_err(row_error)?,
        payload_sha256: row.try_get("payload_sha256").map_err(row_error)?,
        payload_json: row.try_get("payload_json").map_err(row_error)?,
        source_event_time: row.try_get("source_event_time").map_err(row_error)?,
        processing_state: parse_enum(
            "processing_state",
            row.try_get("processing_state").map_err(row_error)?,
        )?,
        attempt_count: from_i32(
            "attempt_count",
            row.try_get("attempt_count").map_err(row_error)?,
        )?,
        lease_token: row.try_get("lease_token").map_err(row_error)?,
        version: from_i64("version", row.try_get("version").map_err(row_error)?)?,
    })
}
