use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_embedding_store::{
    ChunkEmbeddingUpsertRequest, KnowledgeEmbeddingStore, KnowledgeEmbeddingStoreError,
};
use sdkwork_knowledgebase_agent_provider::serialize_embedding_vector;
use sqlx::{Any, AnyPool, QueryBuilder, Row, Transaction};
use std::sync::Arc;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::{push_sql_timestamp_bind, SqlTimestampDialect};
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};
use crate::postgres_pgvector_retrieval::format_pgvector_literal;

const ACTIVE_STATUS: i64 = 1;
const INITIAL_VERSION: i64 = 0;
const EMBEDDING_UPSERT_BATCH_SIZE: usize = 32;
const MAX_EMBEDDING_DIMENSION: usize = 16_384;
const DEFAULT_PROVIDER_ID: &str = "provider.model.sdkwork-cloudrouter-router";
const DEFAULT_EMBEDDING_MODEL: &str = "openai/text-embedding-3-small";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PostgresKnowledgeEmbeddingStoreError {
    #[error("knowledge embedding store internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeEmbeddingStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    database_engine: DatabaseEngine,
}

#[derive(Debug, Clone)]
struct PreparedEmbeddingUpsert {
    id: i64,
    uuid: String,
    tenant_id: i64,
    organization_id: i64,
    index_id: i64,
    chunk_id: i64,
    embedding_hash: String,
    vector_ref: String,
    vector_json: String,
    pgvector_literal: String,
    dimension: i64,
    provider_id: String,
    model: String,
    now: String,
}

impl PostgresKnowledgeEmbeddingStore {
    pub fn new(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        database_engine: DatabaseEngine,
    ) -> Self {
        Self::with_id_generator(
            pool,
            tenant_id,
            organization_id,
            default_knowledge_id_generator(),
            database_engine,
        )
    }

    pub fn with_id_generator(
        pool: AnyPool,
        tenant_id: u64,
        organization_id: u64,
        id_generator: Arc<dyn KnowledgeIdGenerator>,
        database_engine: DatabaseEngine,
    ) -> Self {
        Self {
            pool,
            tenant_id,
            organization_id,
            id_generator,
            database_engine,
        }
    }

    pub async fn upsert_chunk_embedding(
        &self,
        request: ChunkEmbeddingUpsertRequest,
    ) -> Result<(), PostgresKnowledgeEmbeddingStoreError> {
        self.upsert_chunk_embeddings_batch(std::slice::from_ref(&request))
            .await
    }

    pub async fn upsert_chunk_embeddings_batch(
        &self,
        requests: &[ChunkEmbeddingUpsertRequest],
    ) -> Result<(), PostgresKnowledgeEmbeddingStoreError> {
        if requests.is_empty() {
            return Ok(());
        }

        let timestamp_dialect = SqlTimestampDialect::from_database_engine(self.database_engine);
        let mut tx = self.pool.begin().await.map_err(sqlx_error)?;
        for request_batch in requests.chunks(EMBEDDING_UPSERT_BATCH_SIZE) {
            let prepared = request_batch
                .iter()
                .map(|request| self.prepare_embedding_upsert(request))
                .collect::<Result<Vec<_>, _>>()?;
            bulk_upsert_embeddings_postgres(&mut tx, timestamp_dialect, &prepared).await?;
        }
        tx.commit().await.map_err(sqlx_error)?;
        Ok(())
    }

    fn prepare_embedding_upsert(
        &self,
        request: &ChunkEmbeddingUpsertRequest,
    ) -> Result<PreparedEmbeddingUpsert, PostgresKnowledgeEmbeddingStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;
        if request.vector.is_empty() || request.vector.len() > MAX_EMBEDDING_DIMENSION {
            return Err(PostgresKnowledgeEmbeddingStoreError::Internal(format!(
                "embedding dimension must be between 1 and {MAX_EMBEDDING_DIMENSION}"
            )));
        }
        if request.vector.iter().any(|value| !value.is_finite()) {
            return Err(PostgresKnowledgeEmbeddingStoreError::Internal(
                "embedding vector contains a non-finite value".to_string(),
            ));
        }

        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let index_id = to_i64("index_id", request.index_id)?;
        let chunk_id = to_i64("chunk_id", request.chunk_id)?;
        let dimension = i64::try_from(request.vector.len()).map_err(|_| {
            PostgresKnowledgeEmbeddingStoreError::Internal(
                "embedding dimension overflow".to_string(),
            )
        })?;
        let vector_json = serialize_embedding_vector(&request.vector)
            .map_err(PostgresKnowledgeEmbeddingStoreError::Internal)?;
        let provider_id = request
            .provider_id
            .clone()
            .unwrap_or_else(|| DEFAULT_PROVIDER_ID.to_string());
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.to_string());

        Ok(PreparedEmbeddingUpsert {
            id: next_i64_id(&self.id_generator).map_err(id_error)?,
            uuid: Uuid::new_v4().to_string(),
            tenant_id,
            organization_id,
            index_id,
            chunk_id,
            embedding_hash: format!("sha256:chunk:{chunk_id}:index:{index_id}"),
            vector_ref: format!("inline://vector_json/{chunk_id}"),
            vector_json,
            pgvector_literal: format_pgvector_literal(&request.vector),
            dimension,
            provider_id,
            model,
            now: now_rfc3339()?,
        })
    }

    pub async fn load_chunk_content(
        &self,
        chunk_id: u64,
    ) -> Result<Option<String>, PostgresKnowledgeEmbeddingStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let chunk_id = to_i64("chunk_id", chunk_id)?;
        let content = sqlx::query_scalar::<_, String>(
            r#"
            SELECT content_text
            FROM kb_chunk
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(chunk_id)
        .bind(ACTIVE_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error)?;

        Ok(content)
    }

    pub async fn list_active_chunk_id_content_page(
        &self,
        space_id: u64,
        after_chunk_id: u64,
        limit: u32,
    ) -> Result<Vec<(u64, String)>, PostgresKnowledgeEmbeddingStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id = to_i64("space_id", space_id)?;
        let after_chunk_id = to_i64("after_chunk_id", after_chunk_id)?;
        let limit = i64::from(limit.clamp(1, 512));
        let rows = sqlx::query(
            r#"
            SELECT id, content_text
            FROM kb_chunk
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND status = $4
              AND id > $5
            ORDER BY id ASC
            LIMIT $6
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(ACTIVE_STATUS)
        .bind(after_chunk_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                let chunk_id = row.try_get::<i64, _>("id").map_err(sqlx_error)?;
                let content = row
                    .try_get::<String, _>("content_text")
                    .map_err(sqlx_error)?;
                let chunk_id = u64::try_from(chunk_id).map_err(|_| {
                    PostgresKnowledgeEmbeddingStoreError::Internal("chunk id overflow".to_string())
                })?;
                Ok((chunk_id, content))
            })
            .collect()
    }
}

async fn bulk_upsert_embeddings_postgres(
    transaction: &mut Transaction<'_, Any>,
    timestamp_dialect: SqlTimestampDialect,
    batch: &[PreparedEmbeddingUpsert],
) -> Result<(), PostgresKnowledgeEmbeddingStoreError> {
    let mut builder = QueryBuilder::new(
        r#"
        INSERT INTO kb_embedding (
            id, uuid, tenant_id, organization_id, index_id, chunk_id, embedding_hash, vector_ref, vector_json,
            embedding_vector, dimension, provider_id, model, metadata, status, created_at, updated_at, version
        )
        "#,
    );
    builder.push_values(batch, |mut row, item| {
        row.push_bind(item.id)
            .push_bind(item.uuid.as_str())
            .push_bind(item.tenant_id)
            .push_bind(item.organization_id)
            .push_bind(item.index_id)
            .push_bind(item.chunk_id)
            .push_bind(item.embedding_hash.as_str())
            .push_bind(item.vector_ref.as_str())
            .push_bind(item.vector_json.as_str());
        row.push("CAST(");
        row.push_bind_unseparated(item.pgvector_literal.as_str());
        row.push_unseparated(" AS vector)");
        row.push_bind(item.dimension)
            .push_bind(item.provider_id.as_str())
            .push_bind(item.model.as_str())
            .push("NULL")
            .push_bind(ACTIVE_STATUS);
        push_sql_timestamp_bind(&mut row, timestamp_dialect, item.now.as_str());
        push_sql_timestamp_bind(&mut row, timestamp_dialect, item.now.as_str());
        row.push_bind(INITIAL_VERSION);
    });
    builder.push(
        r#"
        ON CONFLICT (tenant_id, organization_id, index_id, chunk_id) DO UPDATE SET
            embedding_hash = excluded.embedding_hash,
            vector_ref = excluded.vector_ref,
            vector_json = excluded.vector_json,
            embedding_vector = excluded.embedding_vector,
            dimension = excluded.dimension,
            provider_id = excluded.provider_id,
            model = excluded.model,
            status = excluded.status,
            updated_at = excluded.updated_at,
            version = kb_embedding.version + 1
        "#,
    );
    builder
        .build()
        .execute(&mut **transaction)
        .await
        .map_err(sqlx_error)?;
    Ok(())
}

fn ensure_tenant_scope(
    expected: u64,
    actual: u64,
) -> Result<(), PostgresKnowledgeEmbeddingStoreError> {
    if expected != actual {
        return Err(PostgresKnowledgeEmbeddingStoreError::Internal(
            "tenant_id is out of store scope".to_string(),
        ));
    }
    Ok(())
}

fn to_i64(field: &str, value: u64) -> Result<i64, PostgresKnowledgeEmbeddingStoreError> {
    i64::try_from(value).map_err(|_| {
        PostgresKnowledgeEmbeddingStoreError::Internal(format!("{field} is out of i64 range"))
    })
}

fn now_rfc3339() -> Result<String, PostgresKnowledgeEmbeddingStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| PostgresKnowledgeEmbeddingStoreError::Internal(error.to_string()))
}

fn id_error(error: crate::id::KnowledgeIdGeneratorError) -> PostgresKnowledgeEmbeddingStoreError {
    PostgresKnowledgeEmbeddingStoreError::Internal(error.to_string())
}

fn sqlx_error(error: sqlx::Error) -> PostgresKnowledgeEmbeddingStoreError {
    PostgresKnowledgeEmbeddingStoreError::Internal(error.to_string())
}

#[async_trait]
impl KnowledgeEmbeddingStore for PostgresKnowledgeEmbeddingStore {
    async fn upsert_chunk_embedding(
        &self,
        request: ChunkEmbeddingUpsertRequest,
    ) -> Result<(), KnowledgeEmbeddingStoreError> {
        PostgresKnowledgeEmbeddingStore::upsert_chunk_embedding(self, request)
            .await
            .map_err(|error| KnowledgeEmbeddingStoreError::Internal(error.to_string()))
    }

    async fn upsert_chunk_embeddings_batch(
        &self,
        requests: &[ChunkEmbeddingUpsertRequest],
    ) -> Result<(), KnowledgeEmbeddingStoreError> {
        PostgresKnowledgeEmbeddingStore::upsert_chunk_embeddings_batch(self, requests)
            .await
            .map_err(|error| KnowledgeEmbeddingStoreError::Internal(error.to_string()))
    }

    async fn load_chunk_content(
        &self,
        chunk_id: u64,
    ) -> Result<Option<String>, KnowledgeEmbeddingStoreError> {
        PostgresKnowledgeEmbeddingStore::load_chunk_content(self, chunk_id)
            .await
            .map_err(|error| KnowledgeEmbeddingStoreError::Internal(error.to_string()))
    }

    async fn list_active_chunk_id_content_page(
        &self,
        space_id: u64,
        after_chunk_id: u64,
        limit: u32,
    ) -> Result<Vec<(u64, String)>, KnowledgeEmbeddingStoreError> {
        PostgresKnowledgeEmbeddingStore::list_active_chunk_id_content_page(
            self,
            space_id,
            after_chunk_id,
            limit,
        )
        .await
        .map_err(|error| KnowledgeEmbeddingStoreError::Internal(error.to_string()))
    }
}
