use sdkwork_database_config::DatabaseEngine;
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeRetrievalProfile, KnowledgeRetrievalProfileRequest,
};
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeRetrievalProfileStoreError {
    #[error("missing retrieval profile: {0}")]
    NotFound(String),
    #[error("knowledge retrieval profile store internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeRetrievalProfileStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeRetrievalProfileStore {
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

    pub async fn create_profile(
        &self,
        request: KnowledgeRetrievalProfileRequest,
    ) -> Result<KnowledgeRetrievalProfile, KnowledgeRetrievalProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;
        if is_blank(Some(request.name.as_str())) {
            return Err(KnowledgeRetrievalProfileStoreError::Internal(
                "name is required".to_string(),
            ));
        }

        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let top_k = i64::from(request.top_k);
        let rerank_enabled = i64::from(request.rerank_enabled);
        let context_budget_tokens = i64::from(request.context_budget_tokens);
        let now = now_rfc3339()?;

        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$12");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let query = format!(
            r#"
            INSERT INTO kb_retrieval_profile (
                id, uuid, tenant_id, organization_id, name, strategy, top_k, min_score, rerank_enabled,
                context_budget_tokens, status, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, {created_at_expr}, {updated_at_expr}, $14)
            RETURNING id, tenant_id, name, strategy, top_k, min_score, rerank_enabled,
                      context_budget_tokens, status
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(request.name)
            .bind(request.strategy)
            .bind(top_k)
            .bind(request.min_score)
            .bind(rerank_enabled)
            .bind(context_budget_tokens)
            .bind(profile_status_code(&request.status))
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error)?;

        profile_from_row(&row)
    }

    pub async fn get_profile(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeRetrievalProfile, KnowledgeRetrievalProfileStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id = to_i64("profile_id", profile_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, tenant_id, name, strategy, top_k, min_score, rerank_enabled,
                   context_budget_tokens, status
            FROM kb_retrieval_profile
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(profile_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?
        .ok_or_else(|| {
            KnowledgeRetrievalProfileStoreError::NotFound(format!(
                "missing retrieval profile: {profile_id}"
            ))
        })?;

        profile_from_row(&row)
    }

    pub async fn update_profile(
        &self,
        profile_id: u64,
        request: KnowledgeRetrievalProfileRequest,
    ) -> Result<KnowledgeRetrievalProfile, KnowledgeRetrievalProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id = to_i64("profile_id", profile_id)?;
        let top_k = i64::from(request.top_k);
        let rerank_enabled = i64::from(request.rerank_enabled);
        let context_budget_tokens = i64::from(request.context_budget_tokens);
        let now = now_rfc3339()?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$8");
        let query = format!(
            r#"
            UPDATE kb_retrieval_profile
            SET name = $1, strategy = $2, top_k = $3, min_score = $4, rerank_enabled = $5,
                context_budget_tokens = $6, status = $7, updated_at = {updated_at_expr}, version = version + 1
            WHERE tenant_id = $9 AND organization_id = $10 AND id = $11 AND status = $12
            RETURNING id, tenant_id, name, strategy, top_k, min_score, rerank_enabled,
                      context_budget_tokens, status
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(request.name)
            .bind(request.strategy)
            .bind(top_k)
            .bind(request.min_score)
            .bind(rerank_enabled)
            .bind(context_budget_tokens)
            .bind(profile_status_code(&request.status))
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(profile_id)
            .bind(ACTIVE_STATUS)
            .fetch_optional(&self.pool)
            .await
            .map_err(sqlx_error)?
            .ok_or_else(|| {
                KnowledgeRetrievalProfileStoreError::Internal(format!(
                    "missing retrieval profile: {profile_id}"
                ))
            })?;

        profile_from_row(&row)
    }
}

fn profile_from_row(
    row: &AnyRow,
) -> Result<KnowledgeRetrievalProfile, KnowledgeRetrievalProfileStoreError> {
    let top_k: i64 = row.try_get("top_k").map_err(sqlx_error)?;
    let rerank_enabled: i64 = row.try_get("rerank_enabled").map_err(sqlx_error)?;
    let context_budget_tokens: i64 = row.try_get("context_budget_tokens").map_err(sqlx_error)?;
    Ok(KnowledgeRetrievalProfile {
        retrieval_profile_id: from_u64("id", row.try_get("id").map_err(sqlx_error)?)?,
        tenant_id: from_u64("tenant_id", row.try_get("tenant_id").map_err(sqlx_error)?)?,
        name: row.try_get("name").map_err(sqlx_error)?,
        strategy: row.try_get("strategy").map_err(sqlx_error)?,
        top_k: u32::try_from(top_k).map_err(|_| {
            KnowledgeRetrievalProfileStoreError::Internal("top_k is out of range".to_string())
        })?,
        min_score: row.try_get("min_score").map_err(sqlx_error)?,
        rerank_enabled: rerank_enabled != 0,
        context_budget_tokens: u32::try_from(context_budget_tokens).map_err(|_| {
            KnowledgeRetrievalProfileStoreError::Internal(
                "context_budget_tokens is out of range".to_string(),
            )
        })?,
        status: profile_status_name(row.try_get("status").map_err(sqlx_error)?)?,
    })
}

fn profile_status_code(value: &str) -> i64 {
    match value.trim().to_ascii_lowercase().as_str() {
        "active" => ACTIVE_STATUS,
        _ => 0,
    }
}

fn profile_status_name(code: i64) -> Result<String, KnowledgeRetrievalProfileStoreError> {
    match code {
        ACTIVE_STATUS => Ok("active".to_string()),
        0 => Ok("disabled".to_string()),
        value => Err(KnowledgeRetrievalProfileStoreError::Internal(format!(
            "unsupported retrieval profile status code: {value}"
        ))),
    }
}

fn ensure_tenant_scope(
    configured_tenant_id: u64,
    request_tenant_id: u64,
) -> Result<(), KnowledgeRetrievalProfileStoreError> {
    if configured_tenant_id != request_tenant_id {
        return Err(KnowledgeRetrievalProfileStoreError::Internal(
            "tenant_id does not match configured repository tenant".to_string(),
        ));
    }
    Ok(())
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeRetrievalProfileStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeRetrievalProfileStoreError::Internal(format!("{field} is out of i64 range"))
    })
}

fn from_u64(field: &str, value: i64) -> Result<u64, KnowledgeRetrievalProfileStoreError> {
    u64::try_from(value).map_err(|_| {
        KnowledgeRetrievalProfileStoreError::Internal(format!("{field} is out of u64 range"))
    })
}

fn now_rfc3339() -> Result<String, KnowledgeRetrievalProfileStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeRetrievalProfileStoreError::Internal(error.to_string()))
}

fn id_error(error: crate::id::KnowledgeIdGeneratorError) -> KnowledgeRetrievalProfileStoreError {
    KnowledgeRetrievalProfileStoreError::Internal(error.to_string())
}

fn sqlx_error(error: sqlx::Error) -> KnowledgeRetrievalProfileStoreError {
    KnowledgeRetrievalProfileStoreError::Internal(error.to_string())
}
