use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_concept_store::{
    AppendKnowledgeOkfLogEntryRecord, CreateKnowledgeOkfConceptRevisionRecord,
    KnowledgeOkfConceptProjection, KnowledgeOkfConceptStore, KnowledgeOkfConceptStoreError,
    MarkKnowledgeOkfConceptCurrentRevisionRecord, UpsertKnowledgeOkfConceptRecord,
};
use sdkwork_knowledgebase_contract::okf::{
    KnowledgeOkfConcept, KnowledgeOkfConceptRevision, OkfConceptPublishState, OkfConceptSummary,
    OkfLogEntry, OkfLogEventType, OkfRevisionReviewState,
};
use sdkwork_utils_rust::is_blank;
use sqlx::{any::AnyRow, Any, AnyPool, QueryBuilder, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};
use crate::postgres_okf_concept_transaction::{
    next_okf_revision_no_in_transaction, upsert_okf_concept_in_transaction,
};

const ACTIVE_STATUS: i64 = 1;
const DELETED_STATUS: i64 = 0;
const MAX_OKF_LIST_ROWS: i64 = 200;
const INITIAL_VERSION: i64 = 0;
const MAX_PROJECTION_BATCH_SIZE: usize = 200;

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeOkfConceptStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeOkfConceptStore {
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

    pub async fn next_revision_no(
        &self,
        concept_row_id: u64,
    ) -> Result<u64, KnowledgeOkfConceptStoreError> {
        let mut transaction = self.pool.begin().await.map_err(sqlx_error)?;
        let revision_no = next_okf_revision_no_in_transaction(
            &mut transaction,
            self.tenant_id,
            self.organization_id,
            self.timestamp_dialect,
            concept_row_id,
        )
        .await?;
        transaction.commit().await.map_err(sqlx_error)?;
        Ok(revision_no)
    }

    pub async fn get_concept_by_row_id(
        &self,
        concept_row_id: u64,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let concept_row_id = to_i64("concept_row_id", concept_row_id)?;
        let row = sqlx::query(
            r#"
            SELECT
                id,
                space_id,
                concept_id,
                title,
                concept_type,
                logical_path,
                description,
                source_count,
                CAST(tags AS TEXT) AS tags,
                current_revision_id,
                publish_state,
                CAST(updated_at AS TEXT) AS updated_at
            FROM kb_okf_concept
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(concept_row_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?
        .ok_or_else(|| {
            KnowledgeOkfConceptStoreError::Internal(format!(
                "missing okf concept: {concept_row_id}"
            ))
        })?;

        concept_from_row(&row)
    }

    pub async fn list_concept_revisions_page(
        &self,
        concept_row_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeOkfConceptRevision>, Option<u64>, bool), KnowledgeOkfConceptStoreError>
    {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let concept_row_id = to_i64("concept_row_id", concept_row_id)?;
        let page_size = validate_page_size(page_size)?;
        let fetch_limit = page_size + 1;
        let cursor = cursor
            .map(|value| to_i64("revision cursor", value))
            .transpose()?;
        let rows = if let Some(after_revision_no) = cursor {
            sqlx::query(
                r#"
                SELECT
                    id,
                    concept_row_id,
                    revision_no,
                    markdown_object_ref_id,
                    content_hash,
                    review_state,
                    CAST(created_at AS TEXT) AS created_at
                FROM kb_okf_concept_revision
                WHERE tenant_id = $1 AND organization_id = $2 AND concept_row_id = $3 AND status = $4
                  AND revision_no > $5
                ORDER BY revision_no ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(concept_row_id)
            .bind(ACTIVE_STATUS)
            .bind(after_revision_no)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error)?
        } else {
            sqlx::query(
                r#"
                SELECT
                    id,
                    concept_row_id,
                    revision_no,
                    markdown_object_ref_id,
                    content_hash,
                    review_state,
                    CAST(created_at AS TEXT) AS created_at
                FROM kb_okf_concept_revision
                WHERE tenant_id = $1 AND organization_id = $2 AND concept_row_id = $3 AND status = $4
                ORDER BY revision_no ASC
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(concept_row_id)
            .bind(ACTIVE_STATUS)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error)?
        };

        let has_more = rows.len() > page_size as usize;
        let mut items = Vec::with_capacity(rows.len().min(page_size as usize));
        let mut next_position = None;
        for row in rows.iter().take(page_size as usize) {
            let revision = revision_from_row(row)?;
            next_position = Some(revision.revision_no);
            items.push(revision);
        }
        let next_cursor = if has_more { next_position } else { None };
        Ok((items, next_cursor, has_more))
    }

    pub async fn get_revision_by_id(
        &self,
        revision_id: u64,
    ) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let revision_id = to_i64("revision_id", revision_id)?;
        let row = sqlx::query(
            r#"
            SELECT
                id,
                concept_row_id,
                revision_no,
                markdown_object_ref_id,
                content_hash,
                review_state,
                CAST(created_at AS TEXT) AS created_at
            FROM kb_okf_concept_revision
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(revision_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?
        .ok_or_else(|| {
            KnowledgeOkfConceptStoreError::Internal(format!("missing okf revision: {revision_id}"))
        })?;

        revision_from_row(&row)
    }

    pub async fn update_concept_publish_state(
        &self,
        concept_row_id: u64,
        publish_state: OkfConceptPublishState,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let concept_row_id = to_i64("concept_row_id", concept_row_id)?;
        let now = now_rfc3339()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_okf_concept
            SET publish_state = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
            RETURNING
                id,
                space_id,
                concept_id,
                title,
                concept_type,
                logical_path,
                description,
                source_count,
                CAST(tags AS TEXT) AS tags,
                current_revision_id,
                publish_state,
                CAST(updated_at AS TEXT) AS updated_at
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(publish_state.as_str())
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(concept_row_id)
            .bind(ACTIVE_STATUS)
            .fetch_optional(&self.pool)
            .await
            .map_err(sqlx_error)?
            .ok_or_else(|| {
                KnowledgeOkfConceptStoreError::Internal(format!(
                    "missing okf concept: {concept_row_id}"
                ))
            })?;

        concept_from_row(&row)
    }
}

#[async_trait]
impl KnowledgeOkfConceptStore for PostgresKnowledgeOkfConceptStore {
    async fn upsert_concept(
        &self,
        record: UpsertKnowledgeOkfConceptRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let mut transaction = self.pool.begin().await.map_err(sqlx_error)?;
        let concept = upsert_okf_concept_in_transaction(
            &mut transaction,
            self.tenant_id,
            self.organization_id,
            &self.id_generator,
            self.timestamp_dialect,
            record,
        )
        .await?;
        transaction.commit().await.map_err(sqlx_error)?;
        Ok(concept)
    }

    async fn create_revision(
        &self,
        record: CreateKnowledgeOkfConceptRevisionRecord,
    ) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let concept_row_id = to_i64("concept_row_id", record.concept_row_id)?;
        let revision_no = to_i64("revision_no", record.revision_no)?;
        let markdown_object_ref_id =
            to_i64("markdown_object_ref_id", record.markdown_object_ref_id)?;
        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let now = now_rfc3339()?;
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$11");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$12");

        let query = format!(
            r#"
            INSERT INTO kb_okf_concept_revision (
                id,
                uuid,
                tenant_id,
                organization_id,
                concept_row_id,
                revision_no,
                markdown_object_ref_id,
                content_hash,
                review_state,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, {created_at_expr}, {updated_at_expr}, $13)
            RETURNING
                id,
                concept_row_id,
                revision_no,
                markdown_object_ref_id,
                content_hash,
                review_state,
                CAST(created_at AS TEXT) AS created_at
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(concept_row_id)
            .bind(revision_no)
            .bind(markdown_object_ref_id)
            .bind(record.content_hash)
            .bind(record.review_state.as_str())
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error)?;

        revision_from_row(&row)
    }

    async fn next_revision_no(
        &self,
        concept_row_id: u64,
    ) -> Result<u64, KnowledgeOkfConceptStoreError> {
        PostgresKnowledgeOkfConceptStore::next_revision_no(self, concept_row_id).await
    }

    async fn mark_current_revision(
        &self,
        record: MarkKnowledgeOkfConceptCurrentRevisionRecord,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let concept_row_id = to_i64("concept_row_id", record.concept_row_id)?;
        let revision_id = to_i64("revision_id", record.revision_id)?;
        let now = now_rfc3339()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$3");

        let query = format!(
            r#"
            UPDATE kb_okf_concept
            SET current_revision_id = $1,
                publish_state = $2,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $4 AND organization_id = $5 AND id = $6 AND status = $7
            RETURNING
                id,
                space_id,
                concept_id,
                title,
                concept_type,
                logical_path,
                description,
                source_count,
                CAST(tags AS TEXT) AS tags,
                current_revision_id,
                publish_state,
                CAST(updated_at AS TEXT) AS updated_at
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(revision_id)
            .bind(record.publish_state.as_str())
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(concept_row_id)
            .bind(ACTIVE_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error)?;

        concept_from_row(&row)
    }

    async fn list_concept_summaries(
        &self,
        space_id: u64,
        limit: Option<u32>,
    ) -> Result<Vec<OkfConceptSummary>, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let limit = i64::from(
            limit
                .unwrap_or(MAX_OKF_LIST_ROWS as u32)
                .clamp(1, MAX_OKF_LIST_ROWS as u32),
        );
        let rows = sqlx::query(
            r#"
            SELECT
                title,
                concept_id,
                concept_type,
                logical_path,
                description,
                source_count,
                CAST(updated_at AS TEXT) AS updated_at,
                CAST(tags AS TEXT) AS tags
            FROM kb_okf_concept
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
              AND publish_state = 'published'
            ORDER BY concept_id ASC
            LIMIT $5
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error)?;

        rows.into_iter().map(description_from_row).collect()
    }

    async fn list_concept_summaries_page(
        &self,
        space_id: u64,
        cursor: Option<String>,
        page_size: u32,
    ) -> Result<(Vec<OkfConceptSummary>, Option<String>, bool), KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let page_size = validate_page_size(page_size)?;
        let fetch_limit = page_size + 1;

        let rows = if let Some(after_concept_id) = cursor {
            sqlx::query(
                r#"
                SELECT
                    id,
                    title,
                    concept_id,
                    concept_type,
                    logical_path,
                    description,
                    source_count,
                    CAST(updated_at AS TEXT) AS updated_at,
                    CAST(tags AS TEXT) AS tags
                FROM kb_okf_concept
                WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
                  AND publish_state = 'published' AND concept_id > $5
                ORDER BY concept_id ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(ACTIVE_STATUS)
            .bind(after_concept_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error)?
        } else {
            sqlx::query(
                r#"
                SELECT
                    id,
                    title,
                    concept_id,
                    concept_type,
                    logical_path,
                    description,
                    source_count,
                    CAST(updated_at AS TEXT) AS updated_at,
                    CAST(tags AS TEXT) AS tags
                FROM kb_okf_concept
                WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
                  AND publish_state = 'published'
                ORDER BY concept_id ASC
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(ACTIVE_STATUS)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error)?
        };

        let has_more = rows.len() > page_size as usize;
        let mut items = Vec::with_capacity(rows.len().min(page_size as usize));
        let mut last_concept_id = None;
        for row in rows.into_iter().take(page_size as usize) {
            last_concept_id = Some(row.try_get::<String, _>("concept_id").map_err(sqlx_error)?);
            items.push(description_from_row(row)?);
        }
        let next_cursor = if has_more { last_concept_id } else { None };
        Ok((items, next_cursor, has_more))
    }

    async fn list_concept_revisions_page(
        &self,
        concept_row_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeOkfConceptRevision>, Option<u64>, bool), KnowledgeOkfConceptStoreError>
    {
        PostgresKnowledgeOkfConceptStore::list_concept_revisions_page(
            self,
            concept_row_id,
            cursor,
            page_size,
        )
        .await
    }

    async fn append_log_entry(
        &self,
        record: AppendKnowledgeOkfLogEntryRecord,
    ) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", record.space_id)?;
        let now = now_rfc3339()?;
        let space_updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let sequence_query = format!(
            r#"
            UPDATE kb_space
            SET okf_log_sequence_counter = okf_log_sequence_counter + 1,
                updated_at = {space_updated_at_expr},
                version = version + 1
            WHERE tenant_id = $2 AND organization_id = $3 AND id = $4 AND status = $5
            RETURNING okf_log_sequence_counter
            "#,
        );
        // The counter increment and the log row share one transaction so a failed insert
        // cannot consume a sequence number without producing a log entry.
        let mut transaction = self.pool.begin().await.map_err(sqlx_error)?;
        let sequence_no: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(sequence_query.as_str()))
            .bind(now.clone())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(ACTIVE_STATUS)
            .fetch_one(&mut *transaction)
            .await
            .map_err(sqlx_error)?;

        let metadata = log_metadata_to_json(
            &record.actor,
            &record.affected_concepts,
            record.audit_event_id.as_deref(),
            &record.warnings,
        )?;
        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let event_time_expr = self.timestamp_dialect.sql_timestamp_expr("$8");
        let metadata_expr = self.timestamp_dialect.sql_json_expr("$11");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$14");
        let insert_query = format!(
            r#"
            INSERT INTO kb_okf_log_entry (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                sequence_no,
                event_type,
                event_time,
                title,
                privacy_level,
                metadata,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, {event_time_expr}, $9, $10, {metadata_expr}, $12, {created_at_expr}, {updated_at_expr}, $15)
            RETURNING
                event_type,
                CAST(event_time AS TEXT) AS event_time,
                title,
                CAST(metadata AS TEXT) AS metadata
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(insert_query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(sequence_no)
            .bind(record.event_type)
            .bind(record.event_time)
            .bind(record.title)
            .bind(record.privacy_level)
            .bind(metadata)
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_one(&mut *transaction)
            .await
            .map_err(sqlx_error)?;

        transaction.commit().await.map_err(sqlx_error)?;
        log_entry_from_row(&row)
    }

    async fn list_log_entries(
        &self,
        space_id: u64,
    ) -> Result<Vec<OkfLogEntry>, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let rows = sqlx::query(
            r#"
            SELECT event_type, CAST(event_time AS TEXT) AS event_time, title, CAST(metadata AS TEXT) AS metadata
            FROM kb_okf_log_entry
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
            ORDER BY sequence_no DESC
            LIMIT $5
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .bind(MAX_OKF_LIST_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error)?;

        rows.into_iter()
            .map(|row| log_entry_from_row(&row))
            .collect()
    }

    async fn batch_concept_projections_by_paths(
        &self,
        space_id: u64,
        logical_paths: Vec<String>,
    ) -> Result<Vec<KnowledgeOkfConceptProjection>, KnowledgeOkfConceptStoreError> {
        if logical_paths.is_empty() {
            return Ok(vec![]);
        }
        validate_projection_batch_size(logical_paths.len())?;

        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let mut builder = QueryBuilder::<Any>::new(
            r#"
            SELECT logical_path, id, current_revision_id, publish_state
            FROM kb_okf_concept
            WHERE organization_id =
            "#,
        );
        builder.push_bind(organization_id);
        builder.push(" AND tenant_id = ");
        builder.push_bind(tenant_id);
        builder.push(" AND space_id = ");
        builder.push_bind(space_id);
        builder.push(" AND status = ");
        builder.push_bind(ACTIVE_STATUS);
        builder.push(" AND logical_path IN (");
        let mut separated = builder.separated(", ");
        for path in logical_paths {
            separated.push_bind(path);
        }
        separated.push_unseparated(")");

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error)?;

        rows.into_iter().map(projection_from_row).collect()
    }

    async fn mark_concept_deleted(
        &self,
        space_id: u64,
        concept_row_id: u64,
    ) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let concept_row_id = to_i64("concept_row_id", concept_row_id)?;
        let now = now_rfc3339()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let update_concept_query = format!(
            r#"
            UPDATE kb_okf_concept
            SET status = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND space_id = $5 AND id = $6 AND status = $7
            RETURNING
                id,
                space_id,
                concept_id,
                title,
                concept_type,
                logical_path,
                description,
                source_count,
                CAST(tags AS TEXT) AS tags,
                current_revision_id,
                publish_state,
                CAST(updated_at AS TEXT) AS updated_at
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(update_concept_query.as_str()))
            .bind(DELETED_STATUS)
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(concept_row_id)
            .bind(ACTIVE_STATUS)
            .fetch_optional(&self.pool)
            .await
            .map_err(sqlx_error)?
            .ok_or_else(|| {
                KnowledgeOkfConceptStoreError::Internal(format!(
                    "missing okf concept: {concept_row_id}"
                ))
            })?;

        let update_revision_query = format!(
            r#"
            UPDATE kb_okf_concept_revision
            SET status = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND concept_row_id = $5 AND status = $6
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(update_revision_query.as_str()))
            .bind(DELETED_STATUS)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(concept_row_id)
            .bind(ACTIVE_STATUS)
            .execute(&self.pool)
            .await
            .map_err(sqlx_error)?;

        concept_from_row(&row)
    }
}

fn validate_projection_batch_size(len: usize) -> Result<(), KnowledgeOkfConceptStoreError> {
    if len > MAX_PROJECTION_BATCH_SIZE {
        return Err(KnowledgeOkfConceptStoreError::Internal(format!(
            "logical_paths batch size must be <= {MAX_PROJECTION_BATCH_SIZE}"
        )));
    }
    Ok(())
}

fn validate_page_size(page_size: u32) -> Result<i64, KnowledgeOkfConceptStoreError> {
    let max_page_size = MAX_OKF_LIST_ROWS as u32;
    if !(1..=max_page_size).contains(&page_size) {
        return Err(KnowledgeOkfConceptStoreError::Internal(format!(
            "page_size must be between 1 and {max_page_size}"
        )));
    }
    Ok(i64::from(page_size))
}

fn concept_from_row(row: &AnyRow) -> Result<KnowledgeOkfConcept, KnowledgeOkfConceptStoreError> {
    let concept_type: String = row.try_get("concept_type").map_err(sqlx_error)?;
    let publish_state: String = row.try_get("publish_state").map_err(sqlx_error)?;
    let source_count: i64 = row.try_get("source_count").map_err(sqlx_error)?;
    Ok(KnowledgeOkfConcept {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(sqlx_error)?)?,
        concept_id: row.try_get("concept_id").map_err(sqlx_error)?,
        title: row.try_get("title").map_err(sqlx_error)?,
        concept_type,
        logical_path: row.try_get("logical_path").map_err(sqlx_error)?,
        bundle_relative_path: bundle_relative_path_from_logical_path(
            &row.try_get::<String, _>("logical_path")
                .map_err(sqlx_error)?,
        ),
        description: row.try_get("description").map_err(sqlx_error)?,
        source_count: u32::try_from(source_count).map_err(|_| {
            KnowledgeOkfConceptStoreError::Internal("source_count is out of range".to_string())
        })?,
        tags: tags_from_json(row.try_get("tags").map_err(sqlx_error)?)?,
        current_revision_id: row
            .try_get::<Option<i64>, _>("current_revision_id")
            .map_err(sqlx_error)?
            .map(|value| from_i64("current_revision_id", value))
            .transpose()?,
        publish_state: publish_state_from_str(&publish_state)?,
        updated_at: row.try_get("updated_at").map_err(sqlx_error)?,
    })
}

fn revision_from_row(
    row: &AnyRow,
) -> Result<KnowledgeOkfConceptRevision, KnowledgeOkfConceptStoreError> {
    let review_state: String = row.try_get("review_state").map_err(sqlx_error)?;
    Ok(KnowledgeOkfConceptRevision {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        concept_row_id: from_i64(
            "concept_row_id",
            row.try_get("concept_row_id").map_err(sqlx_error)?,
        )?,
        revision_no: from_i64(
            "revision_no",
            row.try_get("revision_no").map_err(sqlx_error)?,
        )?,
        markdown_object_ref_id: from_i64(
            "markdown_object_ref_id",
            row.try_get("markdown_object_ref_id").map_err(sqlx_error)?,
        )?,
        content_hash: row.try_get("content_hash").map_err(sqlx_error)?,
        review_state: review_state_from_str(&review_state)?,
        created_at: row.try_get("created_at").map_err(sqlx_error)?,
    })
}

fn description_from_row(row: AnyRow) -> Result<OkfConceptSummary, KnowledgeOkfConceptStoreError> {
    let concept_type: String = row.try_get("concept_type").map_err(sqlx_error)?;
    let source_count: i64 = row.try_get("source_count").map_err(sqlx_error)?;
    Ok(OkfConceptSummary {
        title: row.try_get("title").map_err(sqlx_error)?,
        concept_id: row.try_get("concept_id").map_err(sqlx_error)?,
        concept_type,
        logical_path: row.try_get("logical_path").map_err(sqlx_error)?,
        bundle_relative_path: bundle_relative_path_from_logical_path(
            &row.try_get::<String, _>("logical_path")
                .map_err(sqlx_error)?,
        ),
        description: row.try_get("description").map_err(sqlx_error)?,
        source_count: u32::try_from(source_count).map_err(|_| {
            KnowledgeOkfConceptStoreError::Internal("source_count is out of range".to_string())
        })?,
        updated_at: row.try_get("updated_at").map_err(sqlx_error)?,
        tags: tags_from_json(row.try_get("tags").map_err(sqlx_error)?)?,
    })
}

fn projection_from_row(
    row: AnyRow,
) -> Result<KnowledgeOkfConceptProjection, KnowledgeOkfConceptStoreError> {
    let publish_state: String = row.try_get("publish_state").map_err(sqlx_error)?;
    Ok(KnowledgeOkfConceptProjection {
        logical_path: row.try_get("logical_path").map_err(sqlx_error)?,
        concept_row_id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        current_revision_id: row
            .try_get::<Option<i64>, _>("current_revision_id")
            .map_err(sqlx_error)?
            .map(|value| from_i64("current_revision_id", value))
            .transpose()?,
        publish_state: publish_state_from_str(&publish_state)?,
    })
}

fn log_entry_from_row(row: &AnyRow) -> Result<OkfLogEntry, KnowledgeOkfConceptStoreError> {
    let event_type: String = row.try_get("event_type").map_err(sqlx_error)?;
    let metadata: Option<String> = row.try_get("metadata").map_err(sqlx_error)?;
    let metadata = log_metadata_from_json(metadata)?;
    Ok(OkfLogEntry {
        occurred_at: row.try_get("event_time").map_err(sqlx_error)?,
        event_type: log_event_type_from_str(&event_type)?,
        title: row.try_get("title").map_err(sqlx_error)?,
        actor: metadata.actor,
        affected_concepts: metadata.affected_concepts,
        audit_event_id: metadata.audit_event_id,
        warnings: metadata.warnings,
    })
}

fn bundle_relative_path_from_logical_path(logical_path: &str) -> String {
    logical_path
        .strip_prefix("okf/")
        .unwrap_or(logical_path)
        .to_string()
}

fn publish_state_from_str(
    value: &str,
) -> Result<OkfConceptPublishState, KnowledgeOkfConceptStoreError> {
    match value {
        "draft" => Ok(OkfConceptPublishState::Draft),
        "candidate_ready" => Ok(OkfConceptPublishState::CandidateReady),
        "needs_review" => Ok(OkfConceptPublishState::NeedsReview),
        "published" => Ok(OkfConceptPublishState::Published),
        "stale" => Ok(OkfConceptPublishState::Stale),
        "rejected" => Ok(OkfConceptPublishState::Rejected),
        "failed" => Ok(OkfConceptPublishState::Failed),
        _ => Err(KnowledgeOkfConceptStoreError::Internal(format!(
            "unknown okf concept publish state: {value}"
        ))),
    }
}

fn review_state_from_str(
    value: &str,
) -> Result<OkfRevisionReviewState, KnowledgeOkfConceptStoreError> {
    match value {
        "pending" => Ok(OkfRevisionReviewState::Pending),
        "approved" => Ok(OkfRevisionReviewState::Approved),
        "rejected" => Ok(OkfRevisionReviewState::Rejected),
        _ => Err(KnowledgeOkfConceptStoreError::Internal(format!(
            "unknown okf revision review state: {value}"
        ))),
    }
}

fn log_event_type_from_str(value: &str) -> Result<OkfLogEventType, KnowledgeOkfConceptStoreError> {
    match value {
        "ingest" => Ok(OkfLogEventType::Ingest),
        "query" => Ok(OkfLogEventType::Query),
        "filed_answer" => Ok(OkfLogEventType::FiledAnswer),
        "compile" => Ok(OkfLogEventType::Compile),
        "review" => Ok(OkfLogEventType::Review),
        "publish" => Ok(OkfLogEventType::Publish),
        "lint" => Ok(OkfLogEventType::Lint),
        "eval" => Ok(OkfLogEventType::Eval),
        "package" => Ok(OkfLogEventType::Package),
        "mirror" => Ok(OkfLogEventType::Mirror),
        "delta_update" => Ok(OkfLogEventType::DeltaUpdate),
        _ => Err(KnowledgeOkfConceptStoreError::Internal(format!(
            "unknown okf log event type: {value}"
        ))),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogMetadata {
    actor: String,
    affected_concepts: Vec<String>,
    audit_event_id: Option<String>,
    warnings: Vec<String>,
}

fn tags_from_json(value: Option<String>) -> Result<Vec<String>, KnowledgeOkfConceptStoreError> {
    match value {
        Some(value) if !is_blank(Some(value.as_str())) => serde_json::from_str(&value)
            .map_err(|error| KnowledgeOkfConceptStoreError::Internal(error.to_string())),
        _ => Ok(vec![]),
    }
}

fn log_metadata_to_json(
    actor: &str,
    affected_concepts: &[String],
    audit_event_id: Option<&str>,
    warnings: &[String],
) -> Result<String, KnowledgeOkfConceptStoreError> {
    serde_json::to_string(&LogMetadata {
        actor: actor.to_string(),
        affected_concepts: affected_concepts.to_vec(),
        audit_event_id: audit_event_id.map(str::to_string),
        warnings: warnings.to_vec(),
    })
    .map_err(|error| KnowledgeOkfConceptStoreError::Internal(error.to_string()))
}

fn log_metadata_from_json(
    value: Option<String>,
) -> Result<LogMetadata, KnowledgeOkfConceptStoreError> {
    match value {
        Some(value) if !is_blank(Some(value.as_str())) => serde_json::from_str(&value)
            .map_err(|error| KnowledgeOkfConceptStoreError::Internal(error.to_string())),
        _ => Ok(LogMetadata {
            actor: "system".to_string(),
            affected_concepts: vec![],
            audit_event_id: None,
            warnings: vec![],
        }),
    }
}

fn now_rfc3339() -> Result<String, KnowledgeOkfConceptStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeOkfConceptStoreError::Internal(error.to_string()))
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeOkfConceptStoreError> {
    i64::try_from(value)
        .map_err(|_| KnowledgeOkfConceptStoreError::Internal(format!("{field} is out of range")))
}

fn from_i64(field: &str, value: i64) -> Result<u64, KnowledgeOkfConceptStoreError> {
    u64::try_from(value)
        .map_err(|_| KnowledgeOkfConceptStoreError::Internal(format!("{field} is negative")))
}

fn sqlx_error(error: sqlx::Error) -> KnowledgeOkfConceptStoreError {
    KnowledgeOkfConceptStoreError::Internal(error.to_string())
}

fn id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeOkfConceptStoreError {
    KnowledgeOkfConceptStoreError::Internal(error.to_string())
}
