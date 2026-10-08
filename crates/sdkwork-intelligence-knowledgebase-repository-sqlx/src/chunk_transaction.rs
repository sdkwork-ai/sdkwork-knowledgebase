use sdkwork_intelligence_knowledgebase_service::ports::knowledge_chunk_store::{
    CreateKnowledgeChunkRecord, KnowledgeChunkStoreError,
};
use sqlx::{Any, AnyPool, Transaction};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{next_i64_id, KnowledgeIdGenerator};

const ACTIVE_STATUS: i64 = 1;
const INITIAL_VERSION: i64 = 0;
const CHUNK_INSERT_BATCH_SIZE: usize = 50;

struct PreparedChunkRow {
    id: i64,
    uuid: String,
    space_id: i64,
    document_id: i64,
    chunk_index: i64,
    content_text: String,
    content_hash: String,
    token_count: Option<i64>,
    locator: Option<String>,
}

pub(crate) struct ReplaceVersionChunksContext<'a> {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub id_generator: &'a Arc<dyn KnowledgeIdGenerator>,
    pub timestamp_dialect: SqlTimestampDialect,
    pub document_version_id: u64,
}

pub(crate) async fn replace_version_chunks_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    context: ReplaceVersionChunksContext<'_>,
    chunks: &[CreateKnowledgeChunkRecord],
) -> Result<usize, KnowledgeChunkStoreError> {
    let tenant_id_i64 = chunk_to_i64("tenant_id", context.tenant_id)?;
    let organization_id_i64 = chunk_to_i64("organization_id", context.organization_id)?;
    let version_id = chunk_to_i64("document_version_id", context.document_version_id)?;
    let now = chunk_now()?;

    // Embeddings are derived data owned by their chunks: removing them here (same
    // transaction) keeps re-replacing an existing version's chunks from aborting on
    // the RESTRICT FK from kb_embedding, and leaves no ghost vectors pointing at
    // deleted chunk ids. Retrieval hits stay immutable on purpose — a version with
    // recorded hits still fails the chunk delete loudly instead of rewriting history.
    sqlx::query(
        r#"
        DELETE FROM kb_embedding
        WHERE tenant_id = $1 AND organization_id = $2 AND chunk_id IN (
            SELECT id FROM kb_chunk
            WHERE tenant_id = $1 AND organization_id = $2 AND document_version_id = $3
        )
        "#,
    )
    .bind(tenant_id_i64)
    .bind(organization_id_i64)
    .bind(version_id)
    .execute(&mut **transaction)
    .await
    .map_err(chunk_internal_error)?;

    sqlx::query(
        r#"
        DELETE FROM kb_chunk
        WHERE tenant_id = $1 AND organization_id = $2 AND document_version_id = $3
        "#,
    )
    .bind(tenant_id_i64)
    .bind(organization_id_i64)
    .bind(version_id)
    .execute(&mut **transaction)
    .await
    .map_err(chunk_internal_error)?;

    if chunks.is_empty() {
        return Ok(0);
    }

    let mut prepared = Vec::with_capacity(chunks.len());
    for record in chunks {
        if record.document_version_id != context.document_version_id {
            return Err(KnowledgeChunkStoreError::InvalidRecord(
                "chunk document_version_id must match replace target".to_string(),
            ));
        }

        prepared.push(PreparedChunkRow {
            id: next_i64_id(context.id_generator).map_err(chunk_id_error)?,
            uuid: Uuid::new_v4().to_string(),
            space_id: chunk_to_i64("space_id", record.space_id)?,
            document_id: chunk_to_i64("document_id", record.document_id)?,
            chunk_index: i64::from(record.chunk_index),
            content_text: record.content_text.clone(),
            content_hash: record.content_hash.clone(),
            token_count: record.token_count.map(i64::from),
            locator: record.locator.clone(),
        });
    }

    for batch in prepared.chunks(CHUNK_INSERT_BATCH_SIZE) {
        bulk_insert_kb_chunks_postgres(
            transaction,
            tenant_id_i64,
            organization_id_i64,
            version_id,
            context.timestamp_dialect,
            &now,
            batch,
        )
        .await?;
    }

    Ok(chunks.len())
}

async fn bulk_insert_kb_chunks_postgres(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: i64,
    organization_id: i64,
    version_id: i64,
    timestamp_dialect: SqlTimestampDialect,
    now: &str,
    batch: &[PreparedChunkRow],
) -> Result<(), KnowledgeChunkStoreError> {
    // Any-dialect QueryBuilder emits `?` placeholders, which the sqlx 0.9
    // Postgres backend forwards verbatim; Postgres then parses `?` as the
    // jsonb key-exists operator and the INSERT dies with a syntax error.
    // Every other store in this crate hand-writes `$N` placeholders — do the
    // same here (VALUES tuples of 17 expressions each).
    const VALUES_PER_ROW: usize = 17;
    let _ = timestamp_dialect;
    let mut sql = String::with_capacity(512 + batch.len() * 220);
    sql.push_str(
        r#"
        INSERT INTO kb_chunk (
            id, uuid, tenant_id, organization_id, space_id, document_id,
            document_version_id, chunk_index, content_text, content_hash,
            token_count, locator, status, created_at, updated_at, version,
            search_vector
        )
        VALUES "#,
    );
    for (row_index, _) in batch.iter().enumerate() {
        if row_index > 0 {
            sql.push_str(", ");
        }
        sql.push('(');
        let base = row_index * VALUES_PER_ROW;
        let placeholder = |index: usize| format!("${}", base + index);
        sql.push_str(&placeholder(1)); // id
        sql.push_str(&format!(", {}, {}", placeholder(2), placeholder(3))); // uuid, tenant_id
        sql.push_str(&format!(
            ", {}, {}, {}, {}, {}, {}, {}, {}",
            placeholder(4),  // organization_id
            placeholder(5),  // space_id
            placeholder(6),  // document_id
            placeholder(7),  // document_version_id
            placeholder(8),  // chunk_index
            placeholder(9),  // content_text
            placeholder(10), // content_hash
            placeholder(11), // token_count
        ));
        sql.push_str(&format!(
            ", to_jsonb({}), {}",
            placeholder(12), // locator (jsonb column; text encoded as a JSON string scalar)
            placeholder(13), // status
        ));
        sql.push_str(&format!(
            ", CAST({} AS TIMESTAMP), CAST({} AS TIMESTAMP), {}",
            placeholder(14),
            placeholder(15),
            placeholder(16), // version
        ));
        sql.push_str(&format!(
            ", to_tsvector('simple', {})",
            placeholder(17), // search_vector source text
        ));
        sql.push(')');
    }

    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
    for chunk in batch {
        query = query
            .bind(chunk.id)
            .bind(chunk.uuid.as_str())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(chunk.space_id)
            .bind(chunk.document_id)
            .bind(version_id)
            .bind(chunk.chunk_index)
            .bind(chunk.content_text.as_str())
            .bind(chunk.content_hash.as_str())
            .bind(chunk.token_count)
            .bind(chunk.locator.as_deref())
            .bind(ACTIVE_STATUS)
            .bind(now)
            .bind(now)
            .bind(INITIAL_VERSION)
            .bind(chunk.content_text.as_str());
    }
    query
        .execute(&mut **transaction)
        .await
        .map_err(chunk_internal_error)?;
    Ok(())
}

pub async fn replace_version_chunks_with_pool(
    pool: &AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    document_version_id: u64,
    chunks: Vec<CreateKnowledgeChunkRecord>,
) -> Result<usize, KnowledgeChunkStoreError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
    let count = replace_version_chunks_in_transaction(
        &mut tx,
        ReplaceVersionChunksContext {
            tenant_id,
            organization_id,
            id_generator,
            timestamp_dialect,
            document_version_id,
        },
        &chunks,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))?;
    Ok(count)
}

fn chunk_now() -> Result<String, KnowledgeChunkStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| KnowledgeChunkStoreError::Internal(error.to_string()))
}

fn chunk_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeChunkStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeChunkStoreError::InvalidRecord(format!("{field} exceeds i64 integer range"))
    })
}

fn chunk_id_error(error: crate::id::KnowledgeIdGeneratorError) -> KnowledgeChunkStoreError {
    KnowledgeChunkStoreError::Internal(error.to_string())
}

fn chunk_internal_error(error: sqlx::Error) -> KnowledgeChunkStoreError {
    KnowledgeChunkStoreError::Internal(error.to_string())
}
