use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_persistence::{
    ClaimWikiSourceProcessingRequest, CompleteWikiSourceProcessingRequest,
    ListWikiSourceProjectionsRequest, RetryWikiSourceProcessingRequest,
    UpsertWikiSourceProjectionRequest, WikiPersistenceError, WikiPersistenceScope,
    WikiSourceProjection, WikiSourceProjectionPage, WikiSourceProjectionStore,
    WikiSourceProjectionUpsertDisposition, WikiSourceProjectionUpsertResult, WikiUpdatePolicy,
};
use sdkwork_utils_rust::uuid;
use sqlx::{any::AnyRow, Row};

use super::{
    claim_limit, from_i32, from_i64, lease_times, new_lease_token, parse_enum, require_id,
    require_sha256, require_text, retry_time, row_error, sql_error, to_i64, validate_scope,
    SqlxWikiPersistenceStore,
};

pub(super) const PROJECTION_COLUMNS: &str = r#"
    id, uuid, tenant_id, organization_id, site_publication_id, space_id,
    drive_space_uuid, drive_node_uuid, drive_version_uuid, source_path,
    canonical_route, file_kind, media_type, size_bytes, content_sha256,
    source_state, publication_state, visibility, index_state,
    public_drive_version_uuid, page_public_version, source_sequence_no,
    last_source_event_id, processing_attempt_count, processing_lease_token,
    processing_fence, version
"#;

/// Inspection page bound aligned with the app-api `page_size` contract
/// (1..=200). Deliberately not `MAX_CLAIM_BATCH_SIZE`: claim batches and
/// reader pages are different budgets.
const MAX_SOURCE_PROJECTION_PAGE_SIZE: u32 = 200;

#[async_trait]
impl WikiSourceProjectionStore for SqlxWikiPersistenceStore {
    async fn upsert_source_projection(
        &self,
        request: UpsertWikiSourceProjectionRequest,
    ) -> Result<WikiSourceProjectionUpsertResult, WikiPersistenceError> {
        validate_projection_request(&request)?;
        let publication = sqlx::query(
            r#"
            SELECT space_id, drive_space_uuid, default_visibility, update_policy
            FROM kb_site_publication
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1
            "#,
        )
        .bind(to_i64("tenant_id", request.scope.tenant_id)?)
        .bind(to_i64("organization_id", request.scope.organization_id)?)
        .bind(require_id(
            "site_publication_id",
            request.site_publication_id,
        )?)
        .fetch_optional(&self.pool)
        .await
        .map_err(sql_error)?
        .ok_or(WikiPersistenceError::NotFound {
            resource: "wiki_publication",
            id: request.site_publication_id,
        })?;
        let publication_space_id = from_i64(
            "space_id",
            publication.try_get("space_id").map_err(row_error)?,
        )?;
        let publication_drive_space_uuid: String =
            publication.try_get("drive_space_uuid").map_err(row_error)?;
        if publication_space_id != request.space_id
            || publication_drive_space_uuid != request.drive_space_uuid.trim()
        {
            return Err(WikiPersistenceError::Conflict(
                "source projection identity does not match its Wiki publication".to_string(),
            ));
        }
        let default_visibility: String = publication
            .try_get("default_visibility")
            .map_err(row_error)?;
        let update_policy: WikiUpdatePolicy = parse_enum(
            "update_policy",
            publication.try_get("update_policy").map_err(row_error)?,
        )?;

        if let Some(existing) = self
            .get_source_projection_by_node(
                request.scope,
                request.site_publication_id,
                &request.drive_node_uuid,
            )
            .await?
        {
            if request.source_sequence_no < existing.source_sequence_no {
                return Ok(WikiSourceProjectionUpsertResult {
                    projection: existing,
                    disposition: WikiSourceProjectionUpsertDisposition::IgnoredStale,
                });
            }
            if request.source_sequence_no == existing.source_sequence_no {
                if projection_matches_replay(&existing, &request) {
                    return Ok(WikiSourceProjectionUpsertResult {
                        projection: existing,
                        disposition: WikiSourceProjectionUpsertDisposition::UnchangedReplay,
                    });
                }
                return Err(WikiPersistenceError::Conflict(format!(
                    "Drive sequence {} has conflicting source projection payloads",
                    request.source_sequence_no
                )));
            }

            let now = super::now()?;
            let updated_at = self.dialect.sql_timestamp_expr("$13");
            let query = format!(
                r#"
                UPDATE kb_source_file_projection
                SET drive_version_uuid = $4,
                    source_path = $5,
                    file_kind = $6,
                    media_type = $7,
                    size_bytes = $8,
                    content_sha256 = $9,
                    source_state = 'DISCOVERED',
                    publication_state = CASE
                        WHEN $14 = 'UNPUBLISH_DURING_PROCESSING'
                             AND publication_state = 'PUBLISHED' THEN 'UNPUBLISHED'
                        ELSE publication_state
                    END,
                    index_state = CASE WHEN $6 = 'PAGE' THEN 'PENDING' ELSE 'NOT_REQUIRED' END,
                    source_sequence_no = $10,
                    last_source_event_id = $11,
                    processing_attempt_count = 0,
                    next_processing_at = NULL,
                    processing_lease_owner = NULL,
                    processing_lease_token = NULL,
                    processing_lease_expires_at = NULL,
                    processing_fence = processing_fence + 1,
                    last_error_code = NULL,
                    last_error_summary = NULL,
                    updated_by = $12,
                    updated_at = {updated_at},
                    version = version + 1
                WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
                  AND version = $15 AND status = 1
                RETURNING {PROJECTION_COLUMNS}
                "#,
            );
            let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(to_i64("tenant_id", request.scope.tenant_id)?)
                .bind(to_i64("organization_id", request.scope.organization_id)?)
                .bind(to_i64("projection_id", existing.id)?)
                .bind(request.drive_version_uuid.trim())
                .bind(request.source_path.trim())
                .bind(request.file_kind.as_str())
                .bind(request.media_type.trim())
                .bind(to_i64("size_bytes", request.size_bytes)?)
                .bind(request.content_sha256.as_str())
                .bind(to_i64("source_sequence_no", request.source_sequence_no)?)
                .bind(request.source_event_id.trim())
                .bind(require_id("actor_id", request.actor_id)?)
                .bind(&now)
                .bind(update_policy.as_str())
                .bind(to_i64("expected_version", existing.version)?)
                .fetch_optional(&self.pool)
                .await
                .map_err(sql_error)?
                .ok_or(WikiPersistenceError::StaleVersion {
                    resource: "wiki_source_projection",
                    id: existing.id,
                    expected: existing.version,
                })?;
            return Ok(WikiSourceProjectionUpsertResult {
                projection: projection_from_row(&row)?,
                disposition: WikiSourceProjectionUpsertDisposition::Updated,
            });
        }

        let now = super::now()?;
        let timestamp = self.dialect.sql_timestamp_expr("$20");
        let query = format!(
            r#"
            INSERT INTO kb_source_file_projection (
                id, uuid, tenant_id, organization_id, site_publication_id, space_id,
                drive_space_uuid, drive_node_uuid, drive_version_uuid, source_path,
                file_kind, media_type, size_bytes, content_sha256, visibility, index_state,
                source_sequence_no, last_source_event_id, created_by, updated_by,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $19,
                {timestamp}, {timestamp}
            )
            RETURNING {PROJECTION_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(self.next_id()?)
            .bind(uuid())
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id(
                "site_publication_id",
                request.site_publication_id,
            )?)
            .bind(require_id("space_id", request.space_id)?)
            .bind(request.drive_space_uuid.trim())
            .bind(request.drive_node_uuid.trim())
            .bind(request.drive_version_uuid.trim())
            .bind(request.source_path.trim())
            .bind(request.file_kind.as_str())
            .bind(request.media_type.trim())
            .bind(to_i64("size_bytes", request.size_bytes)?)
            .bind(request.content_sha256.as_str())
            .bind(default_visibility)
            .bind(if request.file_kind.as_str() == "PAGE" {
                "PENDING"
            } else {
                "NOT_REQUIRED"
            })
            .bind(to_i64("source_sequence_no", request.source_sequence_no)?)
            .bind(request.source_event_id.trim())
            .bind(require_id("actor_id", request.actor_id)?)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(sql_error)?;
        Ok(WikiSourceProjectionUpsertResult {
            projection: projection_from_row(&row)?,
            disposition: WikiSourceProjectionUpsertDisposition::Created,
        })
    }

    async fn get_source_projection_by_node(
        &self,
        scope: WikiPersistenceScope,
        site_publication_id: u64,
        drive_node_uuid: &str,
    ) -> Result<Option<WikiSourceProjection>, WikiPersistenceError> {
        validate_scope(scope)?;
        let drive_node_uuid = require_text("drive_node_uuid", drive_node_uuid, 64)?;
        let query = format!(
            "SELECT {PROJECTION_COLUMNS} FROM kb_source_file_projection WHERE tenant_id = $1 AND organization_id = $2 AND site_publication_id = $3 AND drive_node_uuid = $4 AND status = 1",
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(require_id("site_publication_id", site_publication_id)?)
            .bind(drive_node_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?
            .map(|row| projection_from_row(&row))
            .transpose()
    }

    async fn list_source_projections(
        &self,
        request: ListWikiSourceProjectionsRequest,
    ) -> Result<WikiSourceProjectionPage, WikiPersistenceError> {
        validate_scope(request.scope)?;
        if request.limit == 0 || request.limit > MAX_SOURCE_PROJECTION_PAGE_SIZE {
            return Err(WikiPersistenceError::InvalidRequest(format!(
                "limit must be between 1 and {MAX_SOURCE_PROJECTION_PAGE_SIZE}"
            )));
        }
        let limit = i64::from(request.limit);
        let query = format!(
            "SELECT {PROJECTION_COLUMNS} FROM kb_source_file_projection \
             WHERE tenant_id = $1 AND organization_id = $2 AND site_publication_id = $3 \
             AND id > COALESCE($4, 0) AND status = 1 \
             ORDER BY id ASC LIMIT $5",
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id(
                "site_publication_id",
                request.site_publication_id,
            )?)
            .bind(
                request
                    .after_projection_id
                    .map(|value| to_i64("after_projection_id", value))
                    .transpose()?,
            )
            .bind(limit + 1)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;

        let mut projections = rows
            .iter()
            .map(projection_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = projections.len() > usize::try_from(limit).unwrap_or(0);
        if has_more {
            projections.truncate(usize::try_from(limit).unwrap_or(0));
        }
        let next_after_projection_id = has_more
            .then(|| projections.last().map(|projection| projection.id))
            .flatten();
        Ok(WikiSourceProjectionPage {
            projections,
            next_after_projection_id,
        })
    }

    async fn claim_source_processing(
        &self,
        request: ClaimWikiSourceProcessingRequest,
    ) -> Result<Vec<WikiSourceProjection>, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let owner = require_text("claim_owner", &request.claim_owner, 128)?;
        let limit = claim_limit(request.limit)?;
        let (now, lease_expires_at) = lease_times(request.lease_seconds)?;
        let now_expr = self.dialect.sql_timestamp_expr("$5");
        let mut transaction = self.pool.begin().await.map_err(sql_error)?;
        let candidate_query = format!(
            r#"
            SELECT id, version
            FROM kb_source_file_projection
            WHERE tenant_id = $1 AND organization_id = $2 AND site_publication_id = $3
              AND id > COALESCE($4, 0) AND status = 1
              AND (
                    source_state IN ('DISCOVERED', 'QUEUED', 'ERROR')
                    OR (
                        source_state = 'READY'
                        AND EXISTS (
                            SELECT 1 FROM kb_site_publication publication
                            WHERE publication.tenant_id = kb_source_file_projection.tenant_id
                              AND publication.organization_id = kb_source_file_projection.organization_id
                              AND publication.id = kb_source_file_projection.site_publication_id
                              AND publication.publication_mode = 'AUTO_PUBLIC_AFTER_CHECKS'
                              AND publication.default_visibility IN ('UNLISTED', 'PUBLIC')
                              AND publication.status = 1
                        )
                        AND (
                            publication_state <> 'PUBLISHED'
                            OR public_drive_version_uuid IS NULL
                            OR public_drive_version_uuid <> drive_version_uuid
                        )
                    )
              )
              AND (next_processing_at IS NULL OR next_processing_at <= {now_expr})
              AND (processing_lease_expires_at IS NULL OR processing_lease_expires_at <= {now_expr})
            ORDER BY id ASC
            LIMIT $6
            "#,
        );
        let candidates = sqlx::query(sqlx::AssertSqlSafe(candidate_query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id(
                "site_publication_id",
                request.site_publication_id,
            )?)
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

        let updated_at = self.dialect.sql_timestamp_expr("$7");
        let lease_expires_at_expr = self.dialect.sql_timestamp_expr("$8");
        let update_query = format!(
            r#"
            UPDATE kb_source_file_projection
            SET source_state = 'PROCESSING',
                processing_attempt_count = processing_attempt_count + 1,
                processing_lease_owner = $5,
                processing_lease_token = $6,
                processing_lease_expires_at = {lease_expires_at_expr},
                processing_fence = processing_fence + 1,
                updated_at = {updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND version = $4 AND status = 1
            RETURNING {PROJECTION_COLUMNS}
            "#,
        );
        let mut claimed = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let projection_id: i64 = candidate.try_get("id").map_err(row_error)?;
            let expected_version: i64 = candidate.try_get("version").map_err(row_error)?;
            let lease_token = new_lease_token();
            if let Some(row) = sqlx::query(sqlx::AssertSqlSafe(update_query.as_str()))
                .bind(to_i64("tenant_id", request.scope.tenant_id)?)
                .bind(to_i64("organization_id", request.scope.organization_id)?)
                .bind(projection_id)
                .bind(expected_version)
                .bind(owner)
                .bind(lease_token)
                .bind(&now)
                .bind(&lease_expires_at)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(sql_error)?
            {
                claimed.push(projection_from_row(&row)?);
            }
        }
        transaction.commit().await.map_err(sql_error)?;
        Ok(claimed)
    }

    async fn complete_source_processing(
        &self,
        request: CompleteWikiSourceProcessingRequest,
    ) -> Result<WikiSourceProjection, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let lease_token = require_text("lease_token", &request.lease_token, 128)?;
        let canonical_route = require_text("canonical_route", &request.canonical_route, 2_048)?;
        let now = super::now()?;
        let mut transaction = self.pool.begin().await.map_err(sql_error)?;
        sqlx::query(
            r#"
            UPDATE kb_site_publication SET updated_at = updated_at
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1
            "#,
        )
        .bind(to_i64("tenant_id", request.scope.tenant_id)?)
        .bind(to_i64("organization_id", request.scope.organization_id)?)
        .bind(require_id(
            "site_publication_id",
            request.site_publication_id,
        )?)
        .execute(&mut *transaction)
        .await
        .map_err(sql_error)?;
        let collision: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT id FROM kb_source_file_projection
            WHERE tenant_id = $1 AND organization_id = $2 AND site_publication_id = $3
              AND id <> $4 AND canonical_route = $5 AND status = 1
              AND source_state <> 'DELETED'
            ORDER BY id ASC LIMIT 1
            "#,
        )
        .bind(to_i64("tenant_id", request.scope.tenant_id)?)
        .bind(to_i64("organization_id", request.scope.organization_id)?)
        .bind(require_id(
            "site_publication_id",
            request.site_publication_id,
        )?)
        .bind(require_id("projection_id", request.projection_id)?)
        .bind(canonical_route)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(sql_error)?;
        if collision.is_some() {
            return Err(WikiPersistenceError::Conflict(
                "another Wiki source projection already owns the canonical route".to_string(),
            ));
        }
        let timestamp = self.dialect.sql_timestamp_expr("$9");
        let query = format!(
            r#"
            UPDATE kb_source_file_projection
            SET source_state = 'READY',
                canonical_route = $6,
                index_state = $7,
                processing_lease_owner = NULL,
                processing_lease_token = NULL,
                processing_lease_expires_at = NULL,
                last_error_code = NULL,
                last_error_summary = NULL,
                last_processed_at = {timestamp},
                updated_by = $8,
                updated_at = {timestamp},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND processing_lease_token = $4 AND processing_fence = $5
              AND source_state = 'PROCESSING' AND status = 1
            RETURNING {PROJECTION_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("projection_id", request.projection_id)?)
            .bind(lease_token)
            .bind(to_i64("processing_fence", request.processing_fence)?)
            .bind(canonical_route)
            .bind(request.index_state.as_str())
            .bind(require_id("actor_id", request.actor_id)?)
            .bind(&now)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(sql_error)?
            .ok_or_else(|| {
                WikiPersistenceError::Conflict(format!(
                    "source projection {} processing lease is stale",
                    request.projection_id
                ))
            })?;
        let projection = projection_from_row(&row)?;
        transaction.commit().await.map_err(sql_error)?;
        Ok(projection)
    }

    async fn retry_source_processing(
        &self,
        request: RetryWikiSourceProcessingRequest,
    ) -> Result<WikiSourceProjection, WikiPersistenceError> {
        validate_scope(request.scope)?;
        let lease_token = require_text("lease_token", &request.lease_token, 128)?;
        let error_code = require_text("error_code", &request.error_code, 128)?;
        let error_summary = require_text("error_summary", &request.error_summary, 1_024)?;
        if request.max_attempts == 0 || request.max_attempts > 100 {
            return Err(WikiPersistenceError::InvalidRequest(
                "max_attempts must be between 1 and 100".to_string(),
            ));
        }
        let (now, retry_at) = retry_time(request.retry_delay_seconds)?;
        let updated_at = self.dialect.sql_timestamp_expr("$10");
        let retry_at_expr = self.dialect.sql_timestamp_expr("$11");
        let query = format!(
            r#"
            UPDATE kb_source_file_projection
            SET source_state = CASE
                    WHEN processing_attempt_count >= $8 THEN 'QUARANTINED'
                    ELSE 'ERROR'
                END,
                next_processing_at = CASE
                    WHEN processing_attempt_count >= $8 THEN NULL
                    ELSE {retry_at_expr}
                END,
                processing_lease_owner = NULL,
                processing_lease_token = NULL,
                processing_lease_expires_at = NULL,
                last_error_code = $6,
                last_error_summary = $7,
                updated_by = $9,
                updated_at = {updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND processing_lease_token = $4 AND processing_fence = $5
              AND source_state = 'PROCESSING' AND status = 1
            RETURNING {PROJECTION_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", request.scope.tenant_id)?)
            .bind(to_i64("organization_id", request.scope.organization_id)?)
            .bind(require_id("projection_id", request.projection_id)?)
            .bind(lease_token)
            .bind(to_i64("processing_fence", request.processing_fence)?)
            .bind(error_code)
            .bind(error_summary)
            .bind(i64::from(request.max_attempts))
            .bind(require_id("actor_id", request.actor_id)?)
            .bind(&now)
            .bind(&retry_at)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?
            .ok_or_else(|| {
                WikiPersistenceError::Conflict(format!(
                    "source projection {} processing lease is stale",
                    request.projection_id
                ))
            })?;
        projection_from_row(&row)
    }
}

fn validate_projection_request(
    request: &UpsertWikiSourceProjectionRequest,
) -> Result<(), WikiPersistenceError> {
    validate_scope(request.scope)?;
    require_id("site_publication_id", request.site_publication_id)?;
    require_id("space_id", request.space_id)?;
    require_id("actor_id", request.actor_id)?;
    require_text("drive_space_uuid", &request.drive_space_uuid, 64)?;
    require_text("drive_node_uuid", &request.drive_node_uuid, 64)?;
    require_text("drive_version_uuid", &request.drive_version_uuid, 64)?;
    require_text("source_path", &request.source_path, 4_096)?;
    require_text("media_type", &request.media_type, 255)?;
    require_sha256("content_sha256", &request.content_sha256)?;
    if request.source_sequence_no == 0 {
        return Err(WikiPersistenceError::InvalidRequest(
            "source_sequence_no must be greater than zero".to_string(),
        ));
    }
    require_text("source_event_id", &request.source_event_id, 128)?;
    Ok(())
}

fn projection_matches_replay(
    projection: &WikiSourceProjection,
    request: &UpsertWikiSourceProjectionRequest,
) -> bool {
    projection.drive_version_uuid == request.drive_version_uuid.trim()
        && projection.source_path == request.source_path.trim()
        && projection.file_kind == request.file_kind
        && projection.media_type == request.media_type.trim()
        && projection.size_bytes == request.size_bytes
        && projection.content_sha256 == request.content_sha256
        && projection.last_source_event_id.as_deref() == Some(request.source_event_id.trim())
}

pub(super) fn projection_from_row(
    row: &AnyRow,
) -> Result<WikiSourceProjection, WikiPersistenceError> {
    Ok(WikiSourceProjection {
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
        space_id: from_i64("space_id", row.try_get("space_id").map_err(row_error)?)?,
        drive_space_uuid: row.try_get("drive_space_uuid").map_err(row_error)?,
        drive_node_uuid: row.try_get("drive_node_uuid").map_err(row_error)?,
        drive_version_uuid: row.try_get("drive_version_uuid").map_err(row_error)?,
        source_path: row.try_get("source_path").map_err(row_error)?,
        canonical_route: row.try_get("canonical_route").map_err(row_error)?,
        file_kind: parse_enum("file_kind", row.try_get("file_kind").map_err(row_error)?)?,
        media_type: row.try_get("media_type").map_err(row_error)?,
        size_bytes: from_i64("size_bytes", row.try_get("size_bytes").map_err(row_error)?)?,
        content_sha256: row.try_get("content_sha256").map_err(row_error)?,
        source_state: parse_enum(
            "source_state",
            row.try_get("source_state").map_err(row_error)?,
        )?,
        publication_state: parse_enum(
            "publication_state",
            row.try_get("publication_state").map_err(row_error)?,
        )?,
        visibility: parse_enum("visibility", row.try_get("visibility").map_err(row_error)?)?,
        index_state: parse_enum(
            "index_state",
            row.try_get("index_state").map_err(row_error)?,
        )?,
        public_drive_version_uuid: row
            .try_get("public_drive_version_uuid")
            .map_err(row_error)?,
        page_public_version: from_i64(
            "page_public_version",
            row.try_get("page_public_version").map_err(row_error)?,
        )?,
        source_sequence_no: from_i64(
            "source_sequence_no",
            row.try_get("source_sequence_no").map_err(row_error)?,
        )?,
        last_source_event_id: row.try_get("last_source_event_id").map_err(row_error)?,
        processing_attempt_count: from_i32(
            "processing_attempt_count",
            row.try_get("processing_attempt_count").map_err(row_error)?,
        )?,
        processing_lease_token: row.try_get("processing_lease_token").map_err(row_error)?,
        processing_fence: from_i64(
            "processing_fence",
            row.try_get("processing_fence").map_err(row_error)?,
        )?,
        version: from_i64("version", row.try_get("version").map_err(row_error)?)?,
    })
}
