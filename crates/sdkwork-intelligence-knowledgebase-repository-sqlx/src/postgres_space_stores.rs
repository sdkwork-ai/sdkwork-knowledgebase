use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_okf_bundle_file_store::{
    CreateKnowledgeOkfBundleFileRecord, KnowledgeOkfBundleFileStore,
    KnowledgeOkfBundleFileStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    BindKnowledgeDriveSpaceRecord, CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
    KnowledgeSpaceStoreError, UpdateKnowledgeSpaceRecord,
};
use sdkwork_knowledgebase_contract::okf_bundle_file::{KnowledgeOkfBundleFile, OkfBundleFileKind};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::{KnowledgeSpace, KnowledgeSpaceStatus};
use sdkwork_utils_rust::is_blank;
use sqlx::{any::AnyRow, AnyPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::sql_timestamp::{utc_sql_timestamp_text, SqlTimestampDialect};
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const ACTIVE_STATUS: i64 = 1;
const PROVISIONING_STATUS: i64 = 0;
const INITIAL_VERSION: i64 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantKnowledgebaseSummary {
    pub space_count: u64,
    pub document_count: u64,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeSpaceStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeSpaceStore {
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

    pub async fn summarize_tenant_knowledgebase(
        &self,
    ) -> Result<TenantKnowledgebaseSummary, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*)
                 FROM kb_space
                 WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
                   AND NOT EXISTS (
                        SELECT 1
                        FROM kb_group_knowledge_space_binding group_binding
                        WHERE group_binding.tenant_id = kb_space.tenant_id
                          AND group_binding.organization_id = kb_space.organization_id
                          AND group_binding.space_id = kb_space.id
                   )) AS space_count,
                (SELECT COUNT(*)
                 FROM kb_document d
                 INNER JOIN kb_space s
                   ON s.id = d.space_id AND s.tenant_id = d.tenant_id
                 WHERE d.tenant_id = $1
                   AND s.organization_id = $2
                   AND d.status = $3
                   AND s.status = $3
                   AND NOT EXISTS (
                        SELECT 1
                        FROM kb_group_knowledge_space_binding group_binding
                        WHERE group_binding.tenant_id = s.tenant_id
                          AND group_binding.organization_id = s.organization_id
                          AND group_binding.space_id = s.id
                   )) AS document_count,
                (SELECT MIN(CAST(created_at AS TEXT))
                 FROM kb_space
                 WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
                   AND NOT EXISTS (
                        SELECT 1
                        FROM kb_group_knowledge_space_binding group_binding
                        WHERE group_binding.tenant_id = kb_space.tenant_id
                          AND group_binding.organization_id = kb_space.organization_id
                          AND group_binding.space_id = kb_space.id
                   )) AS earliest_created_at
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(space_sqlx_error)?;

        Ok(TenantKnowledgebaseSummary {
            space_count: row
                .try_get::<i64, _>("space_count")
                .map_err(space_sqlx_error)? as u64,
            document_count: row
                .try_get::<i64, _>("document_count")
                .map_err(space_sqlx_error)? as u64,
            created_at: optional_timestamp_string(&row, "earliest_created_at")?,
        })
    }

    pub async fn list_active_spaces(
        &self,
        limit: u32,
    ) -> Result<Vec<KnowledgeSpace>, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 500));
        let rows = sqlx::query(
            r#"
            SELECT id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            FROM kb_space
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
              AND NOT EXISTS (
                    SELECT 1
                    FROM kb_group_knowledge_space_binding group_binding
                    WHERE group_binding.tenant_id = kb_space.tenant_id
                      AND group_binding.organization_id = kb_space.organization_id
                      AND group_binding.space_id = kb_space.id
              )
            ORDER BY id DESC
            LIMIT $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(space_sqlx_error)?;

        rows.iter().map(space_from_row).collect()
    }

    pub async fn list_spaces_page(
        &self,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeSpace>, Option<String>, bool), KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let page_size = i64::from(page_size.clamp(1, 200));
        let fetch_limit = page_size + 1;
        let cursor_id = cursor
            .map(|value| space_to_i64("cursor", value))
            .transpose()?;

        let rows = if let Some(after_id) = cursor_id {
            sqlx::query(
                r#"
                SELECT id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
                FROM kb_space
                WHERE tenant_id = $1 AND organization_id = $2 AND status = $3 AND id > $4
                  AND NOT EXISTS (
                        SELECT 1
                        FROM kb_group_knowledge_space_binding group_binding
                        WHERE group_binding.tenant_id = kb_space.tenant_id
                          AND group_binding.organization_id = kb_space.organization_id
                          AND group_binding.space_id = kb_space.id
                  )
                ORDER BY id ASC
                LIMIT $5
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACTIVE_STATUS)
            .bind(after_id)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(space_sqlx_error)?
        } else {
            sqlx::query(
                r#"
                SELECT id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
                FROM kb_space
                WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
                  AND NOT EXISTS (
                        SELECT 1
                        FROM kb_group_knowledge_space_binding group_binding
                        WHERE group_binding.tenant_id = kb_space.tenant_id
                          AND group_binding.organization_id = kb_space.organization_id
                          AND group_binding.space_id = kb_space.id
                  )
                ORDER BY id ASC
                LIMIT $4
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(ACTIVE_STATUS)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(space_sqlx_error)?
        };

        let has_more = rows.len() > page_size as usize;
        let mut items = Vec::new();
        for row in rows.iter().take(page_size as usize) {
            items.push(space_from_row(row)?);
        }
        let next_cursor = if has_more {
            items.last().map(|space| space.id.to_string())
        } else {
            None
        };
        Ok((items, next_cursor, has_more))
    }
}

#[async_trait]
impl KnowledgeSpaceStore for PostgresKnowledgeSpaceStore {
    async fn create_space(
        &self,
        record: CreateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let id = next_i64_id(&self.id_generator).map_err(space_id_error)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;

        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$11");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$12");
        let query = format!(
            r#"
            INSERT INTO kb_space (
                id,
                uuid,
                tenant_id,
                organization_id,
                name,
                description,
                drive_space_id,
                status,
                okf_bundle_initialized,
                knowledge_mode,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, {created_at_expr}, {updated_at_expr}, $13)
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(record.name)
            .bind(record.description)
            .bind(None::<String>)
            .bind(space_status_code(KnowledgeSpaceStatus::Active))
            .bind(bool_code(record.okf_bundle_initialized))
            .bind(space_knowledge_mode_code(record.knowledge_mode))
            .bind(now.clone())
            .bind(&now)
            .bind(INITIAL_VERSION)
            .fetch_one(&self.pool)
            .await
            .map_err(space_sqlx_error)?;

        space_from_row(&row)
    }

    async fn get_space(&self, space_id: u64) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;

        let row = sqlx::query(
            r#"
            SELECT id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            FROM kb_space
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
              AND NOT EXISTS (
                    SELECT 1
                    FROM kb_group_knowledge_space_binding group_binding
                    WHERE group_binding.tenant_id = kb_space.tenant_id
                      AND group_binding.organization_id = kb_space.organization_id
                      AND group_binding.space_id = kb_space.id
              )
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id_i64)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| space_fetch_error(space_id, error))?;

        space_from_row(&row)
    }

    async fn get_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        self.get_group_managed_space_by_status(space_id, ACTIVE_STATUS)
            .await
    }

    async fn get_group_provisioning_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        self.get_group_managed_space_by_status(space_id, PROVISIONING_STATUS)
            .await
    }

    async fn mark_drive_space_bound(
        &self,
        space_id: u64,
        record: BindKnowledgeDriveSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let drive_space_id = require_safe_drive_id(record.drive_space_id, "drive_space_id")?;
        if record.actor_id == 0 {
            return Err(KnowledgeSpaceStoreError::Internal(
                "Wiki publication actor_id must be greater than zero".to_string(),
            ));
        }
        let actor_id = space_to_i64("actor_id", record.actor_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;
        let publication_id = next_i64_id(&self.id_generator).map_err(space_id_error)?;
        let publication_uuid = Uuid::new_v4().to_string();
        let mut transaction = self.pool.begin().await.map_err(space_sqlx_error)?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_space
            SET drive_space_id = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5
              AND status IN ($6, $7)
              AND (drive_space_id IS NULL OR drive_space_id = $1)
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = match sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(&drive_space_id)
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(PROVISIONING_STATUS)
            .fetch_one(&mut *transaction)
            .await
        {
            Ok(row) => row,
            Err(error) => {
                let _ = transaction.rollback().await;
                return Err(space_fetch_error(space_id, error));
            }
        };
        let space = match space_from_row(&row) {
            Ok(space) => space,
            Err(error) => {
                let _ = transaction.rollback().await;
                return Err(error);
            }
        };

        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$9");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$9");
        let publication_query = format!(
            r#"
            INSERT INTO kb_site_publication (
                id, uuid, tenant_id, organization_id, space_id, drive_space_uuid,
                title, created_by, updated_by, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $8, {created_at_expr}, {updated_at_expr}
            )
            ON CONFLICT (tenant_id, space_id) DO NOTHING
            "#,
        );
        if let Err(error) = sqlx::query(sqlx::AssertSqlSafe(publication_query.as_str()))
            .bind(publication_id)
            .bind(publication_uuid)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(&drive_space_id)
            .bind(&space.name)
            .bind(actor_id)
            .bind(&now)
            .execute(&mut *transaction)
            .await
        {
            let _ = transaction.rollback().await;
            return Err(space_sqlx_error(error));
        }

        let existing_drive_space: Option<String> =
            match sqlx::query_scalar(
                r#"
            SELECT drive_space_uuid
            FROM kb_site_publication
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = 1
            "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(row) => row,
                Err(error) => {
                    let _ = transaction.rollback().await;
                    return Err(space_sqlx_error(error));
                }
            };
        let Some(existing_drive_space) = existing_drive_space else {
            let _ = transaction.rollback().await;
            return Err(KnowledgeSpaceStoreError::Conflict(format!(
                "knowledge space {space_id} has no active Wiki publication after binding"
            )));
        };
        if existing_drive_space != drive_space_id {
            let _ = transaction.rollback().await;
            return Err(KnowledgeSpaceStoreError::Conflict(format!(
                "knowledge space {space_id} already owns a Wiki publication for another Drive Space"
            )));
        }

        transaction.commit().await.map_err(space_sqlx_error)?;
        Ok(space)
    }

    async fn mark_okf_bundle_initialized(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let query = format!(
            r#"
            UPDATE kb_space
            SET okf_bundle_initialized = 1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $2 AND organization_id = $3 AND id = $4
              AND status IN ($5, $6)
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(PROVISIONING_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| space_fetch_error(space_id, error))?;

        space_from_row(&row)
    }

    async fn activate_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_space
            SET status = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5
              AND status = $6
              AND EXISTS (
                    SELECT 1
                    FROM kb_group_knowledge_space_binding group_binding
                    WHERE group_binding.tenant_id = kb_space.tenant_id
                      AND group_binding.organization_id = kb_space.organization_id
                      AND group_binding.space_id = kb_space.id
              )
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(space_status_code(KnowledgeSpaceStatus::Active))
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(PROVISIONING_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| space_fetch_error(space_id, error))?;
        space_from_row(&row)
    }

    async fn archive_group_managed_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let current = self.get_group_managed_space_any_status(space_id).await?;
        match current.status {
            KnowledgeSpaceStatus::Archived => Ok(current),
            KnowledgeSpaceStatus::Active | KnowledgeSpaceStatus::Provisioning => {
                let expected_status = space_status_code(current.status);
                match self
                    .transition_group_managed_space_status(
                        space_id,
                        expected_status,
                        KnowledgeSpaceStatus::Archived,
                    )
                    .await
                {
                    Ok(archived) => Ok(archived),
                    Err(error) => {
                        // A concurrent archive may have already converged the physical state.
                        let reloaded = self.get_group_managed_space_any_status(space_id).await?;
                        if reloaded.status == KnowledgeSpaceStatus::Archived {
                            Ok(reloaded)
                        } else {
                            Err(error)
                        }
                    }
                }
            }
            KnowledgeSpaceStatus::Deleted => Err(KnowledgeSpaceStoreError::Conflict(
                "a deleted group-managed space cannot be archived".to_string(),
            )),
        }
    }

    async fn update_space(
        &self,
        space_id: u64,
        record: UpdateKnowledgeSpaceRecord,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        if record
            .name
            .as_ref()
            .is_some_and(|name| is_blank(Some(name.as_str())))
        {
            return Err(KnowledgeSpaceStoreError::Conflict(
                "name must not be blank".to_string(),
            ));
        }

        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;

        // Field-level defaults are resolved inside the UPDATE (COALESCE) instead of a
        // read-modify-write: two concurrent partial updates previously both read the same
        // baseline and the later full-row write resurrected the field the earlier writer
        // had just replaced.
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$3");
        let query = format!(
            r#"
            UPDATE kb_space
            SET name = COALESCE($1, name), description = COALESCE($2, description),
                updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $4 AND organization_id = $5 AND id = $6 AND status = $7
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(record.name)
            .bind(record.description)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(ACTIVE_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| space_fetch_error(space_id, error))?;

        space_from_row(&row)
    }

    async fn update_group_managed_space_description(
        &self,
        space_id: u64,
        description: String,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_space
            SET description = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
              AND EXISTS (
                    SELECT 1
                    FROM kb_group_knowledge_space_binding group_binding
                    WHERE group_binding.tenant_id = kb_space.tenant_id
                      AND group_binding.organization_id = kb_space.organization_id
                      AND group_binding.space_id = kb_space.id
                      AND group_binding.lifecycle_state = 'active'
                      AND group_binding.acl_projection_state = 'active'
              )
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(description)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(ACTIVE_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| space_fetch_error(space_id, error))?;

        space_from_row(&row)
    }

    async fn mark_space_deleted(&self, space_id: u64) -> Result<(), KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;

        let mut transaction = self.pool.begin().await.map_err(space_sqlx_error)?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_space
            SET status = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5
              AND status IN ($6, $7)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(space_status_code(KnowledgeSpaceStatus::Deleted))
            .bind(&now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(ACTIVE_STATUS)
            .bind(PROVISIONING_STATUS)
            .execute(&mut *transaction)
            .await
            .map_err(space_sqlx_error)?;

        let publication_updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let publication_query = format!(
            r#"
            UPDATE kb_site_publication
            SET wiki_status = 'ARCHIVED', status = 0, updated_at = {publication_updated_at_expr},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = 1
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(publication_query.as_str()))
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(&now)
            .execute(&mut *transaction)
            .await
            .map_err(space_sqlx_error)?;

        transaction.commit().await.map_err(space_sqlx_error)?;

        Ok(())
    }
}

impl PostgresKnowledgeSpaceStore {
    async fn transition_group_managed_space_status(
        &self,
        space_id: u64,
        expected_status: i64,
        next_status: KnowledgeSpaceStatus,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeSpaceStoreError::Internal)?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_space
            SET status = $1, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
              AND EXISTS (
                    SELECT 1
                    FROM kb_group_knowledge_space_binding group_binding
                    WHERE group_binding.tenant_id = kb_space.tenant_id
                      AND group_binding.organization_id = kb_space.organization_id
                      AND group_binding.space_id = kb_space.id
              )
            RETURNING id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(space_status_code(next_status))
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id_i64)
            .bind(expected_status)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| space_fetch_error(space_id, error))?;
        space_from_row(&row)
    }

    async fn get_group_managed_space_by_status(
        &self,
        space_id: u64,
        expected_status: i64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let row = sqlx::query(
            r#"
            SELECT space.id, space.uuid, space.name, space.description, space.drive_space_id,
                   space.status, space.okf_bundle_initialized, space.knowledge_mode
            FROM kb_space space
            INNER JOIN kb_group_knowledge_space_binding group_binding
              ON group_binding.tenant_id = space.tenant_id
             AND group_binding.organization_id = space.organization_id
             AND group_binding.space_id = space.id
            WHERE space.tenant_id = $1 AND space.organization_id = $2
              AND space.id = $3 AND space.status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id_i64)
        .bind(expected_status)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| space_fetch_error(space_id, error))?;
        space_from_row(&row)
    }

    async fn get_group_managed_space_any_status(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = space_to_i64("space_id", space_id)?;
        let row = sqlx::query(
            r#"
            SELECT space.id, space.uuid, space.name, space.description, space.drive_space_id,
                   space.status, space.okf_bundle_initialized, space.knowledge_mode
            FROM kb_space space
            INNER JOIN kb_group_knowledge_space_binding group_binding
              ON group_binding.tenant_id = space.tenant_id
             AND group_binding.organization_id = space.organization_id
             AND group_binding.space_id = space.id
            WHERE space.tenant_id = $1 AND space.organization_id = $2 AND space.id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id_i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| space_fetch_error(space_id, error))?;
        space_from_row(&row)
    }

    pub async fn find_first_okf_bundle_initialized_space(
        &self,
    ) -> Result<Option<KnowledgeSpace>, KnowledgeSpaceStoreError> {
        let tenant_id = space_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = space_to_i64("organization_id", self.organization_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
            FROM kb_space
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3 AND okf_bundle_initialized = 1
              AND NOT EXISTS (
                    SELECT 1
                    FROM kb_group_knowledge_space_binding group_binding
                    WHERE group_binding.tenant_id = kb_space.tenant_id
                      AND group_binding.organization_id = kb_space.organization_id
                      AND group_binding.space_id = kb_space.id
              )
            ORDER BY id ASC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(space_sqlx_error)?;

        row.map(|row| space_from_row(&row)).transpose()
    }
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeOkfBundleFileStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeOkfBundleFileStore {
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
impl KnowledgeOkfBundleFileStore for PostgresKnowledgeOkfBundleFileStore {
    async fn create_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        self.insert_file_entry(record).await
    }

    async fn upsert_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        self.upsert_file_entry_record(record).await
    }
}

impl PostgresKnowledgeOkfBundleFileStore {
    async fn insert_file_entry(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        let tenant_id = okf_bundle_file_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = okf_bundle_file_to_i64("organization_id", self.organization_id)?;
        let space_id = okf_bundle_file_to_i64("space_id", record.space_id)?;
        let id = next_i64_id(&self.id_generator).map_err(okf_bundle_file_id_error)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeOkfBundleFileStoreError::Internal)?;

        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$14");
        let query = format!(
            r#"
            INSERT INTO kb_okf_bundle_file (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                logical_path,
                file_kind,
                artifact_role,
                drive_bucket,
                drive_object_key,
                checksum_sha256_hex,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {created_at_expr}, {updated_at_expr}, $15)
            RETURNING
                id,
                space_id,
                logical_path,
                file_kind,
                artifact_role,
                drive_bucket,
                drive_object_key,
                checksum_sha256_hex
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(record.logical_path)
            .bind(okf_bundle_file_type_code(record.file_kind))
            .bind(record.artifact_role)
            .bind(record.drive_bucket)
            .bind(record.drive_object_key)
            .bind(record.checksum_sha256_hex)
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_one(&self.pool)
            .await
            .map_err(okf_bundle_file_sqlx_error)?;

        okf_bundle_file_from_row(&row)
    }

    async fn upsert_file_entry_record(
        &self,
        record: CreateKnowledgeOkfBundleFileRecord,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        let tenant_id = okf_bundle_file_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = okf_bundle_file_to_i64("organization_id", self.organization_id)?;
        let space_id = okf_bundle_file_to_i64("space_id", record.space_id)?;
        let id = next_i64_id(&self.id_generator).map_err(okf_bundle_file_id_error)?;
        let now = utc_sql_timestamp_text().map_err(KnowledgeOkfBundleFileStoreError::Internal)?;

        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$14");
        let query = format!(
            r#"
            INSERT INTO kb_okf_bundle_file (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                logical_path,
                file_kind,
                artifact_role,
                drive_bucket,
                drive_object_key,
                checksum_sha256_hex,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {created_at_expr}, {updated_at_expr}, $15)
            ON CONFLICT(tenant_id, organization_id, space_id, logical_path)
            DO UPDATE SET
                file_kind = excluded.file_kind,
                artifact_role = excluded.artifact_role,
                drive_bucket = excluded.drive_bucket,
                drive_object_key = excluded.drive_object_key,
                checksum_sha256_hex = excluded.checksum_sha256_hex,
                status = excluded.status,
                updated_at = excluded.updated_at,
                version = kb_okf_bundle_file.version + 1
            RETURNING
                id,
                space_id,
                logical_path,
                file_kind,
                artifact_role,
                drive_bucket,
                drive_object_key,
                checksum_sha256_hex
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(record.logical_path)
            .bind(okf_bundle_file_type_code(record.file_kind))
            .bind(record.artifact_role)
            .bind(record.drive_bucket)
            .bind(record.drive_object_key)
            .bind(record.checksum_sha256_hex)
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_one(&self.pool)
            .await
            .map_err(okf_bundle_file_sqlx_error)?;

        okf_bundle_file_from_row(&row)
    }

    pub async fn list_file_entries(
        &self,
    ) -> Result<Vec<KnowledgeOkfBundleFile>, KnowledgeOkfBundleFileStoreError> {
        let tenant_id = okf_bundle_file_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = okf_bundle_file_to_i64("organization_id", self.organization_id)?;
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                space_id,
                logical_path,
                file_kind,
                artifact_role,
                drive_bucket,
                drive_object_key,
                checksum_sha256_hex
            FROM kb_okf_bundle_file
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
        .map_err(okf_bundle_file_sqlx_error)?;

        rows.iter().map(okf_bundle_file_from_row).collect()
    }

    pub async fn list_file_entries_page(
        &self,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeOkfBundleFile>, Option<String>, bool), KnowledgeOkfBundleFileStoreError>
    {
        let tenant_id = okf_bundle_file_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = okf_bundle_file_to_i64("organization_id", self.organization_id)?;
        let cursor = cursor
            .map(|value| okf_bundle_file_to_i64("cursor", value))
            .transpose()?;
        let page_size = page_size.clamp(1, 200) as usize;
        let rows = sqlx::query(
            r#"
            SELECT id, space_id, logical_path, file_kind, artifact_role,
                   drive_bucket, drive_object_key, checksum_sha256_hex
            FROM kb_okf_bundle_file
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
        .map_err(okf_bundle_file_sqlx_error)?;

        let has_more = rows.len() > page_size;
        let items = rows
            .iter()
            .take(page_size)
            .map(okf_bundle_file_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.id.to_string()))
            .flatten();
        Ok((items, next_cursor, has_more))
    }

    pub async fn get_file_entry_by_id(
        &self,
        entry_id: u64,
    ) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
        let tenant_id = okf_bundle_file_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = okf_bundle_file_to_i64("organization_id", self.organization_id)?;
        let entry_id = okf_bundle_file_to_i64("entry_id", entry_id)?;
        let row = sqlx::query(
            r#"
            SELECT
                id,
                space_id,
                logical_path,
                file_kind,
                artifact_role,
                drive_bucket,
                drive_object_key,
                checksum_sha256_hex
            FROM kb_okf_bundle_file
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(entry_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(okf_bundle_file_sqlx_error)?
        .ok_or(KnowledgeOkfBundleFileStoreError::NotFound(entry_id as u64))?;

        okf_bundle_file_from_row(&row)
    }
}

fn space_from_row(row: &AnyRow) -> Result<KnowledgeSpace, KnowledgeSpaceStoreError> {
    let status_code: i64 = row.try_get("status").map_err(space_sqlx_error)?;
    let okf_bundle_initialized: i64 = row
        .try_get("okf_bundle_initialized")
        .map_err(space_sqlx_error)?;
    Ok(KnowledgeSpace {
        id: space_from_i64("id", row.try_get("id").map_err(space_sqlx_error)?)?,
        uuid: row.try_get("uuid").map_err(space_sqlx_error)?,
        name: row.try_get("name").map_err(space_sqlx_error)?,
        description: row.try_get("description").map_err(space_sqlx_error)?,
        drive_space_id: row.try_get("drive_space_id").map_err(space_sqlx_error)?,
        status: space_status_from_code(status_code)?,
        okf_bundle_initialized: okf_bundle_initialized != 0,
        knowledge_mode: space_knowledge_mode_from_row(row)?,
    })
}

fn space_knowledge_mode_code(mode: KnowledgeAgentKnowledgeMode) -> &'static str {
    mode.as_str()
}

fn space_knowledge_mode_from_row(
    row: &AnyRow,
) -> Result<KnowledgeAgentKnowledgeMode, KnowledgeSpaceStoreError> {
    let value: Option<String> = row.try_get("knowledge_mode").map_err(space_sqlx_error)?;
    match value.as_deref().unwrap_or("okf_bundle") {
        "okf_bundle" => Ok(KnowledgeAgentKnowledgeMode::OkfBundle),
        "rag" => Ok(KnowledgeAgentKnowledgeMode::Rag),
        "external" => Ok(KnowledgeAgentKnowledgeMode::External),
        other => Err(KnowledgeSpaceStoreError::Internal(format!(
            "unsupported knowledge_mode value: {other}"
        ))),
    }
}

fn require_safe_drive_id(
    value: String,
    field_name: &str,
) -> Result<String, KnowledgeSpaceStoreError> {
    let value = value.trim().to_string();
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    {
        return Err(KnowledgeSpaceStoreError::Internal(format!(
            "invalid {field_name}"
        )));
    }
    Ok(value)
}

fn okf_bundle_file_from_row(
    row: &AnyRow,
) -> Result<KnowledgeOkfBundleFile, KnowledgeOkfBundleFileStoreError> {
    let file_kind: String = row
        .try_get("file_kind")
        .map_err(okf_bundle_file_sqlx_error)?;
    Ok(KnowledgeOkfBundleFile {
        id: okf_bundle_file_from_i64("id", row.try_get("id").map_err(okf_bundle_file_sqlx_error)?)?,
        space_id: okf_bundle_file_from_i64(
            "space_id",
            row.try_get("space_id")
                .map_err(okf_bundle_file_sqlx_error)?,
        )?,
        logical_path: row
            .try_get("logical_path")
            .map_err(okf_bundle_file_sqlx_error)?,
        file_kind: okf_bundle_file_type_from_code(&file_kind)?,
        artifact_role: row
            .try_get("artifact_role")
            .map_err(okf_bundle_file_sqlx_error)?,
        drive_bucket: row
            .try_get("drive_bucket")
            .map_err(okf_bundle_file_sqlx_error)?,
        drive_object_key: row
            .try_get("drive_object_key")
            .map_err(okf_bundle_file_sqlx_error)?,
        checksum_sha256_hex: row
            .try_get("checksum_sha256_hex")
            .map_err(okf_bundle_file_sqlx_error)?,
        staged_import_root: None,
        import_id: None,
    })
}

fn space_status_code(value: KnowledgeSpaceStatus) -> i64 {
    match value {
        KnowledgeSpaceStatus::Provisioning => PROVISIONING_STATUS,
        KnowledgeSpaceStatus::Active => 1,
        KnowledgeSpaceStatus::Archived => 2,
        KnowledgeSpaceStatus::Deleted => 3,
    }
}

fn space_status_from_code(code: i64) -> Result<KnowledgeSpaceStatus, KnowledgeSpaceStoreError> {
    match code {
        PROVISIONING_STATUS => Ok(KnowledgeSpaceStatus::Provisioning),
        1 => Ok(KnowledgeSpaceStatus::Active),
        2 => Ok(KnowledgeSpaceStatus::Archived),
        3 => Ok(KnowledgeSpaceStatus::Deleted),
        _ => Err(KnowledgeSpaceStoreError::Internal(format!(
            "unknown knowledge space status code: {code}"
        ))),
    }
}

fn okf_bundle_file_type_code(value: OkfBundleFileKind) -> &'static str {
    match value {
        OkfBundleFileKind::BundleProfile => "bundle_profile",
        OkfBundleFileKind::BundleAgents => "bundle_agents",
        OkfBundleFileKind::BundleIndex => "bundle_index",
        OkfBundleFileKind::BundleLog => "bundle_log",
        OkfBundleFileKind::ConceptRevision => "concept_revision",
        OkfBundleFileKind::GraphExport => "graph_export",
        OkfBundleFileKind::ContextPack => "context_pack",
        OkfBundleFileKind::OutputExport => "output_export",
    }
}

fn okf_bundle_file_type_from_code(
    value: &str,
) -> Result<OkfBundleFileKind, KnowledgeOkfBundleFileStoreError> {
    match value {
        "bundle_profile" => Ok(OkfBundleFileKind::BundleProfile),
        "bundle_agents" => Ok(OkfBundleFileKind::BundleAgents),
        "bundle_index" => Ok(OkfBundleFileKind::BundleIndex),
        "bundle_log" => Ok(OkfBundleFileKind::BundleLog),
        "concept_revision" => Ok(OkfBundleFileKind::ConceptRevision),
        "graph_export" => Ok(OkfBundleFileKind::GraphExport),
        "context_pack" => Ok(OkfBundleFileKind::ContextPack),
        "output_export" => Ok(OkfBundleFileKind::OutputExport),
        _ => Err(KnowledgeOkfBundleFileStoreError::Internal(format!(
            "unknown okf bundle file kind: {value}"
        ))),
    }
}

fn bool_code(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn optional_timestamp_string(
    row: &AnyRow,
    column: &str,
) -> Result<Option<String>, KnowledgeSpaceStoreError> {
    let value = row
        .try_get::<Option<String>, _>(column)
        .map_err(space_sqlx_error)?;
    Ok(value.filter(|text| !is_blank(Some(text.as_str()))))
}

fn space_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeSpaceStoreError> {
    to_i64(field, value).map_err(KnowledgeSpaceStoreError::Internal)
}

fn okf_bundle_file_to_i64(
    field: &str,
    value: u64,
) -> Result<i64, KnowledgeOkfBundleFileStoreError> {
    to_i64(field, value).map_err(KnowledgeOkfBundleFileStoreError::Internal)
}

fn space_from_i64(field: &str, value: i64) -> Result<u64, KnowledgeSpaceStoreError> {
    from_i64(field, value).map_err(KnowledgeSpaceStoreError::Internal)
}

fn okf_bundle_file_from_i64(
    field: &str,
    value: i64,
) -> Result<u64, KnowledgeOkfBundleFileStoreError> {
    from_i64(field, value).map_err(KnowledgeOkfBundleFileStoreError::Internal)
}

fn to_i64(field: &str, value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{field} is out of range"))
}

fn from_i64(field: &str, value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("{field} is negative"))
}

fn space_sqlx_error(error: sqlx::Error) -> KnowledgeSpaceStoreError {
    KnowledgeSpaceStoreError::Internal(error.to_string())
}

fn space_id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeSpaceStoreError {
    KnowledgeSpaceStoreError::Internal(error.to_string())
}

fn okf_bundle_file_sqlx_error(error: sqlx::Error) -> KnowledgeOkfBundleFileStoreError {
    KnowledgeOkfBundleFileStoreError::Internal(error.to_string())
}

fn okf_bundle_file_id_error(
    error: crate::KnowledgeIdGeneratorError,
) -> KnowledgeOkfBundleFileStoreError {
    KnowledgeOkfBundleFileStoreError::Internal(error.to_string())
}

fn space_fetch_error(space_id: u64, error: sqlx::Error) -> KnowledgeSpaceStoreError {
    if matches!(error, sqlx::Error::RowNotFound) {
        return KnowledgeSpaceStoreError::NotFound(format!(
            "knowledge space {space_id} was not found"
        ));
    }
    space_sqlx_error(error)
}
