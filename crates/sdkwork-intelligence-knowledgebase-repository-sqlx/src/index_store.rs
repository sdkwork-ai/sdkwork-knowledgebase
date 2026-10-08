use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_index_store::{
    KnowledgeIndexStore, KnowledgeIndexStoreError as PortKnowledgeIndexStoreError,
};
use sdkwork_knowledgebase_contract::rag::{KnowledgeIndex, KnowledgeIndexRequest};
use sdkwork_utils_rust::is_blank;
use sqlx::{any::AnyRow, AnyPool, Row};
use std::sync::Arc;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const ACTIVE_STATUS: i64 = 1;
const INITIAL_VERSION: i64 = 0;
const DEFAULT_SCHEMA_VERSION: &str = "2026-06-01";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeIndexStoreError {
    #[error("knowledge index was not found: {0}")]
    NotFound(String),
    #[error("knowledge index store internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeIndexStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeIndexStore {
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

    pub async fn create_index(
        &self,
        request: KnowledgeIndexRequest,
    ) -> Result<KnowledgeIndex, KnowledgeIndexStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;
        if is_blank(Some(request.index_kind.as_str())) {
            return Err(KnowledgeIndexStoreError::Internal(
                "index_kind is required".to_string(),
            ));
        }

        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", request.space_id)?;
        let dimension = request.dimension.map(i64::from).unwrap_or_default();
        let now = now_rfc3339()?;

        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$14");
        let query = format!(
            r#"
            INSERT INTO kb_index (
                id, uuid, tenant_id, organization_id, space_id, index_kind,
                embedding_provider_id, embedding_model, dimension, metric,
                schema_version, status, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {created_at_expr}, {updated_at_expr}, $15)
            RETURNING id, tenant_id, space_id, index_kind, status
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(space_id)
            .bind(request.index_kind)
            .bind(request.embedding_provider_id)
            .bind(request.embedding_model)
            .bind(dimension)
            .bind(request.metric)
            .bind(DEFAULT_SCHEMA_VERSION)
            .bind(ACTIVE_STATUS)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error)?;

        index_from_row(&row)
    }

    pub async fn get_index(
        &self,
        index_id: u64,
    ) -> Result<KnowledgeIndex, KnowledgeIndexStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let index_id = to_i64("index_id", index_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, space_id, index_kind, status
            FROM kb_index
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(index_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?
        .ok_or_else(|| {
            KnowledgeIndexStoreError::NotFound(format!("knowledge index {index_id} was not found"))
        })?;

        index_from_row(&row)
    }

    pub async fn list_active_indexes(
        &self,
        limit: u32,
    ) -> Result<Vec<KnowledgeIndex>, KnowledgeIndexStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 500));
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, space_id, index_kind, status
            FROM kb_index
            WHERE tenant_id = $1 AND organization_id = $2 AND status = $3
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
        .map_err(sqlx_error)?;

        rows.iter().map(index_from_row).collect()
    }

    pub async fn list_active_indexes_page(
        &self,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<(Vec<KnowledgeIndex>, Option<String>, bool), KnowledgeIndexStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let cursor = cursor.map(|value| to_i64("cursor", value)).transpose()?;
        let page_size = page_size.clamp(1, 200) as usize;
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, space_id, index_kind, status
            FROM kb_index
            WHERE tenant_id = $1
              AND organization_id = $2
              AND status = $3
              AND ($4 IS NULL OR id > $4)
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
        .map_err(sqlx_error)?;

        let has_more = rows.len() > page_size;
        let items = rows
            .iter()
            .take(page_size)
            .map(index_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.index_id.to_string()))
            .flatten();
        Ok((items, next_cursor, has_more))
    }

    pub async fn get_or_create_active_vector_index(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeIndex, KnowledgeIndexStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;

        if let Some(index) = self
            .find_active_vector_index(tenant_id, organization_id, space_id)
            .await?
        {
            return Ok(index);
        }

        let create_result = self
            .create_index(KnowledgeIndexRequest {
                tenant_id: self.tenant_id,
                space_id: u64::try_from(space_id).map_err(|_| {
                    KnowledgeIndexStoreError::Internal("space_id out of u64 range".to_string())
                })?,
                index_kind: "vector".to_string(),
                embedding_provider_id: None,
                embedding_model: None,
                dimension: Some(1536),
                metric: Some("cosine".to_string()),
            })
            .await;
        match create_result {
            Ok(index) => Ok(index),
            Err(create_error) => self
                .find_active_vector_index(tenant_id, organization_id, space_id)
                .await?
                .ok_or(create_error),
        }
    }

    async fn find_active_vector_index(
        &self,
        tenant_id: i64,
        organization_id: i64,
        space_id: i64,
    ) -> Result<Option<KnowledgeIndex>, KnowledgeIndexStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, space_id, index_kind, status
            FROM kb_index
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND index_kind = $4
              AND status = $5
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind("vector")
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?;
        row.as_ref().map(index_from_row).transpose()
    }
}

#[async_trait]
impl KnowledgeIndexStore for PostgresKnowledgeIndexStore {
    async fn get_index(
        &self,
        index_id: u64,
    ) -> Result<KnowledgeIndex, PortKnowledgeIndexStoreError> {
        PostgresKnowledgeIndexStore::get_index(self, index_id)
            .await
            .map_err(map_index_store_port_error)
    }

    async fn get_or_create_active_vector_index(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeIndex, PortKnowledgeIndexStoreError> {
        PostgresKnowledgeIndexStore::get_or_create_active_vector_index(self, space_id)
            .await
            .map_err(map_index_store_port_error)
    }
}

fn map_index_store_port_error(error: KnowledgeIndexStoreError) -> PortKnowledgeIndexStoreError {
    PortKnowledgeIndexStoreError::Internal(error.to_string())
}

fn index_from_row(row: &AnyRow) -> Result<KnowledgeIndex, KnowledgeIndexStoreError> {
    Ok(KnowledgeIndex {
        index_id: from_u64("id", row.try_get("id").map_err(sqlx_error)?)?,
        tenant_id: from_u64("tenant_id", row.try_get("tenant_id").map_err(sqlx_error)?)?,
        space_id: from_u64("space_id", row.try_get("space_id").map_err(sqlx_error)?)?,
        index_kind: row.try_get("index_kind").map_err(sqlx_error)?,
        status: status_name(row.try_get("status").map_err(sqlx_error)?)?,
    })
}

fn status_name(code: i64) -> Result<String, KnowledgeIndexStoreError> {
    match code {
        ACTIVE_STATUS => Ok("active".to_string()),
        0 => Ok("inactive".to_string()),
        value => Err(KnowledgeIndexStoreError::Internal(format!(
            "unsupported index status code: {value}"
        ))),
    }
}

fn ensure_tenant_scope(
    configured_tenant_id: u64,
    request_tenant_id: u64,
) -> Result<(), KnowledgeIndexStoreError> {
    if configured_tenant_id != request_tenant_id {
        return Err(KnowledgeIndexStoreError::Internal(
            "tenant_id does not match configured repository tenant".to_string(),
        ));
    }
    Ok(())
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeIndexStoreError> {
    i64::try_from(value)
        .map_err(|_| KnowledgeIndexStoreError::Internal(format!("{field} is out of i64 range")))
}

fn from_u64(field: &str, value: i64) -> Result<u64, KnowledgeIndexStoreError> {
    u64::try_from(value)
        .map_err(|_| KnowledgeIndexStoreError::Internal(format!("{field} is out of u64 range")))
}

fn now_rfc3339() -> Result<String, KnowledgeIndexStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeIndexStoreError::Internal(error.to_string()))
}

fn id_error(error: crate::id::KnowledgeIdGeneratorError) -> KnowledgeIndexStoreError {
    KnowledgeIndexStoreError::Internal(error.to_string())
}

fn sqlx_error(error: sqlx::Error) -> KnowledgeIndexStoreError {
    KnowledgeIndexStoreError::Internal(error.to_string())
}
