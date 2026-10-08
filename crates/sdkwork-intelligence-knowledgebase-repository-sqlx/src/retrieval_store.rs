use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_backend::{
    KnowledgeChunkSearchHit, KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend,
    KnowledgeRetrievalBackendError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_trace_store::{
    CreateKnowledgeRetrievalHitRecord, CreateKnowledgeRetrievalTraceRecord,
    KnowledgeRetrievalTraceHitRecord, KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStore,
    KnowledgeRetrievalTraceStoreError,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeRetrievalMethod;
use sqlx::{any::AnyRow, AnyPool, QueryBuilder, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::binding_scope_filters::push_binding_scope_filters;
use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};
use crate::keyword_search::KeywordSearchBackend;

const ACTIVE_STATUS: i64 = 1;
const SUCCEEDED_STATUS: i64 = 1;
const INITIAL_VERSION: i64 = 0;
/// Maximum hits returned per retrieval trace (bounded to avoid OOM).
const MAX_RETRIEVAL_TRACE_HITS: i64 = 256;
/// This store serves keyword/embedding-indexed search only. Production servers
/// always wire `PgVectorLayeredRetrievalBackend`, which routes Vector/Hybrid
/// scoring to the pgvector ANN index (`kb_embedding.embedding_vector`) and uses
/// this store only for keyword search, so this store is a degraded mode for
/// runtimes without a PgPool (tests and tooling).

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeChunkRetrievalStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    #[allow(dead_code)] // 保留 keyword backend 选择机制（当前仅 PostgresTsVector）
    keyword_backend: KeywordSearchBackend,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeChunkRetrievalStore {
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
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }
}

impl PostgresKnowledgeChunkRetrievalStore {
    pub async fn list_trace_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<KnowledgeRetrievalTraceRecord>, KnowledgeRetrievalTraceStoreError> {
        let tenant_id = trace_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = trace_to_i64("organization_id", self.organization_id)?;
        let limit = i64::from(limit.clamp(1, 200));
        let rows = sqlx::query(
            r#"
            SELECT
                tenant_id,
                id AS retrieval_trace_id,
                actor_id,
                retrieval_profile_id,
                query_text_redacted,
                latency_ms,
                result_count,
                status
            FROM kb_retrieval_trace
            WHERE tenant_id = $1 AND organization_id = $2
            ORDER BY id DESC
            LIMIT $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(trace_sqlx_error)?;

        rows.into_iter().map(trace_record_from_row).collect()
    }

    pub async fn list_trace_summaries_page(
        &self,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<
        (Vec<KnowledgeRetrievalTraceRecord>, Option<String>, bool),
        KnowledgeRetrievalTraceStoreError,
    > {
        let tenant_id = trace_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = trace_to_i64("organization_id", self.organization_id)?;
        let cursor = cursor
            .map(|value| trace_to_i64("cursor", value))
            .transpose()?;
        let page_size = page_size.clamp(1, 200) as usize;
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, id AS retrieval_trace_id, actor_id, retrieval_profile_id,
                   query_text_redacted, latency_ms, result_count, status
            FROM kb_retrieval_trace
            WHERE tenant_id = $1
              AND organization_id = $2
              AND ($3 IS NULL OR id > $3)
            ORDER BY id ASC
            LIMIT $4
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(cursor)
        .bind((page_size + 1) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(trace_sqlx_error)?;

        let has_more = rows.len() > page_size;
        let items = rows
            .into_iter()
            .take(page_size)
            .map(trace_record_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.retrieval_trace_id.to_string()))
            .flatten();
        Ok((items, next_cursor, has_more))
    }
}

#[async_trait]
impl KnowledgeRetrievalBackend for PostgresKnowledgeChunkRetrievalStore {
    async fn search_chunks(
        &self,
        request: KnowledgeChunkSearchRequest,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        if request.tenant_id != self.tenant_id {
            return Err(KnowledgeRetrievalBackendError::TenantMismatch);
        }

        match request.method {
            KnowledgeRetrievalMethod::Vector => {
                self.search_embedding_indexed_chunks(request, TermMatchOperator::Any)
                    .await
            }
            KnowledgeRetrievalMethod::Hybrid => {
                self.search_chunks_with_term_operator(request, TermMatchOperator::Any)
                    .await
            }
            KnowledgeRetrievalMethod::Graph
            | KnowledgeRetrievalMethod::External
            | KnowledgeRetrievalMethod::Structured
            | KnowledgeRetrievalMethod::LlmRerank => {
                return Err(KnowledgeRetrievalBackendError::UnsupportedMethod(
                    request.method,
                ));
            }
            KnowledgeRetrievalMethod::Exact => {
                self.search_chunks_with_term_operator(request, TermMatchOperator::All)
                    .await
            }
            _ => {
                self.search_chunks_with_term_operator(request, TermMatchOperator::Any)
                    .await
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TermMatchOperator {
    Any,
    All,
}

impl PostgresKnowledgeChunkRetrievalStore {
    async fn search_chunks_with_term_operator(
        &self,
        request: KnowledgeChunkSearchRequest,
        term_operator: TermMatchOperator,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        let tenant_id = backend_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = backend_to_i64("organization_id", self.organization_id)?;
        let space_id = backend_to_i64("space_id", request.binding.space_id)?;
        let top_k = i64::from(request.top_k.clamp(1, 64));
        let query_terms = normalized_query_terms(&request.query);

        if query_terms.is_empty() {
            return Ok(vec![]);
        }

        let fts_match = build_keyword_match_expression(&query_terms, term_operator);

        let mut query = QueryBuilder::new(
            r#"
            SELECT
                c.id AS chunk_id,
                c.document_id,
                c.document_version_id,
                c.space_id,
                d.title,
                c.content_text,
                c.token_count,
                c.locator #>> '{}' AS locator,
                "kb://documents/" || c.document_id AS source_uri,
            "#,
        );
        push_keyword_score_expression(&mut query, &query_terms, &fts_match);
        query.push(
            r#"
                AS score
            FROM kb_chunk c
            JOIN kb_document d
              ON d.tenant_id = c.tenant_id
             AND d.organization_id = c.organization_id
             AND d.id = c.document_id
             AND d.status =
            "#,
        );
        query.push_bind(ACTIVE_STATUS);
        query.push(
            r#"
            WHERE c.tenant_id =
            "#,
        );
        query.push_bind(tenant_id);
        query.push(" AND c.organization_id = ");
        query.push_bind(organization_id);
        query.push(" AND c.space_id = ");
        query.push_bind(space_id);
        query.push(" AND c.status = ");
        query.push_bind(ACTIVE_STATUS);
        push_binding_scope_filters(
            &mut query,
            tenant_id,
            organization_id,
            space_id,
            &request.binding,
        )?;
        query.push(" AND (");
        push_keyword_or_title_filter(&mut query, &query_terms, term_operator, &fts_match);
        query.push(") ORDER BY score DESC, c.id ASC LIMIT ");
        query.push_bind(top_k);

        let rows = query
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(backend_sqlx_error)?;

        rows.into_iter()
            .map(|row| chunk_hit_from_row(row, request.method, request.binding.min_score))
            .filter_map(Result::transpose)
            .collect()
    }

    async fn search_embedding_indexed_chunks(
        &self,
        request: KnowledgeChunkSearchRequest,
        term_operator: TermMatchOperator,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        let tenant_id = backend_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = backend_to_i64("organization_id", self.organization_id)?;
        let space_id = backend_to_i64("space_id", request.binding.space_id)?;
        let top_k = i64::from(request.top_k.clamp(1, 64));
        let query_terms = normalized_query_terms(&request.query);

        if query_terms.is_empty() {
            return Ok(vec![]);
        }

        let fts_match = build_keyword_match_expression(&query_terms, term_operator);

        let mut query = QueryBuilder::new(
            r#"
            SELECT
                c.id AS chunk_id,
                c.document_id,
                c.document_version_id,
                c.space_id,
                d.title,
                c.content_text,
                c.token_count,
                c.locator #>> '{}' AS locator,
                "kb://documents/" || c.document_id AS source_uri,
            "#,
        );
        push_keyword_score_expression(&mut query, &query_terms, &fts_match);
        query.push(
            r#"
                AS score
            FROM kb_chunk c
            JOIN kb_document d
              ON d.tenant_id = c.tenant_id
             AND d.organization_id = c.organization_id
             AND d.id = c.document_id
             AND d.status =
            "#,
        );
        query.push_bind(ACTIVE_STATUS);
        query.push(
            r#"
            INNER JOIN kb_embedding e
              ON e.tenant_id = c.tenant_id
             AND e.organization_id = c.organization_id
             AND e.chunk_id = c.id
             AND e.status =
            "#,
        );
        query.push_bind(ACTIVE_STATUS);
        query.push(
            r#"
            WHERE c.tenant_id =
            "#,
        );
        query.push_bind(tenant_id);
        query.push(" AND c.organization_id = ");
        query.push_bind(organization_id);
        query.push(" AND c.space_id = ");
        query.push_bind(space_id);
        query.push(" AND c.status = ");
        query.push_bind(ACTIVE_STATUS);
        push_binding_scope_filters(
            &mut query,
            tenant_id,
            organization_id,
            space_id,
            &request.binding,
        )?;
        query.push(" AND (");
        push_keyword_or_title_filter(&mut query, &query_terms, term_operator, &fts_match);
        query.push(") ORDER BY score DESC, c.id ASC LIMIT ");
        query.push_bind(top_k);

        let rows = query
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(backend_sqlx_error)?;

        rows.into_iter()
            .map(|row| chunk_hit_from_row(row, request.method, request.binding.min_score))
            .filter_map(Result::transpose)
            .collect()
    }
}

#[async_trait]
impl KnowledgeRetrievalTraceStore for PostgresKnowledgeChunkRetrievalStore {
    async fn create_trace(
        &self,
        record: CreateKnowledgeRetrievalTraceRecord,
    ) -> Result<u64, KnowledgeRetrievalTraceStoreError> {
        if record.tenant_id != self.tenant_id {
            return Err(KnowledgeRetrievalTraceStoreError::Internal(
                "trace tenant_id must match store tenant scope".to_string(),
            ));
        }

        let id = next_i64_id(&self.id_generator).map_err(trace_id_error)?;
        let tenant_id = trace_to_i64("tenant_id", record.tenant_id)?;
        let organization_id = trace_to_i64("organization_id", self.organization_id)?;
        let actor_id = record
            .actor_id
            .map(|value| trace_to_i64("actor_id", value))
            .transpose()?;
        let retrieval_profile_id = record
            .retrieval_profile_id
            .map(|value| trace_to_i64("retrieval_profile_id", value))
            .transpose()?;
        let result_count = i64::from(record.result_count);
        let latency_ms = record.latency_ms.map(|value| value as i64);
        let status = trace_status_code(&record.status)?;
        let now = now_rfc3339().map_err(KnowledgeRetrievalTraceStoreError::Internal)?;
        let request_payload_expr = self.timestamp_dialect.sql_json_expr("$9");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$14");

        let query = format!(
            r#"
            INSERT INTO kb_retrieval_trace (
                id,
                uuid,
                tenant_id,
                organization_id,
                actor_id,
                retrieval_profile_id,
                query_hash,
                query_text_redacted,
                request_payload,
                latency_ms,
                result_count,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, {request_payload_expr}, $10, $11, $12, {created_at_expr}, {updated_at_expr}, $15)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(actor_id)
            .bind(retrieval_profile_id)
            .bind(record.query_hash_sha256_hex)
            .bind(record.query_text_redacted)
            .bind(record.request_payload_json)
            .bind(latency_ms)
            .bind(result_count)
            .bind(status)
            .bind(now.clone())
            .bind(now)
            .bind(INITIAL_VERSION)
            .execute(&self.pool)
            .await
            .map_err(trace_sqlx_error)?;

        u64::try_from(id).map_err(|_| {
            KnowledgeRetrievalTraceStoreError::Internal(
                "generated retrieval trace id is negative".to_string(),
            )
        })
    }

    async fn create_hits(
        &self,
        records: Vec<CreateKnowledgeRetrievalHitRecord>,
    ) -> Result<(), KnowledgeRetrievalTraceStoreError> {
        let mut transaction = self.pool.begin().await.map_err(trace_sqlx_error)?;
        for record in records {
            if record.tenant_id != self.tenant_id {
                return Err(KnowledgeRetrievalTraceStoreError::Internal(
                    "hit tenant_id must match store tenant scope".to_string(),
                ));
            }

            let id = next_i64_id(&self.id_generator).map_err(trace_id_error)?;
            let tenant_id = trace_to_i64("tenant_id", record.tenant_id)?;
            let organization_id = trace_to_i64("organization_id", self.organization_id)?;
            let retrieval_trace_id = trace_to_i64("retrieval_trace_id", record.retrieval_trace_id)?;
            let chunk_id = trace_to_i64("chunk_id", record.chunk_id)?;
            let document_id = trace_to_i64("document_id", record.document_id)?;
            let document_version_id = record
                .document_version_id
                .map(|value| trace_to_i64("document_version_id", value))
                .transpose()?;
            let result_rank = i64::from(record.result_rank);
            let now = now_rfc3339().map_err(KnowledgeRetrievalTraceStoreError::Internal)?;
            let citation_expr = self.timestamp_dialect.sql_json_expr("$12");
            let metadata_expr = self.timestamp_dialect.sql_json_expr("$13");
            let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$15");
            let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$16");

            let query = format!(
                r#"
                INSERT INTO kb_retrieval_hit (
                    id,
                    uuid,
                    tenant_id,
                    organization_id,
                    retrieval_trace_id,
                    chunk_id,
                    document_id,
                    document_version_id,
                    score,
                    result_rank,
                    match_reason,
                    citation,
                    metadata,
                    status,
                    created_at,
                    updated_at,
                    version
                )
                SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, {citation_expr}, {metadata_expr}, $14, {created_at_expr}, {updated_at_expr}, $17
                WHERE EXISTS (
                    SELECT 1
                    FROM kb_retrieval_trace trace
                    JOIN kb_chunk chunk
                      ON chunk.tenant_id = trace.tenant_id
                     AND chunk.organization_id = trace.organization_id
                     AND chunk.id = $6
                     AND chunk.document_id = $7
                    JOIN kb_document document
                      ON document.tenant_id = chunk.tenant_id
                     AND document.organization_id = chunk.organization_id
                     AND document.id = chunk.document_id
                    WHERE trace.tenant_id = $3
                      AND trace.organization_id = $4
                      AND trace.id = $5
                      AND ($8 IS NULL OR EXISTS (
                          SELECT 1
                          FROM kb_document_version document_version
                          WHERE document_version.tenant_id = trace.tenant_id
                            AND document_version.organization_id = trace.organization_id
                            AND document_version.id = $8
                            AND document_version.document_id = document.id
                      ))
                )
                "#,
            );
            let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(id)
                .bind(Uuid::new_v4().to_string())
                .bind(tenant_id)
                .bind(organization_id)
                .bind(retrieval_trace_id)
                .bind(chunk_id)
                .bind(document_id)
                .bind(document_version_id)
                .bind(record.score)
                .bind(result_rank)
                .bind(record.match_reason)
                .bind(record.citation_json)
                .bind(record.metadata_json)
                .bind(ACTIVE_STATUS)
                .bind(now.clone())
                .bind(now)
                .bind(INITIAL_VERSION)
                .execute(&mut *transaction)
                .await
                .map_err(trace_sqlx_error)?;
            if result.rows_affected() != 1 {
                return Err(KnowledgeRetrievalTraceStoreError::NotFound(
                    record.retrieval_trace_id,
                ));
            }
        }

        transaction.commit().await.map_err(trace_sqlx_error)?;
        Ok(())
    }

    async fn retrieve_trace(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStoreError> {
        if tenant_id != self.tenant_id {
            return Err(KnowledgeRetrievalTraceStoreError::NotFound(
                retrieval_trace_id,
            ));
        }

        let row = sqlx::query(
            r#"
            SELECT
                tenant_id,
                id AS retrieval_trace_id,
                actor_id,
                retrieval_profile_id,
                query_text_redacted,
                latency_ms,
                result_count,
                status
            FROM kb_retrieval_trace
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
            "#,
        )
        .bind(trace_to_i64("tenant_id", tenant_id)?)
        .bind(trace_to_i64("organization_id", self.organization_id)?)
        .bind(trace_to_i64("retrieval_trace_id", retrieval_trace_id)?)
        .fetch_optional(&self.pool)
        .await
        .map_err(trace_sqlx_error)?
        .ok_or(KnowledgeRetrievalTraceStoreError::NotFound(
            retrieval_trace_id,
        ))?;

        trace_record_from_row(row)
    }

    async fn list_trace_hits(
        &self,
        tenant_id: u64,
        retrieval_trace_id: u64,
    ) -> Result<Vec<KnowledgeRetrievalTraceHitRecord>, KnowledgeRetrievalTraceStoreError> {
        if tenant_id != self.tenant_id {
            return Err(KnowledgeRetrievalTraceStoreError::NotFound(
                retrieval_trace_id,
            ));
        }

        let rows = sqlx::query(
            r#"
            SELECT
                h.chunk_id,
                h.document_id,
                h.document_version_id,
                c.space_id,
                d.title,
                c.content_text,
                h.score,
                h.result_rank,
                h.match_reason,
                CAST(h.citation AS TEXT) AS citation,
                c.token_count
            FROM kb_retrieval_hit h
            JOIN kb_chunk c
              ON c.tenant_id = h.tenant_id
             AND c.organization_id = h.organization_id
             AND c.id = h.chunk_id
            JOIN kb_document d
              ON d.tenant_id = h.tenant_id
             AND d.organization_id = h.organization_id
             AND d.id = h.document_id
            WHERE h.tenant_id = $1
              AND h.organization_id = $2
              AND h.retrieval_trace_id = $3
            ORDER BY h.result_rank ASC, h.id ASC
            LIMIT $4
            "#,
        )
        .bind(trace_to_i64("tenant_id", tenant_id)?)
        .bind(trace_to_i64("organization_id", self.organization_id)?)
        .bind(trace_to_i64("retrieval_trace_id", retrieval_trace_id)?)
        .bind(MAX_RETRIEVAL_TRACE_HITS)
        .fetch_all(&self.pool)
        .await
        .map_err(trace_sqlx_error)?;

        rows.into_iter().map(trace_hit_from_row).collect()
    }
}

pub(crate) fn merge_hybrid_hits(
    keyword_hits: Vec<KnowledgeChunkSearchHit>,
    vector_hits: Vec<KnowledgeChunkSearchHit>,
    _dimension: usize,
) -> Vec<KnowledgeChunkSearchHit> {
    let mut merged = std::collections::BTreeMap::<u64, KnowledgeChunkSearchHit>::new();

    for hit in keyword_hits {
        merged.insert(
            hit.chunk_id,
            KnowledgeChunkSearchHit {
                score: hit.score * 0.4,
                match_reason: Some("hybrid_keyword".to_string()),
                ..hit
            },
        );
    }

    for hit in vector_hits {
        merged
            .entry(hit.chunk_id)
            .and_modify(|existing| {
                existing.score += hit.score * 0.6;
                existing.match_reason = Some("hybrid_keyword_vector".to_string());
            })
            .or_insert(KnowledgeChunkSearchHit {
                score: hit.score * 0.6,
                match_reason: Some("hybrid_vector".to_string()),
                ..hit
            });
    }

    let mut hits = merged.into_values().collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

fn push_keyword_score_expression(
    query: &mut QueryBuilder<sqlx::Any>,
    terms: &[String],
    keyword_match: &str,
) {
    query.push("COALESCE(ts_rank_cd(c.search_vector, to_tsquery('simple', ");
    query.push_bind(keyword_match.to_string());
    query.push(")), 0.0)");
    for term in terms {
        query.push(" + CASE WHEN LOWER(d.title) LIKE ");
        query.push_bind(like_contains_term(term));
        query.push(" ESCAPE '\\' THEN 0.5 ELSE 0.0 END");
    }
}

/// Builds a `%term%` LIKE pattern with the term's `%`/`_`/`\` escaped so user
/// input cannot inject wildcards into the title-match predicate; the paired
/// SQL fragments use `ESCAPE '\'`.
fn like_contains_term(term: &str) -> String {
    let mut pattern = String::with_capacity(term.len() + 2);
    pattern.push('%');
    for ch in term.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(ch);
    }
    pattern.push('%');
    pattern
}

fn push_keyword_or_title_filter(
    query: &mut QueryBuilder<sqlx::Any>,
    terms: &[String],
    term_operator: TermMatchOperator,
    keyword_match: &str,
) {
    push_postgres_ts_or_title_filter(query, terms, term_operator, keyword_match)
}

fn push_postgres_ts_or_title_filter(
    query: &mut QueryBuilder<sqlx::Any>,
    terms: &[String],
    term_operator: TermMatchOperator,
    tsquery: &str,
) {
    match term_operator {
        TermMatchOperator::Any => {
            query.push("(c.search_vector @@ to_tsquery('simple', ");
            query.push_bind(tsquery.to_string());
            query.push(")");
            for term in terms {
                query.push(" OR LOWER(d.title) LIKE ");
                query.push_bind(like_contains_term(term));
                query.push(" ESCAPE '\\'");
            }
            query.push(")");
        }
        TermMatchOperator::All => {
            for (index, term) in terms.iter().enumerate() {
                if index > 0 {
                    query.push(" AND ");
                }
                let single_term_query =
                    build_postgres_tsquery(std::slice::from_ref(term), TermMatchOperator::All);
                query.push("(c.search_vector @@ to_tsquery('simple', ");
                query.push_bind(single_term_query);
                query.push(") OR LOWER(d.title) LIKE ");
                query.push_bind(like_contains_term(term));
                query.push(" ESCAPE '\\'");
                query.push(")");
            }
        }
    }
}

fn build_keyword_match_expression(terms: &[String], term_operator: TermMatchOperator) -> String {
    build_postgres_tsquery(terms, term_operator)
}

fn build_postgres_tsquery(terms: &[String], term_operator: TermMatchOperator) -> String {
    let escaped_terms: Vec<String> = terms.iter().map(|term| escape_tsquery_term(term)).collect();
    if escaped_terms.is_empty() {
        return String::new();
    }
    let separator = match term_operator {
        TermMatchOperator::Any => " | ",
        TermMatchOperator::All => " & ",
    };
    escaped_terms.join(separator)
}

fn escape_tsquery_term(term: &str) -> String {
    let sanitized: String = term
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if sanitized.is_empty() {
        "empty".to_string()
    } else {
        sanitized
    }
}

fn normalized_query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|term| {
            term.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|term| !term.is_empty())
        .take(8)
        .collect()
}

fn chunk_hit_from_row(
    row: AnyRow,
    method: KnowledgeRetrievalMethod,
    min_score: Option<f64>,
) -> Result<Option<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
    let score: f64 = row.try_get("score").map_err(backend_sqlx_error)?;
    if min_score
        .map(|min_score| score < min_score)
        .unwrap_or(false)
    {
        return Ok(None);
    }

    Ok(Some(KnowledgeChunkSearchHit {
        chunk_id: u64_from_row(&row, "chunk_id")?,
        document_id: u64_from_row(&row, "document_id")?,
        document_version_id: optional_u64_from_row(&row, "document_version_id")?,
        space_id: u64_from_row(&row, "space_id")?,
        title: row.try_get("title").map_err(backend_sqlx_error)?,
        content: row.try_get("content_text").map_err(backend_sqlx_error)?,
        score,
        token_count: optional_i64_from_row(&row, "token_count")?.map(|value| value as u32),
        locator: row.try_get("locator").map_err(backend_sqlx_error)?,
        source_uri: row.try_get("source_uri").map_err(backend_sqlx_error)?,
        retrieval_method: method,
        match_reason: Some(format!("{method:?}")),
    }))
}

fn trace_status_code(status: &str) -> Result<i64, KnowledgeRetrievalTraceStoreError> {
    match status {
        "succeeded" => Ok(SUCCEEDED_STATUS),
        value => Err(KnowledgeRetrievalTraceStoreError::Internal(format!(
            "unsupported retrieval trace status: {value}"
        ))),
    }
}

fn trace_status_name(status: i64) -> Result<String, KnowledgeRetrievalTraceStoreError> {
    match status {
        SUCCEEDED_STATUS => Ok("succeeded".to_string()),
        value => Err(KnowledgeRetrievalTraceStoreError::Internal(format!(
            "unsupported retrieval trace status code: {value}"
        ))),
    }
}

fn trace_method_from_match_reason(
    value: Option<String>,
) -> Result<KnowledgeRetrievalMethod, KnowledgeRetrievalTraceStoreError> {
    match value
        .as_deref()
        .unwrap_or("hybrid")
        .to_ascii_lowercase()
        .as_str()
    {
        "exact" => Ok(KnowledgeRetrievalMethod::Exact),
        "keyword" => Ok(KnowledgeRetrievalMethod::Keyword),
        "fulltext" | "full_text" => Ok(KnowledgeRetrievalMethod::FullText),
        "structured" => Ok(KnowledgeRetrievalMethod::Structured),
        "graph" => Ok(KnowledgeRetrievalMethod::Graph),
        "vector" => Ok(KnowledgeRetrievalMethod::Vector),
        "hybrid" => Ok(KnowledgeRetrievalMethod::Hybrid),
        "llmrerank" | "llm_rerank" => Ok(KnowledgeRetrievalMethod::LlmRerank),
        "external" => Ok(KnowledgeRetrievalMethod::External),
        value => Err(KnowledgeRetrievalTraceStoreError::Internal(format!(
            "unsupported retrieval hit match reason: {value}"
        ))),
    }
}

fn trace_record_from_row(
    row: AnyRow,
) -> Result<KnowledgeRetrievalTraceRecord, KnowledgeRetrievalTraceStoreError> {
    let result_count = row
        .try_get::<i64, _>("result_count")
        .map_err(trace_sqlx_error)?;
    Ok(KnowledgeRetrievalTraceRecord {
        tenant_id: trace_u64_from_row(&row, "tenant_id")?,
        retrieval_trace_id: trace_u64_from_row(&row, "retrieval_trace_id")?,
        actor_id: trace_optional_u64_from_row(&row, "actor_id")?,
        retrieval_profile_id: trace_optional_u64_from_row(&row, "retrieval_profile_id")?,
        query_text_redacted: row
            .try_get("query_text_redacted")
            .map_err(trace_sqlx_error)?,
        latency_ms: trace_optional_i64_from_row(&row, "latency_ms")?.map(|value| value as u64),
        result_count: u32::try_from(result_count).map_err(|_| {
            KnowledgeRetrievalTraceStoreError::Internal(
                "result_count is out of u32 range".to_string(),
            )
        })?,
        status: trace_status_name(row.try_get("status").map_err(trace_sqlx_error)?)?,
    })
}

fn trace_hit_from_row(
    row: AnyRow,
) -> Result<KnowledgeRetrievalTraceHitRecord, KnowledgeRetrievalTraceStoreError> {
    let result_rank = row
        .try_get::<i64, _>("result_rank")
        .map_err(trace_sqlx_error)?;
    Ok(KnowledgeRetrievalTraceHitRecord {
        chunk_id: trace_u64_from_row(&row, "chunk_id")?,
        document_id: trace_u64_from_row(&row, "document_id")?,
        document_version_id: trace_optional_u64_from_row(&row, "document_version_id")?,
        space_id: trace_u64_from_row(&row, "space_id")?,
        title: row.try_get("title").map_err(trace_sqlx_error)?,
        content: row.try_get("content_text").map_err(trace_sqlx_error)?,
        score: row.try_get("score").map_err(trace_sqlx_error)?,
        result_rank: u32::try_from(result_rank).map_err(|_| {
            KnowledgeRetrievalTraceStoreError::Internal(
                "result_rank is out of u32 range".to_string(),
            )
        })?,
        token_count: trace_optional_i64_from_row(&row, "token_count")?.map(|value| value as u32),
        retrieval_method: trace_method_from_match_reason(
            row.try_get("match_reason").map_err(trace_sqlx_error)?,
        )?,
        citation_json: row.try_get("citation").map_err(trace_sqlx_error)?,
    })
}

fn u64_from_row(row: &AnyRow, column: &str) -> Result<u64, KnowledgeRetrievalBackendError> {
    let value: i64 = row.try_get(column).map_err(backend_sqlx_error)?;
    u64::try_from(value).map_err(|_| {
        KnowledgeRetrievalBackendError::Internal(format!("{column} must not be negative"))
    })
}

fn optional_u64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<Option<u64>, KnowledgeRetrievalBackendError> {
    optional_i64_from_row(row, column)?
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                KnowledgeRetrievalBackendError::Internal(format!("{column} must not be negative"))
            })
        })
        .transpose()
}

fn optional_i64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<Option<i64>, KnowledgeRetrievalBackendError> {
    row.try_get(column).map_err(backend_sqlx_error)
}

fn backend_to_i64(field_name: &str, value: u64) -> Result<i64, KnowledgeRetrievalBackendError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeRetrievalBackendError::Internal(format!("{field_name} exceeds signed int64 range"))
    })
}

fn trace_to_i64(field_name: &str, value: u64) -> Result<i64, KnowledgeRetrievalTraceStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeRetrievalTraceStoreError::Internal(format!(
            "{field_name} exceeds signed int64 range"
        ))
    })
}

fn trace_u64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<u64, KnowledgeRetrievalTraceStoreError> {
    let value: i64 = row.try_get(column).map_err(trace_sqlx_error)?;
    u64::try_from(value).map_err(|_| {
        KnowledgeRetrievalTraceStoreError::Internal(format!("{column} must not be negative"))
    })
}

fn trace_optional_u64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<Option<u64>, KnowledgeRetrievalTraceStoreError> {
    trace_optional_i64_from_row(row, column)?
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                KnowledgeRetrievalTraceStoreError::Internal(format!(
                    "{column} must not be negative"
                ))
            })
        })
        .transpose()
}

fn trace_optional_i64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<Option<i64>, KnowledgeRetrievalTraceStoreError> {
    row.try_get(column).map_err(trace_sqlx_error)
}

fn now_rfc3339() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| error.to_string())
}

fn backend_sqlx_error(error: sqlx::Error) -> KnowledgeRetrievalBackendError {
    KnowledgeRetrievalBackendError::Internal(error.to_string())
}

fn trace_sqlx_error(error: sqlx::Error) -> KnowledgeRetrievalTraceStoreError {
    KnowledgeRetrievalTraceStoreError::Internal(error.to_string())
}

fn trace_id_error(
    error: crate::id::KnowledgeIdGeneratorError,
) -> KnowledgeRetrievalTraceStoreError {
    KnowledgeRetrievalTraceStoreError::Internal(error.to_string())
}

#[cfg(test)]
mod fts_tests {
    use super::*;

    #[test]
    fn postgres_tsquery_joins_terms_for_any_and_all() {
        let terms = vec!["alpha".to_string(), "beta".to_string()];
        assert_eq!(
            build_postgres_tsquery(&terms, TermMatchOperator::Any),
            "alpha | beta"
        );
        assert_eq!(
            build_postgres_tsquery(&terms, TermMatchOperator::All),
            "alpha & beta"
        );
    }
}
