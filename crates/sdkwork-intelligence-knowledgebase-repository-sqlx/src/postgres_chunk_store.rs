use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_chunk_store::{
    CreateKnowledgeChunkRecord, KnowledgeChunkStore, KnowledgeChunkStoreError,
};
use sqlx::{AnyPool, Row};
use std::sync::Arc;

use crate::chunk_transaction::replace_version_chunks_with_pool;
use crate::id::{default_knowledge_id_generator, KnowledgeIdGenerator};
use crate::keyword_search::KeywordSearchBackend;

/// Safety cap for in-process document content assembly (PAGINATION_SPEC store-level bound).

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeChunkStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    #[allow(dead_code)] // 保留 keyword backend 选择机制（当前仅 PostgresTsVector）
    keyword_backend: KeywordSearchBackend,
    database_engine: DatabaseEngine,
}

impl PostgresKnowledgeChunkStore {
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
            database_engine: DatabaseEngine::Postgres,
        }
    }

    /// Selects the SQL timestamp dialect used by chunk transactions so PostgreSQL sessions
    /// bind timestamps through the typed CAST path instead of implicit text conversion.
    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.database_engine = database_engine;
        self
    }
}

#[async_trait]
impl KnowledgeChunkStore for PostgresKnowledgeChunkStore {
    async fn replace_version_chunks(
        &self,
        document_version_id: u64,
        chunks: Vec<CreateKnowledgeChunkRecord>,
    ) -> Result<usize, KnowledgeChunkStoreError> {
        replace_version_chunks_with_pool(
            &self.pool,
            self.tenant_id,
            self.organization_id,
            &self.id_generator,
            crate::db::sql_timestamp::SqlTimestampDialect::from_database_engine(
                self.database_engine,
            ),
            document_version_id,
            chunks,
        )
        .await
    }

    async fn list_chunk_ids_for_document_version(
        &self,
        document_version_id: u64,
    ) -> Result<Vec<u64>, KnowledgeChunkStoreError> {
        let tenant_id = chunk_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = chunk_to_i64("organization_id", self.organization_id)?;
        let version_id = chunk_to_i64("document_version_id", document_version_id)?;
        const ACTIVE_STATUS: i64 = 1;
        const CHUNK_PAGE_SIZE: i64 = 512;
        let mut chunk_ids = Vec::new();
        // Keyset loop by chunk_index so documents with more chunks than any single-page
        // limit are never silently truncated (a document version is unbounded). The next
        // bound is the last observed chunk_index, never an assumed page offset.
        let mut after_chunk_index = -1i64;
        loop {
            let rows = sqlx::query(
                r#"
                SELECT id, chunk_index
                FROM kb_chunk
                WHERE tenant_id = $1
                  AND organization_id = $2
                  AND document_version_id = $3
                  AND status = $4
                  AND chunk_index > $5
                ORDER BY chunk_index ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(version_id)
            .bind(ACTIVE_STATUS)
            .bind(after_chunk_index)
            .bind(CHUNK_PAGE_SIZE)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
            if rows.is_empty() {
                break;
            }
            let page_len = rows.len() as i64;
            after_chunk_index = rows
                .last()
                .and_then(|row| row.try_get::<i64, _>("chunk_index").ok())
                .unwrap_or(after_chunk_index + page_len);
            chunk_ids.extend(
                rows.iter()
                    .filter_map(|row| row.try_get::<i64, _>("id").ok()),
            );
            if page_len < CHUNK_PAGE_SIZE {
                break;
            }
        }

        chunk_ids
            .into_iter()
            .map(|id| {
                u64::try_from(id).map_err(|_| {
                    KnowledgeChunkStoreError::Internal("chunk id exceeds u64 range".to_string())
                })
            })
            .collect()
    }

    async fn list_chunk_texts_for_document_version(
        &self,
        document_version_id: u64,
    ) -> Result<Vec<String>, KnowledgeChunkStoreError> {
        let tenant_id = chunk_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = chunk_to_i64("organization_id", self.organization_id)?;
        let version_id = chunk_to_i64("document_version_id", document_version_id)?;
        const ACTIVE_STATUS: i64 = 1;
        const CHUNK_PAGE_SIZE: i64 = 512;
        let mut chunk_texts = Vec::new();
        // Keyset loop by chunk_index so a document version is never silently truncated.
        let mut after_chunk_index = -1i64;
        loop {
            let rows = sqlx::query(
                r#"
                SELECT content_text, chunk_index
                FROM kb_chunk
                WHERE tenant_id = $1
                  AND organization_id = $2
                  AND document_version_id = $3
                  AND status = $4
                  AND chunk_index > $5
                ORDER BY chunk_index ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(version_id)
            .bind(ACTIVE_STATUS)
            .bind(after_chunk_index)
            .bind(CHUNK_PAGE_SIZE)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
            if rows.is_empty() {
                break;
            }
            let page_len = rows.len() as i64;
            after_chunk_index = rows
                .last()
                .and_then(|row| row.try_get::<i64, _>("chunk_index").ok())
                .unwrap_or(after_chunk_index + page_len);
            chunk_texts.extend(
                rows.iter()
                    .filter_map(|row| row.try_get::<String, _>("content_text").ok()),
            );
            if page_len < CHUNK_PAGE_SIZE {
                break;
            }
        }

        Ok(chunk_texts)
    }

    async fn list_chunk_id_content_for_document_version(
        &self,
        document_version_id: u64,
    ) -> Result<Vec<(u64, String)>, KnowledgeChunkStoreError> {
        let tenant_id = chunk_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = chunk_to_i64("organization_id", self.organization_id)?;
        let version_id = chunk_to_i64("document_version_id", document_version_id)?;
        const ACTIVE_STATUS: i64 = 1;
        const CHUNK_PAGE_SIZE: i64 = 512;
        let mut chunk_rows = Vec::new();
        // Keyset loop by chunk_index so a document version is never silently truncated.
        let mut after_chunk_index = -1i64;
        loop {
            let rows = sqlx::query(
                r#"
                SELECT id, content_text, chunk_index
                FROM kb_chunk
                WHERE tenant_id = $1
                  AND organization_id = $2
                  AND document_version_id = $3
                  AND status = $4
                  AND chunk_index > $5
                ORDER BY chunk_index ASC
                LIMIT $6
                "#,
            )
            .bind(tenant_id)
            .bind(organization_id)
            .bind(version_id)
            .bind(ACTIVE_STATUS)
            .bind(after_chunk_index)
            .bind(CHUNK_PAGE_SIZE)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
            if rows.is_empty() {
                break;
            }
            let page_len = rows.len() as i64;
            after_chunk_index = rows
                .last()
                .and_then(|row| row.try_get::<i64, _>("chunk_index").ok())
                .unwrap_or(after_chunk_index + page_len);
            chunk_rows.extend(rows);
            if page_len < CHUNK_PAGE_SIZE {
                break;
            }
        }

        chunk_rows
            .into_iter()
            .map(|row| {
                let chunk_id = row
                    .try_get::<i64, _>("id")
                    .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
                let content = row
                    .try_get::<String, _>("content_text")
                    .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
                let chunk_id = u64::try_from(chunk_id).map_err(|_| {
                    KnowledgeChunkStoreError::Internal("chunk id exceeds u64 range".to_string())
                })?;
                Ok((chunk_id, content))
            })
            .collect()
    }
}

fn chunk_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeChunkStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeChunkStoreError::InvalidRecord(format!("{field} exceeds i64 integer range"))
    })
}
