use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_object_ref_store::{
    CreateKnowledgeDriveObjectRefRecord, KnowledgeDriveObjectRefStore,
    KnowledgeDriveObjectRefStoreError,
};
use sdkwork_intelligence_knowledgebase_service::tenant_quota::ensure_storage_capacity;
use sdkwork_knowledgebase_contract::KnowledgeDriveObjectRef;
use sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits;
use sqlx::{any::AnyRow, AnyConnection, AnyPool, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};
use crate::quota_transaction::begin_tenant_quota_transaction;

const ACTIVE_STATUS: i64 = 1;

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeDriveObjectRefStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    database_engine: DatabaseEngine,
    quota_limits: Option<KnowledgebaseTenantQuotaLimits>,
}

impl PostgresKnowledgeDriveObjectRefStore {
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
impl KnowledgeDriveObjectRefStore for PostgresKnowledgeDriveObjectRefStore {
    async fn create_object_ref(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        if let Some(limits) = self.quota_limits {
            return self.create_object_ref_with_quota(record, limits).await;
        }
        self.insert_object_ref(record).await
    }

    async fn create_or_get_object_ref(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        if let Some(limits) = self.quota_limits {
            return self
                .create_or_get_object_ref_with_quota(record, limits)
                .await;
        }
        if let Some(object_ref) = self.insert_object_ref_if_absent(record.clone()).await? {
            return Ok(object_ref);
        }

        let object_ref = self.get_object_ref_by_locator(&record).await?;
        self.enrich_object_ref_drive_binding(object_ref, &record)
            .await
    }

    async fn list_object_refs_by_logical_path_prefix(
        &self,
        space_id: u64,
        prefix: &str,
    ) -> Result<Vec<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        PostgresKnowledgeDriveObjectRefStore::list_object_refs_by_logical_path_prefix(
            self, space_id, prefix,
        )
        .await
    }

    async fn get_object_ref_by_id(
        &self,
        object_ref_id: u64,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let object_ref_id = to_i64("object_ref_id", object_ref_id)?;
        let row = sqlx::query(
            r#"
            SELECT
                id,
                space_id,
                drive_provider_kind,
                drive_space_id,
                drive_node_id,
                logical_path,
                drive_storage_provider_id,
                drive_bucket,
                drive_object_key,
                drive_object_version,
                drive_etag,
                content_type,
                size_bytes,
                checksum_sha256_hex,
                object_role,
                access_mode
            FROM kb_drive_object_ref
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(object_ref_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?
        .ok_or_else(|| {
            KnowledgeDriveObjectRefStoreError::Internal(format!(
                "drive object ref not found: {object_ref_id}"
            ))
        })?;

        object_ref_from_row(&row)
    }
}

impl PostgresKnowledgeDriveObjectRefStore {
    async fn create_object_ref_with_quota(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(sqlx_error)?;
        self.ensure_storage_quota_on(&mut transaction, record.size_bytes, limits)
            .await?;
        let object_ref = self
            .insert_object_ref_with_conflict_clause_on(record, "", &mut transaction)
            .await?
            .ok_or_else(|| {
                KnowledgeDriveObjectRefStoreError::Internal(
                    "failed to insert drive object ref".to_string(),
                )
            })?;
        transaction.commit().await.map_err(sqlx_error)?;
        Ok(object_ref)
    }

    async fn create_or_get_object_ref_with_quota(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let mut transaction =
            begin_tenant_quota_transaction(&self.pool, tenant_id, organization_id)
                .await
                .map_err(sqlx_error)?;

        if let Some(object_ref) = self
            .find_object_ref_by_locator_on(&record, &mut transaction)
            .await?
        {
            transaction.commit().await.map_err(sqlx_error)?;
            return self
                .enrich_object_ref_drive_binding(object_ref, &record)
                .await;
        }

        self.ensure_storage_quota_on(&mut transaction, record.size_bytes, limits)
            .await?;
        let object_ref = self
            .insert_object_ref_with_conflict_clause_on(
                record.clone(),
                "ON CONFLICT DO NOTHING",
                &mut transaction,
            )
            .await?;
        let object_ref = match object_ref {
            Some(object_ref) => object_ref,
            None => self
                .find_object_ref_by_locator_on(&record, &mut transaction)
                .await?
                .ok_or_else(|| {
                    KnowledgeDriveObjectRefStoreError::Internal(
                        "object-ref identity conflict did not resolve to an active record"
                            .to_string(),
                    )
                })?,
        };
        transaction.commit().await.map_err(sqlx_error)?;
        self.enrich_object_ref_drive_binding(object_ref, &record)
            .await
    }

    async fn ensure_storage_quota_on(
        &self,
        connection: &mut AnyConnection,
        additional_bytes: u64,
        limits: KnowledgebaseTenantQuotaLimits,
    ) -> Result<(), KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        // O(1) trigger-maintained counter read; falls back to the exact
        // aggregate scan only while the counter row is not materialized yet.
        let total = crate::quota_transaction::tenant_active_storage_bytes(
            connection,
            tenant_id,
            organization_id,
        )
        .await
        .map_err(sqlx_error)?;
        ensure_storage_capacity(total, additional_bytes, &limits)?;
        Ok(())
    }

    async fn get_object_ref_by_locator(
        &self,
        record: &CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let mut connection = self.pool.acquire().await.map_err(sqlx_error)?;
        self.find_object_ref_by_locator_on(record, &mut connection)
            .await?
            .ok_or_else(|| {
                KnowledgeDriveObjectRefStoreError::Internal(
                    "drive object ref locator did not resolve to an active record".to_string(),
                )
            })
    }

    async fn find_object_ref_by_locator_on(
        &self,
        record: &CreateKnowledgeDriveObjectRefRecord,
        connection: &mut AnyConnection,
    ) -> Result<Option<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", record.space_id)?;
        let row = sqlx::query(
            r#"
            SELECT
                id,
                space_id,
                drive_provider_kind,
                drive_space_id,
                drive_node_id,
                logical_path,
                drive_storage_provider_id,
                drive_bucket,
                drive_object_key,
                drive_object_version,
                drive_etag,
                content_type,
                size_bytes,
                checksum_sha256_hex,
                object_role,
                access_mode
            FROM kb_drive_object_ref
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND drive_storage_provider_id = $4
              AND drive_bucket = $5
              AND drive_object_key = $6
              AND COALESCE(drive_object_version, '') = COALESCE($7, '')
              AND object_role = $8
              AND status = $9
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(&record.drive_storage_provider_id)
        .bind(&record.drive_bucket)
        .bind(&record.drive_object_key)
        .bind(&record.drive_object_version)
        .bind(&record.object_role)
        .bind(ACTIVE_STATUS)
        .fetch_optional(connection)
        .await
        .map_err(sqlx_error)?;

        row.map(|row| object_ref_from_row(&row)).transpose()
    }

    async fn enrich_object_ref_drive_binding(
        &self,
        object_ref: KnowledgeDriveObjectRef,
        record: &CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        let should_update_drive_space =
            object_ref.drive_space_id.is_none() && record.drive_space_id.is_some();
        let should_update_drive_node =
            object_ref.drive_node_id.is_none() && record.drive_node_id.is_some();
        let should_update_logical_path =
            object_ref.logical_path.is_none() && record.logical_path.is_some();
        if !should_update_drive_space && !should_update_drive_node && !should_update_logical_path {
            return Ok(object_ref);
        }

        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let object_ref_id = to_i64("object_ref_id", object_ref.id)?;
        let now = now_rfc3339()?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let query = format!(
            r#"
            UPDATE kb_drive_object_ref
            SET drive_space_id = COALESCE(drive_space_id, $1),
                drive_node_id = COALESCE(drive_node_id, $2),
                logical_path = COALESCE(logical_path, $3),
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $5 AND organization_id = $6 AND id = $7 AND status = $8
            RETURNING
                id,
                space_id,
                drive_provider_kind,
                drive_space_id,
                drive_node_id,
                logical_path,
                drive_storage_provider_id,
                drive_bucket,
                drive_object_key,
                drive_object_version,
                drive_etag,
                content_type,
                size_bytes,
                checksum_sha256_hex,
                object_role,
                access_mode
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(&record.drive_space_id)
            .bind(&record.drive_node_id)
            .bind(&record.logical_path)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(object_ref_id)
            .bind(ACTIVE_STATUS)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error)?;

        object_ref_from_row(&row)
    }

    pub async fn list_object_refs_by_logical_path_prefix(
        &self,
        space_id: u64,
        prefix: &str,
    ) -> Result<Vec<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let prefix = escape_sql_like_prefix(prefix.trim_end_matches('/'));
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                space_id,
                drive_provider_kind,
                drive_space_id,
                drive_node_id,
                logical_path,
                drive_storage_provider_id,
                drive_bucket,
                drive_object_key,
                drive_object_version,
                drive_etag,
                content_type,
                size_bytes,
                checksum_sha256_hex,
                object_role,
                access_mode
            FROM kb_drive_object_ref
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND logical_path LIKE $4 ESCAPE '\'
              AND status = $5
            ORDER BY logical_path ASC
            LIMIT 200
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(prefix)
        .bind(ACTIVE_STATUS)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error)?;

        rows.iter().map(object_ref_from_row).collect()
    }

    async fn insert_object_ref_if_absent(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<Option<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        self.insert_object_ref_with_conflict_clause(record, "ON CONFLICT DO NOTHING")
            .await
    }

    async fn insert_object_ref(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
    ) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
        self.insert_object_ref_with_conflict_clause(record, "")
            .await?
            .ok_or_else(|| {
                KnowledgeDriveObjectRefStoreError::Internal(
                    "failed to insert drive object ref".to_string(),
                )
            })
    }

    async fn insert_object_ref_with_conflict_clause(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
        conflict_clause: &str,
    ) -> Result<Option<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        let mut connection = self.pool.acquire().await.map_err(sqlx_error)?;
        self.insert_object_ref_with_conflict_clause_on(record, conflict_clause, &mut connection)
            .await
    }

    async fn insert_object_ref_with_conflict_clause_on(
        &self,
        record: CreateKnowledgeDriveObjectRefRecord,
        conflict_clause: &str,
        connection: &mut AnyConnection,
    ) -> Result<Option<KnowledgeDriveObjectRef>, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", record.space_id)?;
        let size_bytes = to_i64("size_bytes", record.size_bytes)?;
        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let now = now_rfc3339()?;
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$21");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$22");
        let query = format!(
            r#"
            INSERT INTO kb_drive_object_ref (
                id,
                uuid,
                tenant_id,
                organization_id,
                space_id,
                drive_provider_kind,
                drive_space_id,
                drive_node_id,
                logical_path,
                drive_storage_provider_id,
                drive_bucket,
                drive_object_key,
                drive_object_version,
                drive_etag,
                content_type,
                size_bytes,
                checksum_sha256_hex,
                object_role,
                access_mode,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, {created_at_expr}, {updated_at_expr}, 0)
            {conflict_clause}
            RETURNING
                id,
                space_id,
                drive_provider_kind,
                drive_space_id,
                drive_node_id,
                logical_path,
                drive_storage_provider_id,
                drive_bucket,
                drive_object_key,
                drive_object_version,
                drive_etag,
                content_type,
                size_bytes,
                checksum_sha256_hex,
                object_role,
                access_mode
            "#
        );

        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(record.drive_provider_kind)
            .bind(record.drive_space_id)
            .bind(record.drive_node_id)
            .bind(record.logical_path)
            .bind(record.drive_storage_provider_id)
            .bind(record.drive_bucket)
            .bind(record.drive_object_key)
            .bind(record.drive_object_version)
            .bind(record.drive_etag)
            .bind(record.content_type)
            .bind(size_bytes)
            .bind(record.checksum_sha256_hex)
            .bind(record.object_role)
            .bind(record.access_mode)
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .fetch_optional(connection)
            .await
            .map_err(sqlx_error)?;

        row.map(|row| object_ref_from_row(&row)).transpose()
    }

    pub async fn sum_active_storage_bytes(&self) -> Result<u64, KnowledgeDriveObjectRefStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let total: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(size_bytes), 0)
            FROM kb_drive_object_ref
            WHERE tenant_id = $1
              AND organization_id = $2
              AND status = $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(ACTIVE_STATUS)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error)?;

        Ok(u64::try_from(total.unwrap_or(0)).unwrap_or(0))
    }
}

fn object_ref_from_row(
    row: &AnyRow,
) -> Result<KnowledgeDriveObjectRef, KnowledgeDriveObjectRefStoreError> {
    Ok(KnowledgeDriveObjectRef {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(sqlx_error)?)?,
        drive_space_id: row.try_get("drive_space_id").map_err(sqlx_error)?,
        drive_node_id: row.try_get("drive_node_id").map_err(sqlx_error)?,
        logical_path: row.try_get("logical_path").map_err(sqlx_error)?,
        drive_provider_kind: row.try_get("drive_provider_kind").map_err(sqlx_error)?,
        drive_storage_provider_id: row
            .try_get("drive_storage_provider_id")
            .map_err(sqlx_error)?,
        drive_bucket: row.try_get("drive_bucket").map_err(sqlx_error)?,
        drive_object_key: row.try_get("drive_object_key").map_err(sqlx_error)?,
        drive_object_version: row.try_get("drive_object_version").map_err(sqlx_error)?,
        drive_etag: row.try_get("drive_etag").map_err(sqlx_error)?,
        content_type: row.try_get("content_type").map_err(sqlx_error)?,
        size_bytes: from_i64("size_bytes", row.try_get("size_bytes").map_err(sqlx_error)?)?,
        checksum_sha256_hex: row.try_get("checksum_sha256_hex").map_err(sqlx_error)?,
        object_role: row.try_get("object_role").map_err(sqlx_error)?,
        access_mode: row.try_get("access_mode").map_err(sqlx_error)?,
    })
}

fn now_rfc3339() -> Result<String, KnowledgeDriveObjectRefStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeDriveObjectRefStoreError::Internal(error.to_string()))
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeDriveObjectRefStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeDriveObjectRefStoreError::Internal(format!("{field} is out of range"))
    })
}

fn from_i64(field: &str, value: i64) -> Result<u64, KnowledgeDriveObjectRefStoreError> {
    u64::try_from(value)
        .map_err(|_| KnowledgeDriveObjectRefStoreError::Internal(format!("{field} is negative")))
}

fn sqlx_error(error: sqlx::Error) -> KnowledgeDriveObjectRefStoreError {
    KnowledgeDriveObjectRefStoreError::Internal(error.to_string())
}

fn id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeDriveObjectRefStoreError {
    KnowledgeDriveObjectRefStoreError::Internal(error.to_string())
}

fn escape_sql_like_prefix(prefix: &str) -> String {
    let mut escaped = String::with_capacity(prefix.len() + 4);
    for ch in prefix.chars() {
        match ch {
            '\\' | '%' | '_' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            other => escaped.push(other),
        }
    }
    format!("{escaped}%")
}

#[cfg(test)]
mod like_escape_tests {
    use super::escape_sql_like_prefix;

    #[test]
    fn escape_sql_like_prefix_escapes_wildcards() {
        assert_eq!(escape_sql_like_prefix("okf/%test"), "okf/\\%test%");
        assert_eq!(escape_sql_like_prefix("a_b"), "a\\_b%");
    }
}
