//! PostgreSQL pgvector ANN retrieval backend.

use crate::binding_scope_filters::push_binding_scope_filters_postgres;
use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_backend::{
    KnowledgeChunkSearchHit, KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend,
    KnowledgeRetrievalBackendError,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeRetrievalMethod;
use sqlx::{PgPool, QueryBuilder, Row};

const ACTIVE_STATUS: i64 = 1;

#[derive(Debug, Clone)]
pub struct PgVectorKnowledgeRetrievalBackend {
    pool: PgPool,
    tenant_id: u64,
    organization_id: u64,
}

impl PgVectorKnowledgeRetrievalBackend {
    pub fn new(pool: PgPool, tenant_id: u64, organization_id: u64) -> Self {
        Self {
            pool,
            tenant_id,
            organization_id,
        }
    }
}

#[async_trait]
impl KnowledgeRetrievalBackend for PgVectorKnowledgeRetrievalBackend {
    async fn search_chunks(
        &self,
        request: KnowledgeChunkSearchRequest,
    ) -> Result<Vec<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
        if request.tenant_id != self.tenant_id {
            return Err(KnowledgeRetrievalBackendError::TenantMismatch);
        }

        match request.method {
            KnowledgeRetrievalMethod::Vector | KnowledgeRetrievalMethod::Hybrid => {}
            other => {
                return Err(KnowledgeRetrievalBackendError::UnsupportedMethod(other));
            }
        }

        let query_embedding = request.query_embedding.as_ref().ok_or_else(|| {
            KnowledgeRetrievalBackendError::Internal(
                "pgvector search requires query_embedding".to_string(),
            )
        })?;
        if query_embedding.iter().any(|value| !value.is_finite()) {
            return Err(KnowledgeRetrievalBackendError::Internal(
                "query embedding contains a non-finite value".to_string(),
            ));
        }
        let vector_literal = format_pgvector_literal(query_embedding);
        let tenant_id = backend_to_i64("tenant_id", self.tenant_id)?;
        let organization_id = backend_to_i64("organization_id", self.organization_id)?;
        let space_id = backend_to_i64("space_id", request.binding.space_id)?;
        let collection_id = request
            .binding
            .collection_id
            .map(|value| backend_to_i64("collection_id", value))
            .transpose()?;
        let top_k = i64::from(request.top_k.clamp(1, 64));
        let min_score = request.binding.min_score.unwrap_or(0.0);

        let mut query = QueryBuilder::new(
            r#"
            SELECT
                c.id AS chunk_id,
                c.document_id,
                c.document_version_id,
                c.space_id,
                c.collection_id,
                d.title,
                c.content_text,
                c.token_count,
                c.locator,
                'kb://documents/' || c.document_id::text AS source_uri,
                (1 - (e.embedding_vector <=> CAST(
            "#,
        );
        query.push_bind(vector_literal.clone());
        query.push(
            r#"
                 AS vector))) AS score
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
             AND e.embedding_vector IS NOT NULL
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
        if let Some(collection_id) = collection_id {
            query.push(" AND c.collection_id = ");
            query.push_bind(collection_id);
        }
        push_binding_scope_filters_postgres(
            &mut query,
            tenant_id,
            organization_id,
            space_id,
            &request.binding,
        )?;
        query.push(" ORDER BY e.embedding_vector <=> CAST(");
        query.push_bind(vector_literal);
        query.push(" AS vector) LIMIT ");
        query.push_bind(top_k);

        let rows = query
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(backend_sqlx_error)?;

        rows.into_iter()
            .filter_map(|row| chunk_hit_from_row(row, request.method, min_score).transpose())
            .collect()
    }
}

fn chunk_hit_from_row(
    row: sqlx::postgres::PgRow,
    method: KnowledgeRetrievalMethod,
    min_score: f64,
) -> Result<Option<KnowledgeChunkSearchHit>, KnowledgeRetrievalBackendError> {
    let score: f64 = row.try_get("score").map_err(backend_sqlx_error)?;
    if score < min_score {
        return Ok(None);
    }
    Ok(Some(KnowledgeChunkSearchHit {
        chunk_id: row
            .try_get::<i64, _>("chunk_id")
            .map_err(backend_sqlx_error)? as u64,
        document_id: row
            .try_get::<i64, _>("document_id")
            .map_err(backend_sqlx_error)? as u64,
        document_version_id: row
            .try_get::<Option<i64>, _>("document_version_id")
            .map_err(backend_sqlx_error)?
            .map(|value| value as u64),
        space_id: row
            .try_get::<i64, _>("space_id")
            .map_err(backend_sqlx_error)? as u64,
        collection_id: row
            .try_get::<Option<i64>, _>("collection_id")
            .map_err(backend_sqlx_error)?
            .map(|value| value as u64),
        title: row.try_get("title").map_err(backend_sqlx_error)?,
        content: row.try_get("content_text").map_err(backend_sqlx_error)?,
        score,
        token_count: row
            .try_get::<Option<i64>, _>("token_count")
            .map_err(backend_sqlx_error)?
            .map(|value| value as u32),
        locator: row.try_get("locator").map_err(backend_sqlx_error)?,
        source_uri: row.try_get("source_uri").map_err(backend_sqlx_error)?,
        retrieval_method: method,
        match_reason: Some("pgvector_ann".to_string()),
    }))
}

pub fn format_pgvector_literal(vector: &[f32]) -> String {
    let mut output = String::from("[");
    for (index, value) in vector.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        // NaN/±Inf cannot round-trip through the Postgres `vector` type and would turn a
        // valid retrieval into a server-side parse error; bounded embeddings are written
        // verbatim, non-finite values are rejected as invalid input instead.
        if !value.is_finite() {
            return String::new();
        }
        output.push_str(&value.to_string());
    }
    output.push(']');
    output
}

fn backend_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeRetrievalBackendError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeRetrievalBackendError::Internal(format!("{field} exceeds i64 range: {value}"))
    })
}

fn backend_sqlx_error(error: sqlx::Error) -> KnowledgeRetrievalBackendError {
    KnowledgeRetrievalBackendError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgvector_literal_serializes_f32_components() {
        assert_eq!(format_pgvector_literal(&[1.0, 0.5]), "[1,0.5]");
    }
}
