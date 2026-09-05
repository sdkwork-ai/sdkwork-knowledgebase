use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope, KnowledgeDocumentStore,
    KnowledgeDocumentStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_version_store::{
    CreateKnowledgeDocumentVersionRecord, CreateNextKnowledgeDocumentVersionRecord,
    KnowledgeDocumentVersionStore, KnowledgeDocumentVersionStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::{
    ClaimIngestionJobsRequest, ClaimedIngestionJob, CompleteRunningIngestionRecord,
    CompletedIngestionResult, CreateIngestionJobRecord, CreateOrGetIngestionJobResult,
    DriveImportJobLinkage, IngestionJobLifecycle, IngestionJobStore, IngestionJobStoreError,
    KNOWLEDGE_UPLOAD_SESSION_TTL, MAX_INGESTION_JOB_LEASE,
    STALE_UPLOAD_SESSION_RECOVERY_BATCH_SIZE,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_outbox_store::AppendOutboxEventRecord;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_source_store::{
    CreateKnowledgeSourceRecord, KnowledgeSourceLineageSnapshot, KnowledgeSourceStore,
    KnowledgeSourceStoreError,
};
use sdkwork_intelligence_knowledgebase_service::tenant_quota::{
    ensure_document_capacity, ensure_ingest_concurrency,
};
use sdkwork_knowledgebase_contract::document::{
    KnowledgeDocument, KnowledgeDocumentState, KnowledgeDocumentVersion,
    KnowledgeDocumentVersionState, KnowledgeDocumentVisibility,
};
use sdkwork_knowledgebase_contract::ingest::{IngestionJob, IngestionJobState};
use sdkwork_knowledgebase_contract::source::{KnowledgeSource, KnowledgeSourceType};
use sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits;
use sdkwork_utils_rust::{is_blank, truncate};
use sqlx::{any::AnyRow, AnyConnection, AnyPool, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use uuid::Uuid;

use crate::chunk_transaction::{
    replace_version_chunks_in_transaction, ReplaceVersionChunksContext,
};
use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};
use crate::keyword_search::KeywordSearchBackend;
use crate::quota_transaction::begin_tenant_quota_transaction;

const ACTIVE_STATUS: i64 = 1;
const INITIAL_VERSION: i64 = 0;
const OUTBOX_STATUS_PENDING: i64 = 0;

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeSourceStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeSourceStore {
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
}

#[async_trait]
impl KnowledgeSourceStore for PostgresKnowledgeSourceStore {
    async fn create_source(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        self.insert_source(record).await
    }

    async fn create_or_get_source(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        if let Some(source) = self.insert_source_if_absent(record.clone()).await? {
            return Ok(source);
        }

        self.get_source_by_identity(&record).await
    }

    async fn newest_lineage_activity_at(
        &self,
        space_id: u64,
    ) -> Result<Option<String>, KnowledgeSourceStoreError> {
        self.query_newest_lineage_activity_at(space_id).await
    }

    async fn list_space_source_lineage(
        &self,
        space_id: u64,
    ) -> Result<Vec<KnowledgeSourceLineageSnapshot>, KnowledgeSourceStoreError> {
        self.query_space_source_lineage(space_id).await
    }

    async fn list_sources_for_space(
        &self,
        space_id: u64,
    ) -> Result<Vec<KnowledgeSource>, KnowledgeSourceStoreError> {
        self.query_sources_for_space(space_id).await
    }
}

impl PostgresKnowledgeSourceStore {
    async fn get_source_by_identity(
        &self,
        record: &CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let space_id = source_to_i64("space_id", record.space_id)?;
        let source_type_value = record.source_type.as_str().to_string();
        let row = sqlx::query(
            r#"
            SELECT id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
            FROM kb_source
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND source_type = $4
              AND COALESCE(provider, '') = COALESCE($5, '')
              AND COALESCE(drive_bucket, '') = COALESCE($6, '')
              AND COALESCE(drive_prefix, '') = COALESCE($7, '')
              AND status = $8
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(source_type_value)
        .bind(&record.provider)
        .bind(&record.drive_bucket)
        .bind(&record.drive_prefix)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(source_sqlx_error)?;

        source_from_row(&row)
    }

    async fn insert_source_if_absent(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<Option<KnowledgeSource>, KnowledgeSourceStoreError> {
        self.insert_source_with_conflict_clause(record, "ON CONFLICT DO NOTHING")
            .await
    }

    async fn insert_source(
        &self,
        record: CreateKnowledgeSourceRecord,
    ) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
        self.insert_source_with_conflict_clause(record, "")
            .await?
            .ok_or_else(|| {
                KnowledgeSourceStoreError::Internal("failed to insert source".to_string())
            })
    }

    async fn insert_source_with_conflict_clause(
        &self,
        record: CreateKnowledgeSourceRecord,
        conflict_clause: &str,
    ) -> Result<Option<KnowledgeSource>, KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let space_id = source_to_i64("space_id", record.space_id)?;
        let id = next_i64_id(&self.id_generator).map_err(source_id_error)?;
        let now = source_now()?;
        let source_type_value = record.source_type.as_str().to_string();
        let metadata_expr = self.timestamp_dialect.sql_json_expr("$10");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$12");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let query = format!(
            r#"
            INSERT INTO kb_source (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                source_type,
                provider,
                drive_bucket,
                drive_prefix,
                metadata,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, {metadata_expr}, $11, {created_at_expr}, {updated_at_expr}, $14)
            {conflict_clause}
            RETURNING id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
            "#
        );

        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(source_type_value)
            .bind(record.provider.clone())
            .bind(record.drive_bucket.clone())
            .bind(record.drive_prefix.clone())
            .bind(record.connector_metadata_json.clone())
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_optional(&self.pool)
            .await
            .map_err(source_sqlx_error)?;

        row.map(|row| source_from_row(&row)).transpose()
    }

    pub async fn query_newest_lineage_activity_at(
        &self,
        space_id: u64,
    ) -> Result<Option<String>, KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let space_id = source_to_i64("space_id", space_id)?;
        let newest = sqlx::query_scalar::<_, Option<String>>(
            r#"
            SELECT CAST(MAX(COALESCE(last_sync_at, updated_at)) AS TEXT)
            FROM kb_source
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(source_sqlx_error)?;
        Ok(newest)
    }

    pub async fn query_space_source_lineage(
        &self,
        space_id: u64,
    ) -> Result<Vec<KnowledgeSourceLineageSnapshot>, KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let space_id = source_to_i64("space_id", space_id)?;
        let rows = sqlx::query(
            r#"
            SELECT id,
                   CAST(updated_at AS TEXT) AS updated_at,
                   CAST(last_sync_at AS TEXT) AS last_sync_at,
                   provider,
                   drive_bucket,
                   drive_prefix
            FROM kb_source
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
            ORDER BY id ASC
            LIMIT 200
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .fetch_all(&self.pool)
        .await
        .map_err(source_sqlx_error)?;

        rows.iter()
            .map(|row| {
                Ok(KnowledgeSourceLineageSnapshot {
                    source_id: source_from_i64(
                        "id",
                        row.try_get("id").map_err(source_sqlx_error)?,
                    )?,
                    updated_at: row.try_get("updated_at").map_err(source_sqlx_error)?,
                    last_sync_at: row.try_get("last_sync_at").map_err(source_sqlx_error)?,
                    provider: row.try_get("provider").map_err(source_sqlx_error)?,
                    drive_bucket: row.try_get("drive_bucket").map_err(source_sqlx_error)?,
                    drive_prefix: row.try_get("drive_prefix").map_err(source_sqlx_error)?,
                })
            })
            .collect()
    }

    pub async fn list_active_sources(
        &self,
    ) -> Result<Vec<KnowledgeSource>, KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let rows = sqlx::query(
            r#"
            SELECT id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
            FROM kb_source
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
            ORDER BY id ASC
            LIMIT 200
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .fetch_all(&self.pool)
        .await
        .map_err(source_sqlx_error)?;

        rows.iter().map(source_from_row).collect()
    }

    pub async fn list_active_sources_page(
        &self,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeSource>, Option<String>, bool), KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let cursor = cursor
            .map(|value| source_to_i64("cursor", value))
            .transpose()?;
        let page_size = page_size.clamp(1, 200) as usize;
        let rows = sqlx::query(
            r#"
            SELECT id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
            FROM kb_source
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3 AND ($4 IS NULL OR id > $4)
            ORDER BY id ASC
            LIMIT $5
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .bind(cursor)
        .bind((page_size + 1) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(source_sqlx_error)?;

        let has_more = rows.len() > page_size;
        let items = rows
            .iter()
            .take(page_size)
            .map(source_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.id.to_string()))
            .flatten();
        Ok((items, next_cursor, has_more))
    }

    async fn query_sources_for_space(
        &self,
        space_id: u64,
    ) -> Result<Vec<KnowledgeSource>, KnowledgeSourceStoreError> {
        let tenant_id = source_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = source_to_i64("organization_id", self.organization_id)?;
        let space_id = source_to_i64("space_id", space_id)?;
        let rows = sqlx::query(
            r#"
            SELECT id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
            FROM kb_source
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
            ORDER BY id ASC
            LIMIT 200
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .fetch_all(&self.pool)
        .await
        .map_err(source_sqlx_error)?;

        rows.iter().map(source_from_row).collect()
    }
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeDocumentStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    database_engine: DatabaseEngine,
    quota_limits: Option<KnowledgebaseTenantQuotaLimits>,
}

impl PostgresKnowledgeDocumentStore {
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
            database_engine: DatabaseEngine::Postgres,
            quota_limits: None,
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self.database_engine = database_engine;
        self
    }

    pub fn with_quota_limits(mut self, quota_limits: KnowledgebaseTenantQuotaLimits) -> Self {
        self.quota_limits = Some(quota_limits);
        self
    }
}

#[async_trait]
impl KnowledgeDocumentStore for PostgresKnowledgeDocumentStore {
    async fn create_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        if let Some(limits) = self.quota_limits {
            return self.create_document_with_quota(record, limits).await;
        }
        self.insert_document(record).await
    }

    async fn create_or_get_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        if let Some(limits) = self.quota_limits {
            return self.create_or_get_document_with_quota(record, limits).await;
        }
        if let Some(document) = self.insert_document_if_absent(record.clone()).await? {
            return Ok(document);
        }

        let document = self.get_document_by_identity(&record).await?;
        self.enrich_document_drive_node_binding(
            document,
            record.original_file_drive_node_id.as_deref(),
        )
        .await
    }

    async fn get_document_by_id(
        &self,
        document_id: u64,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        PostgresKnowledgeDocumentStore::get_document_by_id(self, document_id).await
    }

    async fn list_documents_for_space(
        &self,
        space_id: u64,
        limit: u32,
    ) -> Result<Vec<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        let (items, _, _) = self.list_documents_page(space_id, None, limit).await?;
        Ok(items)
    }

    async fn list_documents_page(
        &self,
        space_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeDocument>, Option<String>, bool), KnowledgeDocumentStoreError> {
        self.list_documents_page_for_space(space_id, cursor, page_size)
            .await
    }
}

impl PostgresKnowledgeDocumentStore {
    async fn create_document_with_quota(
        &self,
        record: CreateKnowledgeDocumentRecord,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(document_sqlx_error)?;
        self.ensure_document_quota_on(&mut transaction, limits)
            .await?;
        let document = self
            .insert_document_with_conflict_clause_on(record, "", &mut transaction)
            .await?
            .ok_or_else(|| {
                KnowledgeDocumentStoreError::Internal("failed to insert document".to_string())
            })?;
        transaction.commit().await.map_err(document_sqlx_error)?;
        Ok(document)
    }

    async fn create_or_get_document_with_quota(
        &self,
        record: CreateKnowledgeDocumentRecord,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(document_sqlx_error)?;

        if let Some(document) = self
            .find_document_by_identity_on(&record, &mut transaction)
            .await?
        {
            transaction.commit().await.map_err(document_sqlx_error)?;
            return self
                .enrich_document_drive_node_binding(
                    document,
                    record.original_file_drive_node_id.as_deref(),
                )
                .await;
        }

        self.ensure_document_quota_on(&mut transaction, limits)
            .await?;
        let document = self
            .insert_document_with_conflict_clause_on(
                record.clone(),
                "ON CONFLICT DO NOTHING",
                &mut transaction,
            )
            .await?;
        let document = match document {
            Some(document) => document,
            None => self
                .find_document_by_identity_on(&record, &mut transaction)
                .await?
                .ok_or_else(|| {
                    KnowledgeDocumentStoreError::Internal(
                        "document identity conflict did not resolve to an active document"
                            .to_string(),
                    )
                })?,
        };
        transaction.commit().await.map_err(document_sqlx_error)?;
        self.enrich_document_drive_node_binding(
            document,
            record.original_file_drive_node_id.as_deref(),
        )
        .await
    }

    async fn ensure_document_quota_on(
        &self,
        connection: &mut AnyConnection,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<(), KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kb_document WHERE tenant_id = $1 AND organization_id = $2 AND status = $3",
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(connection)
        .await
        .map_err(document_sqlx_error)?;
        let count = u64::try_from(count.max(0))
            .map_err(|error| KnowledgeDocumentStoreError::Internal(error.to_string()))?;
        ensure_document_capacity(count, &limits)?;
        Ok(())
    }

    async fn get_document_by_identity(
        &self,
        record: &CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let mut connection = self.pool.acquire().await.map_err(document_sqlx_error)?;
        self.find_document_by_identity_on(record, &mut connection)
            .await?
            .ok_or_else(|| {
                KnowledgeDocumentStoreError::Internal(
                    "document identity did not resolve to an active document".to_string(),
                )
            })
    }

    async fn find_document_by_identity_on(
        &self,
        record: &CreateKnowledgeDocumentRecord,
        connection: &mut AnyConnection,
    ) -> Result<Option<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        validate_document_identity(record)?;

        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let space_id = document_to_i64("space_id", record.space_id)?;
        let collection_id = document_to_i64("collection_id", record.collection_id)?;
        let source_id = record
            .source_id
            .map(|value| document_to_i64("source_id", value))
            .transpose()?;
        let row = sqlx::query(
            r#"
            SELECT id, space_id, collection_id, source_id, original_file_drive_node_id, title, mime_type, language,
                   current_version_id, visibility, content_state, index_state
            FROM kb_document
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND collection_id = $4
              AND identity_scope = $5
              AND (
                  ($6 = 'source_only' AND source_id = $7)
                  OR (
                      $8 = 'source_and_original_drive_node'
                      AND (
                          ($9 IS NULL AND source_id IS NULL)
                          OR ($10 IS NOT NULL AND source_id = $11)
                      )
                      AND COALESCE(original_file_drive_node_id, '') = COALESCE($12, '')
                  )
              )
              AND status = $13
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(collection_id)
        .bind(record.identity_scope.as_str())
        .bind(record.identity_scope.as_str())
        .bind(source_id)
        .bind(record.identity_scope.as_str())
        .bind(source_id)
        .bind(source_id)
        .bind(source_id)
        .bind(&record.original_file_drive_node_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(connection)
        .await
        .map_err(document_sqlx_error)?;

        row.map(|row| document_from_row(&row)).transpose()
    }

    pub async fn list_active_documents(
        &self,
        limit: u32,
    ) -> Result<Vec<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 200));
        let rows = sqlx::query(
            r#"
            SELECT id, space_id, collection_id, source_id, original_file_drive_node_id, title, mime_type, language,
                   current_version_id, visibility, content_state, index_state
            FROM kb_document
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
            ORDER BY id ASC
            LIMIT $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(document_sqlx_error)?;

        rows.iter().map(document_from_row).collect()
    }

    pub async fn list_active_documents_for_space(
        &self,
        space_id: u64,
        limit: u32,
    ) -> Result<Vec<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        let (items, _, _) = self
            .list_documents_page_for_space(space_id, None, limit)
            .await?;
        Ok(items)
    }

    pub async fn list_documents_page_for_space(
        &self,
        space_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeDocument>, Option<String>, bool), KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let space_id = document_to_i64("space_id", space_id)?;
        let page_size = i64::from(page_size.clamp(1, 200));
        let fetch_limit = page_size + 1;
        let cursor_id = cursor
            .map(|value| document_to_i64("cursor", value))
            .transpose()?;

        let rows = if let Some(after_id) = cursor_id {
            sqlx::query(
                r#"
                SELECT id, space_id, collection_id, source_id, original_file_drive_node_id, title, mime_type, language,
                       current_version_id, visibility, content_state, index_state
                FROM kb_document
                WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4 AND id > $5
                ORDER BY id ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(ACTIVE_STATUS)
            .bind(after_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(document_sqlx_error)?
        } else {
            sqlx::query(
                r#"
                SELECT id, space_id, collection_id, source_id, original_file_drive_node_id, title, mime_type, language,
                       current_version_id, visibility, content_state, index_state
                FROM kb_document
                WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = $4
                ORDER BY id ASC
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
            .map_err(document_sqlx_error)?
        };

        let has_more = rows.len() > page_size as usize;
        let mut items = Vec::new();
        for row in rows.into_iter().take(page_size as usize) {
            items.push(document_from_row(&row)?);
        }
        let next_cursor = if has_more {
            items.last().map(|document| document.id.to_string())
        } else {
            None
        };

        Ok((items, next_cursor, has_more))
    }

    pub async fn get_document_by_id(
        &self,
        document_id: u64,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let document_id = document_to_i64("document_id", document_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, space_id, collection_id, source_id, original_file_drive_node_id, title, mime_type, language,
                   current_version_id, visibility, content_state, index_state
            FROM kb_document
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if matches!(error, sqlx::Error::RowNotFound) {
                KnowledgeDocumentStoreError::Internal(format!(
                    "knowledge document {document_id} was not found"
                ))
            } else {
                document_sqlx_error(error)
            }
        })?;

        document_from_row(&row)
    }

    pub async fn update_document_metadata(
        &self,
        document_id: u64,
        title: String,
        mime_type: Option<String>,
        language: Option<String>,
        visibility: Option<sdkwork_knowledgebase_contract::document::KnowledgeDocumentVisibility>,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        if is_blank(Some(title.as_str())) {
            return Err(KnowledgeDocumentStoreError::InvalidRecord(
                "title is required".to_string(),
            ));
        }
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let document_id = document_to_i64("document_id", document_id)?;
        let now = document_now()?;
        let visibility_code = visibility.map(document_visibility_code);
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$5");
        let query = format!(
            r#"
            UPDATE kb_document
            SET title = $1,
                mime_type = $2,
                language = $3,
                visibility = COALESCE($4, visibility),
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $6 AND organization_id = $7 AND id = $8 AND status = $9
            RETURNING id, space_id, collection_id, source_id, original_file_drive_node_id, title, mime_type, language,
                      current_version_id, visibility, content_state, index_state
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(title)
            .bind(mime_type)
            .bind(language)
            .bind(visibility_code)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                if matches!(error, sqlx::Error::RowNotFound) {
                    KnowledgeDocumentStoreError::Internal(format!(
                        "knowledge document {document_id} was not found"
                    ))
                } else {
                    document_sqlx_error(error)
                }
            })?;

        document_from_row(&row)
    }

    pub async fn soft_delete_document(
        &self,
        document_id: u64,
    ) -> Result<(), KnowledgeDocumentStoreError> {
        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let document_id = document_to_i64("document_id", document_id)?;
        let now = document_now()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let query = format!(
            r#"
            UPDATE kb_document
            SET status = 0, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $2 AND organization_id = $3 AND id = $4 AND status = $5
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .execute(&self.pool)
            .await
            .map_err(document_sqlx_error)?;

        if rows.rows_affected() == 0 {
            return Err(KnowledgeDocumentStoreError::Internal(format!(
                "knowledge document {document_id} was not found"
            )));
        }
        Ok(())
    }

    async fn enrich_document_drive_node_binding(
        &self,
        document: KnowledgeDocument,
        original_file_drive_node_id: Option<&str>,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        let Some(original_file_drive_node_id) = original_file_drive_node_id else {
            return Ok(document);
        };
        if document.original_file_drive_node_id.is_some() {
            return Ok(document);
        }

        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let document_id = document_to_i64("document_id", document.id)?;
        let now = document_now()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_document
            SET original_file_drive_node_id = COALESCE(original_file_drive_node_id, $1),
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
            RETURNING
                id,
                space_id,
                collection_id,
                source_id,
                identity_scope,
                original_file_drive_node_id,
                title,
                mime_type,
                language,
                current_version_id,
                visibility,
                content_state,
                index_state
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(original_file_drive_node_id)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(document_sqlx_error)?;

        document_from_row(&row)
    }

    async fn insert_document_if_absent(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<Option<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        self.insert_document_with_conflict_clause(record, "ON CONFLICT DO NOTHING")
            .await
    }

    async fn insert_document(
        &self,
        record: CreateKnowledgeDocumentRecord,
    ) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
        self.insert_document_with_conflict_clause(record, "")
            .await?
            .ok_or_else(|| {
                KnowledgeDocumentStoreError::Internal("failed to insert document".to_string())
            })
    }

    async fn insert_document_with_conflict_clause(
        &self,
        record: CreateKnowledgeDocumentRecord,
        conflict_clause: &str,
    ) -> Result<Option<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        let mut connection = self.pool.acquire().await.map_err(document_sqlx_error)?;
        self.insert_document_with_conflict_clause_on(record, conflict_clause, &mut connection)
            .await
    }

    async fn insert_document_with_conflict_clause_on(
        &self,
        record: CreateKnowledgeDocumentRecord,
        conflict_clause: &str,
        connection: &mut AnyConnection,
    ) -> Result<Option<KnowledgeDocument>, KnowledgeDocumentStoreError> {
        validate_document_identity(&record)?;

        let tenant_id = document_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = document_to_i64("organization_id", self.organization_id)?;
        let space_id = document_to_i64("space_id", record.space_id)?;
        let collection_id = document_to_i64("collection_id", record.collection_id)?;
        let source_id = record
            .source_id
            .map(|value| document_to_i64("source_id", value))
            .transpose()?;
        let generated_id = next_i64_id(&self.id_generator).map_err(document_id_error)?;
        let now = document_now()?;
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$17");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$18");
        let query = format!(
            r#"
            INSERT INTO kb_document (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                collection_id,
                source_id,
                identity_scope,
                original_file_drive_node_id,
                title,
                mime_type,
                language,
                visibility,
                content_state,
                index_state,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, {created_at_expr}, {updated_at_expr}, $19)
            {conflict_clause}
            RETURNING
                id,
                space_id,
                collection_id,
                source_id,
                original_file_drive_node_id,
                title,
                mime_type,
                language,
                current_version_id,
                visibility,
                content_state,
                index_state
            "#
        );

        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(generated_id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(collection_id)
            .bind(source_id)
            .bind(record.identity_scope.as_str())
            .bind(record.original_file_drive_node_id.clone())
            .bind(record.title.clone())
            .bind(record.mime_type.clone())
            .bind(record.language.clone())
            .bind(document_visibility_code(KnowledgeDocumentVisibility::Space))
            .bind(document_state_code(KnowledgeDocumentState::Ready))
            .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_optional(connection)
            .await
            .map_err(document_sqlx_error)?;

        row.map(|row| document_from_row(&row)).transpose()
    }
}

fn validate_document_identity(
    record: &CreateKnowledgeDocumentRecord,
) -> Result<(), KnowledgeDocumentStoreError> {
    if record.identity_scope == KnowledgeDocumentIdentityScope::SourceOnly
        && record.source_id.is_none()
    {
        return Err(KnowledgeDocumentStoreError::InvalidRecord(
            "source_only document identity requires source_id".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeDocumentVersionStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeDocumentVersionStore {
    pub async fn create_next_document_version(
        &self,
        record: CreateNextKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        let tenant_id = version_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = version_to_i64("organization_id", self.organization_id)?;
        let document_id = version_to_i64("document_id", record.document_id)?;
        let original_object_ref_id =
            version_to_i64("original_object_ref_id", record.original_object_ref_id)?;
        let size_bytes = version_to_i64("size_bytes", record.size_bytes)?;
        let generated_id = next_i64_id(&self.id_generator).map_err(version_id_error)?;
        let now = version_now()?;
        let mut transaction = self.pool.begin().await.map_err(version_sqlx_error)?;

        let locked = sqlx::query(
            "UPDATE kb_document SET version = version WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4",
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(ACTIVE_STATUS)
        .execute(&mut *transaction)
        .await
        .map_err(version_sqlx_error)?;
        if locked.rows_affected() != 1 {
            return Err(KnowledgeDocumentVersionStoreError::Internal(format!(
                "active document not found: {}",
                record.document_id
            )));
        }

        let latest_version_no = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT MAX(version_no)
            FROM kb_document_version
            WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&mut *transaction)
        .await
        .map_err(version_sqlx_error)?
        .unwrap_or(0);
        let version_no = latest_version_no.checked_add(1).ok_or_else(|| {
            KnowledgeDocumentVersionStoreError::Internal(
                "document version number exceeds the supported range".to_string(),
            )
        })?;

        let submitted_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$15");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$16");
        let insert_query = format!(
            r#"
            INSERT INTO kb_document_version (
                id, uuid, tenant_id, organization_id, document_id, version_no, original_object_ref_id,
                checksum_sha256_hex, size_bytes, mime_type, parse_state, index_state,
                submitted_at, status, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {submitted_at_expr}, $14, {created_at_expr}, {updated_at_expr}, $17)
            RETURNING id, document_id, version_no, original_object_ref_id,
                      checksum_sha256_hex, size_bytes, mime_type, parse_state, index_state
            "#
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(insert_query.as_str()))
            .bind(generated_id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(version_no)
            .bind(original_object_ref_id)
            .bind(record.checksum_sha256_hex)
            .bind(size_bytes)
            .bind(record.mime_type)
            .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
            .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
            .bind(now.clone())
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now.clone())
            .bind(INITIAL_VERSION)
            .fetch_one(&mut *transaction)
            .await
            .map_err(version_sqlx_error)?;
        let version = document_version_from_row(&row)?;

        let current_version_updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let update_document_query = format!(
            r#"
            UPDATE kb_document
            SET current_version_id = $1,
                updated_at = {current_version_updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
            "#
        );
        sqlx::query(sqlx::AssertSqlSafe(update_document_query.as_str()))
            .bind(generated_id)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .execute(&mut *transaction)
            .await
            .map_err(version_sqlx_error)?;

        transaction.commit().await.map_err(version_sqlx_error)?;
        Ok(version)
    }

    pub async fn get_latest_version_for_document(
        &self,
        document_id: u64,
    ) -> Result<Option<KnowledgeDocumentVersion>, KnowledgeDocumentVersionStoreError> {
        let tenant_id = version_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = version_to_i64("organization_id", self.organization_id)?;
        let document_id = version_to_i64("document_id", document_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, document_id, version_no, original_object_ref_id, checksum_sha256_hex,
                   size_bytes, mime_type, parse_state, index_state
            FROM kb_document_version
            WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND status = $4
            ORDER BY version_no DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(version_sqlx_error)?;

        row.as_ref().map(document_version_from_row).transpose()
    }

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
}

#[async_trait]
impl KnowledgeDocumentVersionStore for PostgresKnowledgeDocumentVersionStore {
    async fn create_document_version(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        self.insert_document_version(record).await
    }

    async fn create_or_get_document_version(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        if let Some(version) = self
            .insert_document_version_if_absent(record.clone())
            .await?
        {
            return Ok(version);
        }

        let version = self.get_document_version_by_identity(&record).await?;
        self.bind_document_current_version_if_latest(
            version.document_id,
            version.id,
            version.version_no,
            &version_now()?,
        )
        .await?;
        Ok(version)
    }
}

impl PostgresKnowledgeDocumentVersionStore {
    async fn get_document_version_by_identity(
        &self,
        record: &CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        let tenant_id = version_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = version_to_i64("organization_id", self.organization_id)?;
        let document_id = version_to_i64("document_id", record.document_id)?;
        let version_no = version_to_i64("version_no", record.version_no)?;
        let row = sqlx::query(
            r#"
            SELECT id, document_id, version_no, original_object_ref_id, checksum_sha256_hex,
                   size_bytes, mime_type, parse_state, index_state
            FROM kb_document_version
            WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND version_no = $4 AND status = $5
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(version_no)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(version_sqlx_error)?;

        document_version_from_row(&row)
    }

    async fn insert_document_version_if_absent(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<Option<KnowledgeDocumentVersion>, KnowledgeDocumentVersionStoreError> {
        self.insert_document_version_with_conflict_clause(record, "ON CONFLICT DO NOTHING")
            .await
    }

    async fn insert_document_version(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
    ) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
        self.insert_document_version_with_conflict_clause(record, "")
            .await?
            .ok_or_else(|| {
                KnowledgeDocumentVersionStoreError::Internal(
                    "failed to insert document version".to_string(),
                )
            })
    }

    async fn insert_document_version_with_conflict_clause(
        &self,
        record: CreateKnowledgeDocumentVersionRecord,
        conflict_clause: &str,
    ) -> Result<Option<KnowledgeDocumentVersion>, KnowledgeDocumentVersionStoreError> {
        let tenant_id = version_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = version_to_i64("organization_id", self.organization_id)?;
        let document_id = version_to_i64("document_id", record.document_id)?;
        let version_no = version_to_i64("version_no", record.version_no)?;
        let original_object_ref_id =
            version_to_i64("original_object_ref_id", record.original_object_ref_id)?;
        let size_bytes = version_to_i64("size_bytes", record.size_bytes)?;
        let generated_id = next_i64_id(&self.id_generator).map_err(version_id_error)?;
        let now = version_now()?;
        let submitted_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$15");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$16");
        let query = format!(
            r#"
            INSERT INTO kb_document_version (
                id,
                uuid,
                tenant_id,
                organization_id,
                document_id,
                version_no,
                original_object_ref_id,
                checksum_sha256_hex,
                size_bytes,
                mime_type,
                parse_state,
                index_state,
                submitted_at,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {submitted_at_expr}, $14, {created_at_expr}, {updated_at_expr}, $17)
            {conflict_clause}
            RETURNING
                id,
                document_id,
                version_no,
                original_object_ref_id,
                checksum_sha256_hex,
                size_bytes,
                mime_type,
                parse_state,
                index_state
            "#
        );

        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(generated_id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(version_no)
            .bind(original_object_ref_id)
            .bind(record.checksum_sha256_hex.clone())
            .bind(size_bytes)
            .bind(record.mime_type.clone())
            .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
            .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
            .bind(now.clone())
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now.clone())
            .bind(INITIAL_VERSION)
            .fetch_optional(&self.pool)
            .await
            .map_err(version_sqlx_error)?;

        let Some(row) = row else {
            return Ok(None);
        };
        let version = document_version_from_row(&row)?;

        self.bind_document_current_version_if_latest(
            version.document_id,
            version.id,
            version.version_no,
            &now,
        )
        .await?;

        Ok(Some(version))
    }

    async fn bind_document_current_version_if_latest(
        &self,
        document_id: u64,
        version_id: u64,
        version_no: u64,
        now: &str,
    ) -> Result<(), KnowledgeDocumentVersionStoreError> {
        let tenant_id = version_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = version_to_i64("organization_id", self.organization_id)?;
        let document_id = version_to_i64("document_id", document_id)?;
        let version_id = version_to_i64("version_id", version_id)?;
        let version_no = version_to_i64("version_no", version_no)?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_document
            SET current_version_id = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
              AND (
                  current_version_id IS NULL
                  OR NOT EXISTS (
                      SELECT 1
                      FROM kb_document_version current_version
                      WHERE current_version.tenant_id = kb_document.tenant_id
                        AND current_version.organization_id = kb_document.organization_id
                        AND current_version.document_id = kb_document.id
                        AND current_version.id = kb_document.current_version_id
                        AND current_version.status = $7
                        AND current_version.version_no >= $8
                  )
              )
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(version_id)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .bind(ACTIVE_STATUS)
            .bind(version_no)
            .execute(&self.pool)
            .await
            .map_err(version_sqlx_error)?;

        Ok(())
    }

    pub async fn list_versions_page_for_document(
        &self,
        document_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<
        (Vec<KnowledgeDocumentVersion>, Option<String>, bool),
        KnowledgeDocumentVersionStoreError,
    > {
        let tenant_id = version_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = version_to_i64("organization_id", self.organization_id)?;
        let document_id = version_to_i64("document_id", document_id)?;
        let page_size = i64::from(page_size.clamp(1, 200));
        let fetch_limit = page_size + 1;
        let cursor_id = cursor
            .map(|value| version_to_i64("cursor", value))
            .transpose()?;

        let rows = if let Some(after_id) = cursor_id {
            sqlx::query(
                r#"
                SELECT id, document_id, version_no, original_object_ref_id, checksum_sha256_hex, size_bytes, mime_type, parse_state, index_state
                FROM kb_document_version
                WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND status = $4 AND id > $5
                ORDER BY id ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .bind(after_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(version_sqlx_error)?
        } else {
            sqlx::query(
                r#"
                SELECT id, document_id, version_no, original_object_ref_id, checksum_sha256_hex, size_bytes, mime_type, parse_state, index_state
                FROM kb_document_version
                WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND status = $4
                ORDER BY id ASC
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(document_id)
            .bind(ACTIVE_STATUS)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(version_sqlx_error)?
        };

        let has_more = rows.len() > page_size as usize;
        let mut items = Vec::new();
        for row in rows.iter().take(page_size as usize) {
            items.push(document_version_from_row(row)?);
        }
        let next_cursor = if has_more {
            items.last().map(|version| version.id.to_string())
        } else {
            None
        };
        Ok((items, next_cursor, has_more))
    }
}

#[derive(Debug, Clone)]
pub struct PostgresIngestionJobStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    #[allow(dead_code)] // 保留 keyword backend 选择机制（当前仅 PostgresTsVector）
    keyword_backend: KeywordSearchBackend,
    timestamp_dialect: SqlTimestampDialect,
    database_engine: DatabaseEngine,
    quota_limits: Option<KnowledgebaseTenantQuotaLimits>,
}

impl PostgresIngestionJobStore {
    pub fn new(pool: AnyPool, tenant_id: u64, organization_id: u64) -> Self {
        Self::with_keyword_backend(
            pool,
            tenant_id,
            organization_id,
            KeywordSearchBackend::PostgresTsVector,
            default_knowledge_id_generator(),
        )
    }

    pub fn with_id_generator(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        id_generator: Arc<dyn KnowledgeIdGenerator>,
    ) -> Self {
        Self::with_keyword_backend(
            pool,
            tenant_id,
            organization_id,
            KeywordSearchBackend::PostgresTsVector,
            id_generator,
        )
    }

    pub fn with_keyword_backend(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        keyword_backend: KeywordSearchBackend,
        id_generator: Arc<dyn KnowledgeIdGenerator>,
    ) -> Self {
        Self {
            pool,
            tenant_id,
            organization_id,
            id_generator,
            keyword_backend,
            timestamp_dialect: SqlTimestampDialect::default(),
            database_engine: DatabaseEngine::Postgres,
            quota_limits: None,
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self.database_engine = database_engine;
        self
    }

    pub fn with_quota_limits(mut self, quota_limits: KnowledgebaseTenantQuotaLimits) -> Self {
        self.quota_limits = Some(quota_limits);
        self
    }

    pub async fn mark_running_job_succeeded_with_outbox(
        &self,
        job_id: u64,
        outbox: AppendOutboxEventRecord,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        if is_blank(Some(outbox.aggregate_type.as_str())) {
            return Err(IngestionJobStoreError::Conflict(
                "aggregate_type is required".to_string(),
            ));
        }
        if is_blank(Some(outbox.event_type.as_str())) {
            return Err(IngestionJobStoreError::Conflict(
                "event_type is required".to_string(),
            ));
        }
        if is_blank(Some(outbox.payload_json.as_str())) {
            return Err(IngestionJobStoreError::Conflict(
                "payload_json is required".to_string(),
            ));
        }

        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(job_sqlx_error)?;
        let job_id_i64 = job_to_i64("job_id", job_id)?;

        let current_row = sqlx::query(
            r#"
            SELECT id, space_id, job_type, idempotency_key, state, error_detail,
                   CAST(metadata AS TEXT) AS metadata, claim_token
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(job_id_i64)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| job_fetch_error(job_id_i64, error))?
        .ok_or(IngestionJobStoreError::NotFound(job_id))?;

        let current_job = job_from_row(&current_row)?;
        if current_job.state != IngestionJobState::Running {
            return Err(IngestionJobStoreError::Conflict(format!(
                "invalid ingestion job transition: {:?} -> {:?}",
                current_job.state,
                IngestionJobState::Succeeded
            )));
        }
        let claim_token: Option<String> =
            current_row.try_get("claim_token").map_err(job_sqlx_error)?;
        if claim_token.is_some() {
            return Err(IngestionJobStoreError::Conflict(
                "ingestion job is owned by a leased worker".to_string(),
            ));
        }

        let now = job_now()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let update_query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET state = $1,
                error_detail = NULL,
                claim_owner = NULL,
                claim_token = NULL,
                lease_expires_at = NULL,
                finished_at = {updated_at_expr},
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6 AND state = $7
              AND claim_token IS NULL
            RETURNING id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            "#,
        );
        let updated_row = sqlx::query(sqlx::AssertSqlSafe(update_query.as_str()))
            .bind(ingestion_state_code(IngestionJobState::Succeeded))
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(job_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(IngestionJobState::Running))
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| job_fetch_error(job_id_i64, error))?;

        let outbox_id = next_i64_id(&self.id_generator)
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        let aggregate_id = job_to_i64("aggregate_id", outbox.aggregate_id)?;

        let payload_expr = self.timestamp_dialect.sql_json_expr("$8");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$10");
        let outbox_query = format!(
            r#"
            INSERT INTO kb_outbox_event (
                id, uuid, tenant_id, organization_id, aggregate_type, aggregate_id, event_type,
                payload, status, created_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, {payload_expr}, $9, {created_at_expr}, $11)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(outbox_query.as_str()))
            .bind(outbox_id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(outbox.aggregate_type)
            .bind(aggregate_id)
            .bind(outbox.event_type)
            .bind(outbox.payload_json)
            .bind(OUTBOX_STATUS_PENDING)
            .bind(&now)
            .bind(INITIAL_VERSION)
            .execute(&mut *transaction)
            .await
            .map_err(job_sqlx_error)?;

        transaction.commit().await.map_err(job_sqlx_error)?;
        job_from_row(&updated_row)
    }

    pub async fn complete_running_ingestion_with_chunks_and_outbox(
        &self,
        record: CompleteRunningIngestionRecord,
    ) -> Result<CompletedIngestionResult, IngestionJobStoreError> {
        if is_blank(Some(record.outbox.aggregate_type.as_str())) {
            return Err(IngestionJobStoreError::Conflict(
                "aggregate_type is required".to_string(),
            ));
        }
        if is_blank(Some(record.outbox.event_type.as_str())) {
            return Err(IngestionJobStoreError::Conflict(
                "event_type is required".to_string(),
            ));
        }
        if is_blank(Some(record.outbox.payload_json.as_str())) {
            return Err(IngestionJobStoreError::Conflict(
                "payload_json is required".to_string(),
            ));
        }

        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(job_sqlx_error)?;
        let job_id_i64 = job_to_i64("job_id", record.job_id)?;

        let current_row = sqlx::query(
            r#"
            SELECT id, space_id, job_type, idempotency_key, state, error_detail,
                   CAST(metadata AS TEXT) AS metadata, claim_token
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(job_id_i64)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| job_fetch_error(job_id_i64, error))?
        .ok_or(IngestionJobStoreError::NotFound(record.job_id))?;

        let current_job = job_from_row(&current_row)?;
        if current_job.state != IngestionJobState::Running {
            return Err(IngestionJobStoreError::Conflict(format!(
                "invalid ingestion job transition: {:?} -> {:?}",
                current_job.state,
                IngestionJobState::Succeeded
            )));
        }
        let current_claim_token: Option<String> =
            current_row.try_get("claim_token").map_err(job_sqlx_error)?;
        if current_claim_token.as_deref() != record.claim_token.as_deref() {
            return Err(IngestionJobStoreError::Conflict(
                "ingestion job lease is no longer owned by this worker".to_string(),
            ));
        }

        let chunk_count = replace_version_chunks_in_transaction(
            &mut transaction,
            ReplaceVersionChunksContext {
                tenant_id: self.tenant_id,
                organization_id: self.organization_id,
                id_generator: &self.id_generator,
                timestamp_dialect: self.timestamp_dialect,
                document_version_id: record.document_version_id,
            },
            &record.chunks,
        )
        .await
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;

        let now = job_now()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let update_query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET state = $1,
                error_detail = NULL,
                claim_owner = NULL,
                claim_token = NULL,
                lease_expires_at = NULL,
                finished_at = {updated_at_expr},
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6 AND state = $7
              AND (
                  ($8 IS NULL AND claim_token IS NULL)
                  OR (claim_token = $8 AND lease_expires_at > {updated_at_expr})
              )
            RETURNING id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            "#,
        );
        let updated_row = sqlx::query(sqlx::AssertSqlSafe(update_query.as_str()))
            .bind(ingestion_state_code(IngestionJobState::Succeeded))
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(job_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind(record.claim_token.as_deref())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(job_sqlx_error)?
            .ok_or_else(|| {
                IngestionJobStoreError::Conflict(
                    "ingestion job lease was lost before completion".to_string(),
                )
            })?;

        let outbox_id = next_i64_id(&self.id_generator)
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        let aggregate_id = job_to_i64("aggregate_id", record.outbox.aggregate_id)?;

        let payload_expr = self.timestamp_dialect.sql_json_expr("$8");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$10");
        let outbox_query = format!(
            r#"
            INSERT INTO kb_outbox_event (
                id, uuid, tenant_id, organization_id, aggregate_type, aggregate_id, event_type,
                payload, status, created_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, {payload_expr}, $9, {created_at_expr}, $11)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(outbox_query.as_str()))
            .bind(outbox_id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(record.outbox.aggregate_type)
            .bind(aggregate_id)
            .bind(record.outbox.event_type)
            .bind(record.outbox.payload_json)
            .bind(OUTBOX_STATUS_PENDING)
            .bind(&now)
            .bind(INITIAL_VERSION)
            .execute(&mut *transaction)
            .await
            .map_err(job_sqlx_error)?;

        transaction.commit().await.map_err(job_sqlx_error)?;
        Ok(CompletedIngestionResult {
            job: job_from_row(&updated_row)?,
            chunk_count,
        })
    }
}

#[async_trait]
impl IngestionJobStore for PostgresIngestionJobStore {
    async fn create_or_get_job(
        &self,
        record: CreateIngestionJobRecord,
    ) -> Result<CreateOrGetIngestionJobResult, IngestionJobStoreError> {
        if let Some(limits) = self.quota_limits {
            return self.create_or_get_job_with_quota(record, limits).await;
        }
        if let Some(job) = self.insert_job_if_absent(record.clone()).await? {
            return Ok(CreateOrGetIngestionJobResult { job, created: true });
        }

        let row = self.get_job_by_idempotency(&record).await?;
        validate_existing_job_idempotency(&row, &record)?;
        Ok(CreateOrGetIngestionJobResult {
            job: job_from_row(&row)?,
            created: false,
        })
    }

    async fn get_job(&self, job_id: u64) -> Result<IngestionJob, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id = job_to_i64("job_id", job_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(job_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| job_fetch_error(job_id, error))?;

        job_from_row(&row)
    }

    async fn get_job_lifecycle(
        &self,
        job_id: u64,
    ) -> Result<IngestionJobLifecycle, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id = job_to_i64("job_id", job_id)?;
        let row = sqlx::query(
            r#"
            SELECT id,
                   space_id,
                   job_type,
                   idempotency_key,
                   state,
                   error_detail,
                   CAST(metadata AS TEXT) AS metadata,
                   CAST(created_at AS TEXT) AS created_at,
                   CAST(updated_at AS TEXT) AS updated_at
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(job_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| job_fetch_error(job_id, error))?;

        job_lifecycle_from_row(&row)
    }

    async fn update_job_state(
        &self,
        job_id: u64,
        expected_state: IngestionJobState,
        state: IngestionJobState,
        error_message: Option<String>,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id = job_to_i64("job_id", job_id)?;
        let now = job_now()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$3");
        let query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET state = $1,
                error_detail = $2,
                claim_owner = NULL,
                claim_token = NULL,
                lease_expires_at = NULL,
                finished_at = CASE WHEN $1 IN (2, 3, 4) THEN {updated_at_expr} ELSE finished_at END,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $4 AND organization_id = $5 AND id = $6 AND status = $7 AND state = $8
              AND claim_token IS NULL
            RETURNING id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(ingestion_state_code(state))
            .bind(error_message)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(job_id)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(expected_state))
            .fetch_one(&self.pool)
            .await
            .map_err(|error| job_fetch_error(job_id, error))?;

        job_from_row(&row)
    }

    async fn attach_drive_import_linkage(
        &self,
        job_id: u64,
        linkage: DriveImportJobLinkage,
    ) -> Result<(), IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id = job_to_i64("job_id", job_id)?;
        let row = sqlx::query(
            r#"
            SELECT CAST(metadata AS TEXT) AS metadata
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(job_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(job_sqlx_error)?;
        let Some(row) = row else {
            return Err(IngestionJobStoreError::NotFound(job_id as u64));
        };
        let existing: Option<String> = row.try_get("metadata").map_err(job_sqlx_error)?;
        let metadata = merge_drive_import_linkage_metadata(existing.as_deref(), &linkage)?;
        let now = job_now()?;
        let metadata_expr = self.timestamp_dialect.sql_json_expr("$1");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET metadata = {metadata_expr}, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(metadata)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(job_id)
            .bind(ACTIVE_STATUS)
            .execute(&self.pool)
            .await
            .map_err(job_sqlx_error)?;

        if updated.rows_affected() == 0 {
            return Err(IngestionJobStoreError::NotFound(job_id as u64));
        }
        Ok(())
    }

    async fn get_drive_import_linkage(
        &self,
        job_id: u64,
    ) -> Result<Option<DriveImportJobLinkage>, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id = job_to_i64("job_id", job_id)?;
        let row = sqlx::query(
            r#"
            SELECT CAST(metadata AS TEXT) AS metadata
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(job_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(job_sqlx_error)?;

        let Some(row) = row else {
            return Err(IngestionJobStoreError::NotFound(job_id as u64));
        };
        let metadata: Option<String> = row.try_get("metadata").map_err(job_sqlx_error)?;
        parse_drive_import_linkage(metadata.as_deref())
    }

    async fn list_jobs_by_state(
        &self,
        state: IngestionJobState,
        limit: u32,
    ) -> Result<Vec<IngestionJob>, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 200));
        let rows = sqlx::query(
            r#"
            SELECT id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND state = $3 AND status = $4
            ORDER BY id ASC
            LIMIT $5
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ingestion_state_code(state))
        .bind(ACTIVE_STATUS)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;

        rows.into_iter().map(|row| job_from_row(&row)).collect()
    }

    async fn claim_ingestion_jobs(
        &self,
        request: ClaimIngestionJobsRequest,
    ) -> Result<Vec<ClaimedIngestionJob>, IngestionJobStoreError> {
        validate_claim_request(&request)?;
        if request.limit == 0 {
            return Ok(Vec::new());
        }

        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let now = OffsetDateTime::now_utc();
        let lease_expires_at = now.checked_add(request.lease_duration).ok_or_else(|| {
            IngestionJobStoreError::Conflict("ingestion job lease expiry overflow".to_string())
        })?;
        let now_text = format_job_timestamp(now)?;
        let lease_expires_at_text = format_job_timestamp(lease_expires_at)?;
        let limit = i64::from(request.limit.min(200));
        let max_attempts = i64::from(request.max_attempts.max(1));
        let exhausted_now_expr = self.timestamp_dialect.sql_timestamp_expr("$8");

        // Jobs whose lease expired with a claimed attempt at/over the maximum
        // are failed permanently instead of being reclaimed forever (crash or
        // lease-loss loops must terminate).
        let fail_exhausted_query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET state = $1,
                claim_owner = NULL,
                claim_token = NULL,
                lease_expires_at = NULL,
                error_detail = $2,
                finished_at = {exhausted_now_expr},
                updated_at = {exhausted_now_expr},
                version = version + 1
            WHERE tenant_id = $3
              AND organization_id = $4
              AND status = $5
              AND job_type = $6
              AND state = $7
              AND lease_expires_at IS NOT NULL
              AND lease_expires_at <= {exhausted_now_expr}
              AND attempt_count >= $9
            "#,
        );
        let exhausted = sqlx::query(sqlx::AssertSqlSafe(fail_exhausted_query.as_str()))
            .bind(ingestion_state_code(IngestionJobState::Failed))
            .bind(format!(
                "ingestion job exceeded the maximum of {max_attempts} claim attempts"
            ))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACTIVE_STATUS)
            .bind(KnowledgeSourceType::DriveObject.as_str())
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind(&now_text)
            .bind(max_attempts)
            .execute(&self.pool)
            .await
            .map_err(job_sqlx_error)?;
        if exhausted.rows_affected() > 0 {
            tracing::error!(
                target: "sdkwork.knowledgebase.ingestion",
                failed_jobs = exhausted.rows_affected(),
                max_attempts,
                "ingestion jobs exhausted their claim attempts and were failed permanently"
            );
        }

        let now_expr = self.timestamp_dialect.sql_timestamp_expr("$7");
        let candidates_query = format!(
            r#"
            SELECT id
            FROM kb_ingestion_job
            WHERE tenant_id = $1
              AND organization_id = $2
              AND status = $3
              AND job_type = $4
              AND attempt_count < $9
              AND (
                  state = $5
                  OR (state = $6 AND lease_expires_at IS NOT NULL AND lease_expires_at <= {now_expr})
              )
            ORDER BY priority DESC, id ASC
            LIMIT $8
            "#,
        );
        let candidate_rows = sqlx::query(sqlx::AssertSqlSafe(candidates_query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACTIVE_STATUS)
            .bind(KnowledgeSourceType::DriveObject.as_str())
            .bind(ingestion_state_code(IngestionJobState::Queued))
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind(&now_text)
            .bind(limit)
            .bind(max_attempts)
            .fetch_all(&self.pool)
            .await
            .map_err(job_sqlx_error)?;

        let lease_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let update_now_expr = self.timestamp_dialect.sql_timestamp_expr("$5");
        let update_query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET state = $1,
                claim_owner = $2,
                claim_token = $3,
                lease_expires_at = {lease_expr},
                attempt_count = attempt_count + 1,
                error_detail = NULL,
                started_at = COALESCE(started_at, {update_now_expr}),
                finished_at = NULL,
                updated_at = {update_now_expr},
                version = version + 1
            WHERE tenant_id = $6
              AND organization_id = $7
              AND id = $8
              AND status = $9
              AND job_type = $10
              AND attempt_count < $13
              AND (
                  state = $11
                  OR (
                      state = $12
                      AND lease_expires_at IS NOT NULL
                      AND lease_expires_at <= {update_now_expr}
                  )
              )
            RETURNING id, space_id, job_type, idempotency_key, state, error_detail,
                      CAST(metadata AS TEXT) AS metadata, attempt_count
            "#,
        );

        let mut claimed = Vec::with_capacity(candidate_rows.len());
        for candidate in candidate_rows {
            let job_id: i64 = candidate.try_get("id").map_err(job_sqlx_error)?;
            let claim_token = Uuid::new_v4().to_string();
            let row = sqlx::query(sqlx::AssertSqlSafe(update_query.as_str()))
                .bind(ingestion_state_code(IngestionJobState::Running))
                .bind(&request.claim_owner)
                .bind(&claim_token)
                .bind(&lease_expires_at_text)
                .bind(&now_text)
                .bind(tenant_id)
                .bind(organization_id)
                .bind(job_id)
                .bind(ACTIVE_STATUS)
                .bind(KnowledgeSourceType::DriveObject.as_str())
                .bind(ingestion_state_code(IngestionJobState::Queued))
                .bind(ingestion_state_code(IngestionJobState::Running))
                .bind(max_attempts)
                .fetch_optional(&self.pool)
                .await
                .map_err(job_sqlx_error)?;
            let Some(row) = row else {
                continue;
            };
            let attempt_count: i64 = row.try_get("attempt_count").map_err(job_sqlx_error)?;
            claimed.push(ClaimedIngestionJob {
                job: job_from_row(&row)?,
                claim_token,
                lease_expires_at,
                attempt_count: u32::try_from(attempt_count).map_err(|error| {
                    IngestionJobStoreError::Internal(format!(
                        "invalid ingestion job attempt_count {attempt_count}: {error}"
                    ))
                })?,
            });
        }
        Ok(claimed)
    }

    async fn renew_ingestion_job_lease(
        &self,
        job_id: u64,
        claim_token: &str,
        lease_duration: Duration,
    ) -> Result<OffsetDateTime, IngestionJobStoreError> {
        validate_claim_token(claim_token)?;
        validate_lease_duration(lease_duration)?;
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id_i64 = job_to_i64("job_id", job_id)?;
        let now = OffsetDateTime::now_utc();
        let lease_expires_at = now.checked_add(lease_duration).ok_or_else(|| {
            IngestionJobStoreError::Conflict("ingestion job lease expiry overflow".to_string())
        })?;
        let now_text = format_job_timestamp(now)?;
        let lease_expires_at_text = format_job_timestamp(lease_expires_at)?;
        let expiry_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let now_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET lease_expires_at = {expiry_expr},
                updated_at = {now_expr},
                version = version + 1
            WHERE tenant_id = $3
              AND organization_id = $4
              AND id = $5
              AND status = $6
              AND state = $7
              AND claim_token = $8
              AND lease_expires_at > {now_expr}
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(&lease_expires_at_text)
            .bind(&now_text)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(job_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind(claim_token)
            .execute(&self.pool)
            .await
            .map_err(job_sqlx_error)?;
        if result.rows_affected() != 1 {
            return Err(IngestionJobStoreError::Conflict(
                "ingestion job lease is expired or no longer owned by this worker".to_string(),
            ));
        }
        Ok(lease_expires_at)
    }

    async fn fail_claimed_ingestion_job(
        &self,
        job_id: u64,
        claim_token: &str,
        error_message: String,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        validate_claim_token(claim_token)?;
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let job_id_i64 = job_to_i64("job_id", job_id)?;
        let now = job_now()?;
        let now_expr = self.timestamp_dialect.sql_timestamp_expr("$3");
        let query = format!(
            r#"
            UPDATE kb_ingestion_job
            SET state = $1,
                error_detail = $2,
                claim_owner = NULL,
                claim_token = NULL,
                lease_expires_at = NULL,
                finished_at = {now_expr},
                updated_at = {now_expr},
                version = version + 1
            WHERE tenant_id = $4
              AND organization_id = $5
              AND id = $6
              AND status = $7
              AND state = $8
              AND claim_token = $9
              AND lease_expires_at > {now_expr}
            RETURNING id, space_id, job_type, idempotency_key, state, error_detail,
                      CAST(metadata AS TEXT) AS metadata
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(ingestion_state_code(IngestionJobState::Failed))
            .bind(truncate(&error_message, 4000, Some("")))
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(job_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind(claim_token)
            .fetch_optional(&self.pool)
            .await
            .map_err(job_sqlx_error)?
            .ok_or_else(|| {
                IngestionJobStoreError::Conflict(
                    "ingestion job lease is expired or no longer owned by this worker".to_string(),
                )
            })?;
        job_from_row(&row)
    }

    async fn fail_expired_upload_sessions(
        &self,
        expired_before: OffsetDateTime,
        limit: u32,
    ) -> Result<u32, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let expired_before = expired_before
            .format(&Rfc3339)
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        let now = job_now()?;
        let limit = i64::from(limit.clamp(1, 200));
        let expired_before_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let now_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            WITH stale_upload_sessions AS (
                SELECT id
                FROM kb_ingestion_job
                WHERE tenant_id = $3
                  AND organization_id = $4
                  AND state IN ($5, $6)
                  AND status = $7
                  AND job_type = $8
                  AND created_at <= {expired_before_expr}
                ORDER BY id ASC
                LIMIT $9
            )
            UPDATE kb_ingestion_job
            SET state = $10,
                error_detail = $11,
                finished_at = {now_expr},
                updated_at = {now_expr},
                version = version + 1
            WHERE tenant_id = $3
              AND organization_id = $4
              AND status = $7
              AND state IN ($5, $6)
              AND id IN (SELECT id FROM stale_upload_sessions)
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(expired_before)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ingestion_state_code(IngestionJobState::Queued))
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind(ACTIVE_STATUS)
            .bind("upload_session")
            .bind(limit)
            .bind(ingestion_state_code(IngestionJobState::Failed))
            .bind("upload session expired before completion")
            .execute(&self.pool)
            .await
            .map_err(job_sqlx_error)?;

        u32::try_from(result.rows_affected())
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))
    }

    async fn mark_running_job_succeeded_with_outbox(
        &self,
        job_id: u64,
        outbox: AppendOutboxEventRecord,
    ) -> Result<IngestionJob, IngestionJobStoreError> {
        PostgresIngestionJobStore::mark_running_job_succeeded_with_outbox(self, job_id, outbox)
            .await
    }

    async fn complete_running_ingestion_with_chunks_and_outbox(
        &self,
        record: CompleteRunningIngestionRecord,
    ) -> Result<CompletedIngestionResult, IngestionJobStoreError> {
        PostgresIngestionJobStore::complete_running_ingestion_with_chunks_and_outbox(self, record)
            .await
    }
}

impl PostgresIngestionJobStore {
    async fn create_or_get_job_with_quota(
        &self,
        record: CreateIngestionJobRecord,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<CreateOrGetIngestionJobResult, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(job_sqlx_error)?;

        if let Some(row) = self
            .find_job_by_idempotency_on(&record, &mut transaction)
            .await?
        {
            validate_existing_job_idempotency(&row, &record)?;
            let job = job_from_row(&row)?;
            transaction.commit().await.map_err(job_sqlx_error)?;
            return Ok(CreateOrGetIngestionJobResult {
                job,
                created: false,
            });
        }

        self.ensure_ingest_quota_on(&mut transaction, limits)
            .await?;
        let job = self
            .insert_job_if_absent_on(record.clone(), &mut transaction)
            .await?;
        let (job, created) = match job {
            Some(job) => (job, true),
            None => {
                let row = self
                    .find_job_by_idempotency_on(&record, &mut transaction)
                    .await?
                    .ok_or_else(|| {
                        IngestionJobStoreError::Internal(
                            "ingestion idempotency conflict did not resolve to an active job"
                                .to_string(),
                        )
                    })?;
                validate_existing_job_idempotency(&row, &record)?;
                (job_from_row(&row)?, false)
            }
        };
        transaction.commit().await.map_err(job_sqlx_error)?;
        Ok(CreateOrGetIngestionJobResult { job, created })
    }

    async fn ensure_ingest_quota_on(
        &self,
        connection: &mut AnyConnection,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<(), IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let expired_before = OffsetDateTime::now_utc()
            .checked_sub(KNOWLEDGE_UPLOAD_SESSION_TTL)
            .ok_or_else(|| {
                IngestionJobStoreError::Internal(
                    "upload session quota cutoff is outside the supported timestamp range"
                        .to_string(),
                )
            })?
            .format(&Rfc3339)
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        let expired_before_expr = self.timestamp_dialect.sql_timestamp_expr("$7");
        let query = format!(
            r#"
            SELECT COUNT(*)
            FROM kb_ingestion_job
            WHERE tenant_id = $1
              AND organization_id = $2
              AND status = $3
              AND state IN ($4, $5)
              AND NOT (
                  job_type = $6
                  AND created_at <= {expired_before_expr}
              )
            "#,
        );
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(IngestionJobState::Queued))
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind("upload_session")
            .bind(expired_before)
            .fetch_one(connection)
            .await
            .map_err(job_sqlx_error)?;
        let count = u32::try_from(count.max(0))
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        ensure_ingest_concurrency(count, &limits)?;
        Ok(())
    }

    pub async fn count_inflight_jobs(&self) -> Result<u32, IngestionJobStoreError> {
        let expired_before = OffsetDateTime::now_utc()
            .checked_sub(KNOWLEDGE_UPLOAD_SESSION_TTL)
            .ok_or_else(|| {
                IngestionJobStoreError::Internal(
                    "upload session recovery cutoff is outside the supported timestamp range"
                        .to_string(),
                )
            })?;
        self.fail_expired_upload_sessions(expired_before, STALE_UPLOAD_SESSION_RECOVERY_BATCH_SIZE)
            .await?;

        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let expired_before = expired_before
            .format(&Rfc3339)
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        let expired_before_expr = self.timestamp_dialect.sql_timestamp_expr("$7");
        let query = format!(
            r#"
            SELECT COUNT(*)
            FROM kb_ingestion_job
            WHERE tenant_id = $1
              AND organization_id = $2
              AND status = $3
              AND state IN ($4, $5)
              AND NOT (
                  job_type = $6
                  AND created_at <= {expired_before_expr}
              )
            "#,
        );
        let row: (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACTIVE_STATUS)
            .bind(ingestion_state_code(IngestionJobState::Queued))
            .bind(ingestion_state_code(IngestionJobState::Running))
            .bind("upload_session")
            .bind(expired_before)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
        u32::try_from(row.0.max(0))
            .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))
    }

    async fn get_job_by_idempotency(
        &self,
        record: &CreateIngestionJobRecord,
    ) -> Result<AnyRow, IngestionJobStoreError> {
        let mut connection = self.pool.acquire().await.map_err(job_sqlx_error)?;
        self.find_job_by_idempotency_on(record, &mut connection)
            .await?
            .ok_or_else(|| {
                IngestionJobStoreError::Internal(
                    "ingestion idempotency key did not resolve to an active job".to_string(),
                )
            })
    }

    async fn find_job_by_idempotency_on(
        &self,
        record: &CreateIngestionJobRecord,
        connection: &mut AnyConnection,
    ) -> Result<Option<AnyRow>, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let space_id = job_to_i64("space_id", record.space_id)?;
        sqlx::query(
            r#"
            SELECT id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            FROM kb_ingestion_job
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND idempotency_key = $4 AND status = $5
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(&record.idempotency_key)
        .bind(ACTIVE_STATUS)
        .fetch_optional(connection)
        .await
        .map_err(job_sqlx_error)
    }

    async fn insert_job_if_absent(
        &self,
        record: CreateIngestionJobRecord,
    ) -> Result<Option<IngestionJob>, IngestionJobStoreError> {
        let mut connection = self.pool.acquire().await.map_err(job_sqlx_error)?;
        self.insert_job_if_absent_on(record, &mut connection).await
    }

    async fn insert_job_if_absent_on(
        &self,
        record: CreateIngestionJobRecord,
        connection: &mut AnyConnection,
    ) -> Result<Option<IngestionJob>, IngestionJobStoreError> {
        let tenant_id = job_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = job_to_i64("organization_id", self.organization_id)?;
        let space_id = job_to_i64("space_id", record.space_id)?;
        let id = next_i64_id(&self.id_generator).map_err(job_id_error)?;
        let now = job_now()?;
        let metadata = job_metadata_to_json(record.idempotency_fingerprint_sha256_hex.as_deref())?;
        let metadata_expr = self.timestamp_dialect.sql_json_expr("$9");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$11");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$12");
        let query = format!(
            r#"
            INSERT INTO kb_ingestion_job (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                job_type,
                state,
                priority,
                progress,
                idempotency_key,
                metadata,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 0, $8, {metadata_expr}, $10, {created_at_expr}, {updated_at_expr}, $13)
            ON CONFLICT(tenant_id, organization_id, space_id, idempotency_key) DO NOTHING
            RETURNING id, space_id, job_type, idempotency_key, state, error_detail, CAST(metadata AS TEXT) AS metadata
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(record.source_type)
            .bind(ingestion_state_code(IngestionJobState::Queued))
            .bind(record.idempotency_key)
            .bind(metadata)
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_optional(connection)
            .await
            .map_err(job_sqlx_error)?;

        row.map(|row| job_from_row(&row)).transpose()
    }
}

fn job_from_row(row: &AnyRow) -> Result<IngestionJob, IngestionJobStoreError> {
    let state_code: i64 = row.try_get("state").map_err(job_sqlx_error)?;
    Ok(IngestionJob {
        id: job_from_i64("id", row.try_get("id").map_err(job_sqlx_error)?)?,
        space_id: job_from_i64("space_id", row.try_get("space_id").map_err(job_sqlx_error)?)?,
        source_type: row.try_get("job_type").map_err(job_sqlx_error)?,
        idempotency_key: row.try_get("idempotency_key").map_err(job_sqlx_error)?,
        state: ingestion_state_from_code(state_code)?,
        error_message: row.try_get("error_detail").map_err(job_sqlx_error)?,
    })
}

fn job_lifecycle_from_row(row: &AnyRow) -> Result<IngestionJobLifecycle, IngestionJobStoreError> {
    let created_at: String = row.try_get("created_at").map_err(job_sqlx_error)?;
    let updated_at: String = row.try_get("updated_at").map_err(job_sqlx_error)?;
    Ok(IngestionJobLifecycle {
        job: job_from_row(row)?,
        created_at: parse_job_timestamp("created_at", &created_at)?,
        updated_at: parse_job_timestamp("updated_at", &updated_at)?,
    })
}

fn parse_job_timestamp(field: &str, value: &str) -> Result<OffsetDateTime, IngestionJobStoreError> {
    let value = value.trim();
    if let Ok(timestamp) = OffsetDateTime::parse(value, &Rfc3339) {
        return Ok(timestamp);
    }

    let normalized = format!("{}Z", value.replacen(' ', "T", 1));
    OffsetDateTime::parse(&normalized, &Rfc3339).map_err(|error| {
        IngestionJobStoreError::Internal(format!(
            "invalid ingestion job {field} timestamp {value:?}: {error}"
        ))
    })
}

fn validate_existing_job_idempotency(
    row: &AnyRow,
    record: &CreateIngestionJobRecord,
) -> Result<(), IngestionJobStoreError> {
    let existing_job_type: String = row.try_get("job_type").map_err(job_sqlx_error)?;
    if existing_job_type != record.source_type {
        return Err(IngestionJobStoreError::Conflict(
            "idempotency_key is already used for a different job_type".to_string(),
        ));
    }

    if let Some(expected_fingerprint) = &record.idempotency_fingerprint_sha256_hex {
        let existing_fingerprint = job_metadata_fingerprint(row)?;
        if existing_fingerprint.as_deref() != Some(expected_fingerprint.as_str()) {
            return Err(IngestionJobStoreError::Conflict(
                "idempotency_key is already used for a different request".to_string(),
            ));
        }
    }

    Ok(())
}

fn job_metadata_to_json(
    idempotency_fingerprint_sha256_hex: Option<&str>,
) -> Result<Option<String>, IngestionJobStoreError> {
    let Some(fingerprint) = idempotency_fingerprint_sha256_hex else {
        return Ok(None);
    };
    if fingerprint.len() != 64 || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(IngestionJobStoreError::Internal(
            "idempotency_fingerprint_sha256_hex must be 64 hex characters".to_string(),
        ));
    }

    Ok(Some(
        serde_json::json!({
            "idempotency_fingerprint_sha256_hex": fingerprint,
        })
        .to_string(),
    ))
}

fn job_metadata_fingerprint(row: &AnyRow) -> Result<Option<String>, IngestionJobStoreError> {
    let metadata: Option<String> = row.try_get("metadata").map_err(job_sqlx_error)?;
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    let metadata: serde_json::Value = serde_json::from_str(&metadata)
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
    Ok(metadata
        .get("idempotency_fingerprint_sha256_hex")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string))
}

fn merge_drive_import_linkage_metadata(
    existing: Option<&str>,
    linkage: &DriveImportJobLinkage,
) -> Result<String, IngestionJobStoreError> {
    let mut metadata = existing
        .map(serde_json::from_str::<serde_json::Value>)
        .transpose()
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?
        .unwrap_or_else(|| serde_json::json!({}));
    let object = encode_drive_object_ref_snapshot(&linkage.original_object_ref)
        .map_err(IngestionJobStoreError::Internal)?;
    metadata["drive_import"] = serde_json::json!({
        "source_id": linkage.source_id,
        "document_id": linkage.document_id,
        "document_version_id": linkage.document_version_id,
        "original_object_ref": object,
    });
    serde_json::to_string(&metadata)
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))
}

fn parse_drive_import_linkage(
    metadata: Option<&str>,
) -> Result<Option<DriveImportJobLinkage>, IngestionJobStoreError> {
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    let metadata: serde_json::Value = serde_json::from_str(metadata)
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))?;
    let Some(linkage) = metadata.get("drive_import") else {
        return Ok(None);
    };
    let read_u64 = |field: &str| -> Result<u64, IngestionJobStoreError> {
        linkage
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                IngestionJobStoreError::Internal(format!("drive_import metadata missing {field}"))
            })
    };
    let original_object_ref = linkage
        .get("original_object_ref")
        .ok_or_else(|| {
            IngestionJobStoreError::Internal(
                "drive_import metadata missing original_object_ref".to_string(),
            )
        })
        .and_then(|value| {
            decode_drive_object_ref_snapshot(value).map_err(IngestionJobStoreError::Internal)
        })?;
    Ok(Some(DriveImportJobLinkage {
        source_id: read_u64("source_id")?,
        document_id: read_u64("document_id")?,
        document_version_id: read_u64("document_version_id")?,
        original_object_ref,
    }))
}

fn source_from_row(row: &AnyRow) -> Result<KnowledgeSource, KnowledgeSourceStoreError> {
    Ok(KnowledgeSource {
        id: source_from_i64("id", row.try_get("id").map_err(source_sqlx_error)?)?,
        space_id: source_from_i64(
            "space_id",
            row.try_get("space_id").map_err(source_sqlx_error)?,
        )?,
        source_type: source_type_from_value(
            &row.try_get::<String, _>("source_type")
                .map_err(source_sqlx_error)?,
        )?,
        provider: row.try_get("provider").map_err(source_sqlx_error)?,
        drive_bucket: row.try_get("drive_bucket").map_err(source_sqlx_error)?,
        drive_prefix: row.try_get("drive_prefix").map_err(source_sqlx_error)?,
        connector_metadata_json: row.try_get("metadata").map_err(source_sqlx_error)?,
    })
}

fn document_from_row(row: &AnyRow) -> Result<KnowledgeDocument, KnowledgeDocumentStoreError> {
    let visibility_code: i64 = row.try_get("visibility").map_err(document_sqlx_error)?;
    let content_state_code: i64 = row.try_get("content_state").map_err(document_sqlx_error)?;
    let index_state_code: i64 = row.try_get("index_state").map_err(document_sqlx_error)?;
    Ok(KnowledgeDocument {
        id: document_from_i64("id", row.try_get("id").map_err(document_sqlx_error)?)?,
        space_id: document_from_i64(
            "space_id",
            row.try_get("space_id").map_err(document_sqlx_error)?,
        )?,
        collection_id: document_from_i64(
            "collection_id",
            row.try_get("collection_id").map_err(document_sqlx_error)?,
        )?,
        source_id: row
            .try_get::<Option<i64>, _>("source_id")
            .map_err(document_sqlx_error)?
            .map(|value| document_from_i64("source_id", value))
            .transpose()?,
        original_file_drive_node_id: row
            .try_get("original_file_drive_node_id")
            .map_err(document_sqlx_error)?,
        title: row.try_get("title").map_err(document_sqlx_error)?,
        mime_type: row.try_get("mime_type").map_err(document_sqlx_error)?,
        language: row.try_get("language").map_err(document_sqlx_error)?,
        current_version_id: row
            .try_get::<Option<i64>, _>("current_version_id")
            .map_err(document_sqlx_error)?
            .map(|value| document_from_i64("current_version_id", value))
            .transpose()?,
        visibility: document_visibility_from_code(visibility_code)?,
        content_state: document_state_from_code(content_state_code)?,
        index_state: version_state_from_code_for_document(index_state_code)?,
    })
}

fn document_version_from_row(
    row: &AnyRow,
) -> Result<KnowledgeDocumentVersion, KnowledgeDocumentVersionStoreError> {
    let parse_state_code: i64 = row.try_get("parse_state").map_err(version_sqlx_error)?;
    let index_state_code: i64 = row.try_get("index_state").map_err(version_sqlx_error)?;
    Ok(KnowledgeDocumentVersion {
        id: version_from_i64("id", row.try_get("id").map_err(version_sqlx_error)?)?,
        document_id: version_from_i64(
            "document_id",
            row.try_get("document_id").map_err(version_sqlx_error)?,
        )?,
        version_no: version_from_i64(
            "version_no",
            row.try_get("version_no").map_err(version_sqlx_error)?,
        )?,
        original_object_ref_id: version_from_i64(
            "original_object_ref_id",
            row.try_get("original_object_ref_id")
                .map_err(version_sqlx_error)?,
        )?,
        checksum_sha256_hex: row
            .try_get("checksum_sha256_hex")
            .map_err(version_sqlx_error)?,
        size_bytes: version_from_i64(
            "size_bytes",
            row.try_get("size_bytes").map_err(version_sqlx_error)?,
        )?,
        mime_type: row.try_get("mime_type").map_err(version_sqlx_error)?,
        parse_state: version_state_from_code(parse_state_code)?,
        index_state: version_state_from_code(index_state_code)?,
    })
}

fn document_visibility_code(value: KnowledgeDocumentVisibility) -> i64 {
    match value {
        KnowledgeDocumentVisibility::Private => 0,
        KnowledgeDocumentVisibility::Space => 1,
        KnowledgeDocumentVisibility::Organization => 2,
        KnowledgeDocumentVisibility::Public => 3,
    }
}

fn document_state_code(value: KnowledgeDocumentState) -> i64 {
    match value {
        KnowledgeDocumentState::Draft => 0,
        KnowledgeDocumentState::Ready => 1,
        KnowledgeDocumentState::Archived => 2,
        KnowledgeDocumentState::Deleted => 3,
    }
}

fn version_state_code(value: KnowledgeDocumentVersionState) -> i64 {
    match value {
        KnowledgeDocumentVersionState::Pending => 0,
        KnowledgeDocumentVersionState::Running => 1,
        KnowledgeDocumentVersionState::Succeeded => 2,
        KnowledgeDocumentVersionState::Failed => 3,
    }
}

fn source_type_from_value(value: &str) -> Result<KnowledgeSourceType, KnowledgeSourceStoreError> {
    match value {
        "upload" => Ok(KnowledgeSourceType::Upload),
        "drive_object" => Ok(KnowledgeSourceType::DriveObject),
        "drive_folder" => Ok(KnowledgeSourceType::DriveFolder),
        "url" => Ok(KnowledgeSourceType::Url),
        "connector" => Ok(KnowledgeSourceType::Connector),
        "api" => Ok(KnowledgeSourceType::Api),
        _ => Err(KnowledgeSourceStoreError::Internal(format!(
            "unknown knowledge source type: {value}"
        ))),
    }
}

fn document_visibility_from_code(
    code: i64,
) -> Result<KnowledgeDocumentVisibility, KnowledgeDocumentStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentVisibility::Private),
        1 => Ok(KnowledgeDocumentVisibility::Space),
        2 => Ok(KnowledgeDocumentVisibility::Organization),
        3 => Ok(KnowledgeDocumentVisibility::Public),
        _ => Err(KnowledgeDocumentStoreError::Internal(format!(
            "unknown document visibility code: {code}"
        ))),
    }
}

fn document_state_from_code(
    code: i64,
) -> Result<KnowledgeDocumentState, KnowledgeDocumentStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentState::Draft),
        1 => Ok(KnowledgeDocumentState::Ready),
        2 => Ok(KnowledgeDocumentState::Archived),
        3 => Ok(KnowledgeDocumentState::Deleted),
        _ => Err(KnowledgeDocumentStoreError::Internal(format!(
            "unknown document state code: {code}"
        ))),
    }
}

fn version_state_from_code_for_document(
    code: i64,
) -> Result<KnowledgeDocumentVersionState, KnowledgeDocumentStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentVersionState::Pending),
        1 => Ok(KnowledgeDocumentVersionState::Running),
        2 => Ok(KnowledgeDocumentVersionState::Succeeded),
        3 => Ok(KnowledgeDocumentVersionState::Failed),
        _ => Err(KnowledgeDocumentStoreError::Internal(format!(
            "unknown document version state code: {code}"
        ))),
    }
}

fn version_state_from_code(
    code: i64,
) -> Result<KnowledgeDocumentVersionState, KnowledgeDocumentVersionStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentVersionState::Pending),
        1 => Ok(KnowledgeDocumentVersionState::Running),
        2 => Ok(KnowledgeDocumentVersionState::Succeeded),
        3 => Ok(KnowledgeDocumentVersionState::Failed),
        _ => Err(KnowledgeDocumentVersionStoreError::Internal(format!(
            "unknown document version state code: {code}"
        ))),
    }
}

fn ingestion_state_code(value: IngestionJobState) -> i64 {
    match value {
        IngestionJobState::Queued => 0,
        IngestionJobState::Running => 1,
        IngestionJobState::Succeeded => 2,
        IngestionJobState::Failed => 3,
        IngestionJobState::Cancelled => 4,
    }
}

fn ingestion_state_from_code(code: i64) -> Result<IngestionJobState, IngestionJobStoreError> {
    match code {
        0 => Ok(IngestionJobState::Queued),
        1 => Ok(IngestionJobState::Running),
        2 => Ok(IngestionJobState::Succeeded),
        3 => Ok(IngestionJobState::Failed),
        4 => Ok(IngestionJobState::Cancelled),
        _ => Err(IngestionJobStoreError::Internal(format!(
            "unknown ingestion job state code: {code}"
        ))),
    }
}

fn source_now() -> Result<String, KnowledgeSourceStoreError> {
    now_rfc3339().map_err(KnowledgeSourceStoreError::Internal)
}

fn document_now() -> Result<String, KnowledgeDocumentStoreError> {
    now_rfc3339().map_err(KnowledgeDocumentStoreError::Internal)
}

fn version_now() -> Result<String, KnowledgeDocumentVersionStoreError> {
    now_rfc3339().map_err(KnowledgeDocumentVersionStoreError::Internal)
}

fn job_now() -> Result<String, IngestionJobStoreError> {
    now_rfc3339().map_err(IngestionJobStoreError::Internal)
}

fn format_job_timestamp(value: OffsetDateTime) -> Result<String, IngestionJobStoreError> {
    value
        .format(&Rfc3339)
        .map_err(|error| IngestionJobStoreError::Internal(error.to_string()))
}

fn validate_claim_request(
    request: &ClaimIngestionJobsRequest,
) -> Result<(), IngestionJobStoreError> {
    if is_blank(Some(request.claim_owner.as_str())) {
        return Err(IngestionJobStoreError::Conflict(
            "ingestion job claim_owner is required".to_string(),
        ));
    }
    if request.claim_owner.chars().count() > 255 {
        return Err(IngestionJobStoreError::Conflict(
            "ingestion job claim_owner exceeds 255 characters".to_string(),
        ));
    }
    validate_lease_duration(request.lease_duration)
}

fn validate_claim_token(claim_token: &str) -> Result<(), IngestionJobStoreError> {
    if is_blank(Some(claim_token)) || claim_token.len() > 64 {
        return Err(IngestionJobStoreError::Conflict(
            "invalid ingestion job claim_token".to_string(),
        ));
    }
    Ok(())
}

fn validate_lease_duration(lease_duration: Duration) -> Result<(), IngestionJobStoreError> {
    if !lease_duration.is_positive() || lease_duration > MAX_INGESTION_JOB_LEASE {
        return Err(IngestionJobStoreError::Conflict(format!(
            "ingestion job lease must be greater than zero and at most {} seconds",
            MAX_INGESTION_JOB_LEASE.whole_seconds()
        )));
    }
    Ok(())
}

fn now_rfc3339() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| error.to_string())
}

fn source_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeSourceStoreError> {
    to_i64(field, value).map_err(KnowledgeSourceStoreError::Internal)
}

fn document_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeDocumentStoreError> {
    to_i64(field, value).map_err(KnowledgeDocumentStoreError::Internal)
}

fn version_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeDocumentVersionStoreError> {
    to_i64(field, value).map_err(KnowledgeDocumentVersionStoreError::Internal)
}

fn job_to_i64(field: &str, value: u64) -> Result<i64, IngestionJobStoreError> {
    to_i64(field, value).map_err(IngestionJobStoreError::Internal)
}

fn source_from_i64(field: &str, value: i64) -> Result<u64, KnowledgeSourceStoreError> {
    from_i64(field, value).map_err(KnowledgeSourceStoreError::Internal)
}

fn document_from_i64(field: &str, value: i64) -> Result<u64, KnowledgeDocumentStoreError> {
    from_i64(field, value).map_err(KnowledgeDocumentStoreError::Internal)
}

fn version_from_i64(field: &str, value: i64) -> Result<u64, KnowledgeDocumentVersionStoreError> {
    from_i64(field, value).map_err(KnowledgeDocumentVersionStoreError::Internal)
}

fn job_from_i64(field: &str, value: i64) -> Result<u64, IngestionJobStoreError> {
    from_i64(field, value).map_err(IngestionJobStoreError::Internal)
}

fn to_i64(field: &str, value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{field} is out of range"))
}

fn from_i64(field: &str, value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("{field} is negative"))
}

fn source_sqlx_error(error: sqlx::Error) -> KnowledgeSourceStoreError {
    KnowledgeSourceStoreError::Internal(error.to_string())
}

fn source_id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeSourceStoreError {
    KnowledgeSourceStoreError::Internal(error.to_string())
}

fn document_sqlx_error(error: sqlx::Error) -> KnowledgeDocumentStoreError {
    KnowledgeDocumentStoreError::Internal(error.to_string())
}

fn document_id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeDocumentStoreError {
    KnowledgeDocumentStoreError::Internal(error.to_string())
}

fn version_sqlx_error(error: sqlx::Error) -> KnowledgeDocumentVersionStoreError {
    KnowledgeDocumentVersionStoreError::Internal(error.to_string())
}

fn version_id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeDocumentVersionStoreError {
    KnowledgeDocumentVersionStoreError::Internal(error.to_string())
}

fn job_sqlx_error(error: sqlx::Error) -> IngestionJobStoreError {
    IngestionJobStoreError::Internal(error.to_string())
}

fn job_id_error(error: crate::KnowledgeIdGeneratorError) -> IngestionJobStoreError {
    IngestionJobStoreError::Internal(error.to_string())
}

fn job_fetch_error(job_id: i64, error: sqlx::Error) -> IngestionJobStoreError {
    if matches!(error, sqlx::Error::RowNotFound) {
        return IngestionJobStoreError::NotFound(job_id as u64);
    }
    job_sqlx_error(error)
}
use crate::drive_import_linkage_snapshot::{
    decode_drive_object_ref_snapshot, encode_drive_object_ref_snapshot,
};
