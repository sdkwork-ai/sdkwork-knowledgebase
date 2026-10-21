use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::DriveImportMetadataStoreError;
use sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::{
    MarkdownIndexMetadataStore, MarkdownIndexMetadataStoreError, MarkdownIndexSourceBinding,
    PrepareMarkdownIndexMetadataRecord, PreparedMarkdownIndexMetadata,
};
use sdkwork_knowledgebase_contract::document::KnowledgeDocument;
use sdkwork_knowledgebase_contract::source::KnowledgeSourceType;
use sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits;
use sqlx::{Any, AnyPool, Row};
use std::sync::Arc;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{default_knowledge_id_generator, KnowledgeIdGenerator};
use crate::postgres_knowledge_document_metadata_transaction::{
    bind_document_current_version_in_transaction, create_or_get_document_in_transaction,
    create_or_get_document_version_in_transaction, create_or_get_object_ref_in_transaction,
    create_or_get_source_in_transaction, document_from_row, METADATA_ACTIVE_STATUS,
};
use crate::quota_transaction::{
    begin_tenant_quota_transaction, enforce_tenant_quotas_after_write, TenantQuotaTransactionError,
};

#[derive(Debug, Clone)]
pub struct PostgresMarkdownIndexMetadataStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    database_engine: DatabaseEngine,
    quota_limits: Option<KnowledgebaseTenantQuotaLimits>,
}

impl PostgresMarkdownIndexMetadataStore {
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
impl MarkdownIndexMetadataStore for PostgresMarkdownIndexMetadataStore {
    async fn create_or_prepare_markdown_index_metadata(
        &self,
        record: PrepareMarkdownIndexMetadataRecord,
    ) -> Result<PreparedMarkdownIndexMetadata, MarkdownIndexMetadataStoreError> {
        postgres_create_or_prepare_markdown_index_metadata(self, record).await
    }
}

async fn postgres_create_or_prepare_markdown_index_metadata(
    store: &PostgresMarkdownIndexMetadataStore,
    record: PrepareMarkdownIndexMetadataRecord,
) -> Result<PreparedMarkdownIndexMetadata, MarkdownIndexMetadataStoreError> {
    use sdkwork_intelligence_knowledgebase_service::ports::markdown_index_metadata_store::validate_markdown_index_document_identity;

    let pool = &store.pool;
    let tenant_id = store.tenant_id;
    let organization_id = store.organization_id;
    let id_generator = &store.id_generator;
    let timestamp_dialect = store.timestamp_dialect;
    let quota_limits = store.quota_limits;

    validate_markdown_index_document_identity(&record.document, &record.source)?;

    let PrepareMarkdownIndexMetadataRecord {
        source,
        object_ref: object_ref_record,
        document: document_record,
        version: version_record,
    } = record;

    let tenant_id_i64 = i64::try_from(tenant_id).map_err(|_| {
        MarkdownIndexMetadataStoreError::invalid_request("tenant_id exceeds i64 range")
    })?;
    let organization_id_i64 = i64::try_from(organization_id).map_err(|_| {
        MarkdownIndexMetadataStoreError::invalid_request("organization_id exceeds i64 range")
    })?;
    let mut transaction = if quota_limits.is_some() {
        begin_tenant_quota_transaction(pool, tenant_id_i64, organization_id_i64)
            .await
            .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?
    } else {
        pool.begin()
            .await
            .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?
    };

    let source_id = match &source {
        MarkdownIndexSourceBinding::Existing { source_id } => *source_id,
        MarkdownIndexSourceBinding::Create(source_record) => {
            create_or_get_source_in_transaction(
                &mut transaction,
                tenant_id,
                organization_id,
                id_generator,
                timestamp_dialect,
                source_record,
            )
            .await
            .map_err(map_markdown_index_metadata_error)?
            .id
        }
    };

    let object_ref = create_or_get_object_ref_in_transaction(
        &mut transaction,
        tenant_id,
        organization_id,
        id_generator,
        timestamp_dialect,
        &object_ref_record,
    )
    .await
    .map_err(map_markdown_index_metadata_error)?;

    // The API ingest surface is also the document editor's save path: a payload
    // for a note that already exists in this space must append a version to that
    // note instead of importing another same-titled document. Only documents the
    // API surface owns are reused (a source-less document created by the editor,
    // or a document whose source is an API payload); upload, drive, and connector
    // imports keep their own document identity.
    let reusable_document = match &source {
        MarkdownIndexSourceBinding::Create(source_record)
            if source_record.source_type == KnowledgeSourceType::Api =>
        {
            find_reusable_api_document_in_transaction(
                &mut transaction,
                tenant_id_i64,
                organization_id_i64,
                document_record.space_id,
                document_record.title.as_str(),
            )
            .await?
        }
        _ => None,
    };

    let (document, version) = match reusable_document {
        Some(existing) => {
            let next_version_no = next_document_version_no_in_transaction(
                &mut transaction,
                tenant_id_i64,
                organization_id_i64,
                existing.id,
            )
            .await?;
            let mut appended_version = version_record;
            appended_version.document_id = existing.id;
            appended_version.version_no = next_version_no;
            appended_version.original_object_ref_id = object_ref.id;
            let version = create_or_get_document_version_in_transaction(
                &mut transaction,
                tenant_id,
                organization_id,
                id_generator,
                timestamp_dialect,
                &appended_version,
            )
            .await
            .map_err(map_markdown_index_metadata_error)?;
            bind_document_current_version_in_transaction(
                &mut transaction,
                tenant_id,
                organization_id,
                timestamp_dialect,
                existing.id,
                version.id,
                version.version_no,
            )
            .await
            .map_err(map_markdown_index_metadata_error)?;
            let document = reload_document_in_transaction(
                &mut transaction,
                tenant_id_i64,
                organization_id_i64,
                existing.id,
            )
            .await?;
            (document, version)
        }
        None => {
            let mut document_record = document_record;
            document_record.source_id = Some(source_id);
            let document = create_or_get_document_in_transaction(
                &mut transaction,
                tenant_id,
                organization_id,
                id_generator,
                timestamp_dialect,
                &document_record,
            )
            .await
            .map_err(map_markdown_index_metadata_error)?;

            let mut version_record = version_record;
            version_record.document_id = document.id;
            version_record.original_object_ref_id = object_ref.id;
            let version = create_or_get_document_version_in_transaction(
                &mut transaction,
                tenant_id,
                organization_id,
                id_generator,
                timestamp_dialect,
                &version_record,
            )
            .await
            .map_err(map_markdown_index_metadata_error)?;

            bind_document_current_version_in_transaction(
                &mut transaction,
                tenant_id,
                organization_id,
                timestamp_dialect,
                document.id,
                version.id,
                version.version_no,
            )
            .await
            .map_err(map_markdown_index_metadata_error)?;
            (document, version)
        }
    };

    if let Some(limits) = quota_limits {
        enforce_tenant_quotas_after_write(
            &mut transaction,
            tenant_id_i64,
            organization_id_i64,
            limits,
        )
        .await
        .map_err(map_quota_transaction_error)?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?;

    Ok(PreparedMarkdownIndexMetadata {
        source_id,
        object_ref,
        document,
        version,
    })
}

/// Finds the document an API payload belongs to.
///
/// A document the editor created without content has no source yet, and a
/// document produced by an earlier API ingest carries an `api` source; both are
/// owned by the API surface, so a payload with the same title appends a version
/// to that document. Documents imported from upload, drive, or connector
/// sources keep their own identity and are never matched here.
async fn find_reusable_api_document_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Any>,
    tenant_id: i64,
    organization_id: i64,
    space_id: u64,
    title: &str,
) -> Result<Option<KnowledgeDocument>, MarkdownIndexMetadataStoreError> {
    let space_id = i64::try_from(space_id).map_err(|_| {
        MarkdownIndexMetadataStoreError::invalid_request("space_id exceeds i64 range")
    })?;
    let row = sqlx::query(
        r#"
        SELECT d.id, d.space_id, d.source_id, d.original_file_drive_node_id, d.title, d.mime_type,
               d.language, d.current_version_id, d.visibility, d.content_state, d.index_state
        FROM kb_document d
        LEFT JOIN kb_source s
          ON s.id = d.source_id
         AND s.tenant_id = d.tenant_id
         AND s.organization_id = d.organization_id
        WHERE d.tenant_id = $1
          AND d.organization_id = $2
          AND d.space_id = $3
          AND d.status = $4
          AND d.title = $5
          AND (
              d.source_id IS NULL
              OR (s.source_type = $6 AND s.status = $4)
          )
        ORDER BY d.id
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(space_id)
    .bind(METADATA_ACTIVE_STATUS)
    .bind(title)
    .bind(KnowledgeSourceType::Api.as_str())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?;

    match row {
        Some(row) => document_from_row(&row)
            .map(Some)
            .map_err(map_markdown_index_metadata_error),
        None => Ok(None),
    }
}

/// Allocates the next document version number inside the write transaction.
async fn next_document_version_no_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Any>,
    tenant_id: i64,
    organization_id: i64,
    document_id: u64,
) -> Result<u64, MarkdownIndexMetadataStoreError> {
    let document_id = i64::try_from(document_id).map_err(|_| {
        MarkdownIndexMetadataStoreError::invalid_request("document_id exceeds i64 range")
    })?;
    let row = sqlx::query(
        r#"
        SELECT COALESCE(MAX(version_no), 0) + 1 AS next_version_no
        FROM kb_document_version
        WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND status = $4
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(document_id)
    .bind(METADATA_ACTIVE_STATUS)
    .fetch_one(&mut **transaction)
    .await
    .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?;
    let next_version_no: i64 = row
        .try_get("next_version_no")
        .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?;
    u64::try_from(next_version_no)
        .map_err(|_| MarkdownIndexMetadataStoreError::internal("document version counter overflow"))
}

/// Reloads one document so the caller observes the version binding it just wrote.
async fn reload_document_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Any>,
    tenant_id: i64,
    organization_id: i64,
    document_id: u64,
) -> Result<KnowledgeDocument, MarkdownIndexMetadataStoreError> {
    let document_id = i64::try_from(document_id).map_err(|_| {
        MarkdownIndexMetadataStoreError::invalid_request("document_id exceeds i64 range")
    })?;
    let row = sqlx::query(
        r#"
        SELECT id, space_id, source_id, original_file_drive_node_id, title, mime_type, language,
               current_version_id, visibility, content_state, index_state
        FROM kb_document
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(document_id)
    .bind(METADATA_ACTIVE_STATUS)
    .fetch_one(&mut **transaction)
    .await
    .map_err(|error| MarkdownIndexMetadataStoreError::internal(error.to_string()))?;
    document_from_row(&row).map_err(map_markdown_index_metadata_error)
}

fn map_markdown_index_metadata_error(
    error: DriveImportMetadataStoreError,
) -> MarkdownIndexMetadataStoreError {
    match error {
        DriveImportMetadataStoreError::InvalidRequest(detail) => {
            MarkdownIndexMetadataStoreError::invalid_request(detail)
        }
        DriveImportMetadataStoreError::Conflict(detail) => {
            MarkdownIndexMetadataStoreError::Conflict(detail)
        }
        DriveImportMetadataStoreError::QuotaExceeded(error) => {
            MarkdownIndexMetadataStoreError::QuotaExceeded(error)
        }
        DriveImportMetadataStoreError::Internal(detail) => {
            MarkdownIndexMetadataStoreError::internal(detail)
        }
    }
}

fn map_quota_transaction_error(
    error: TenantQuotaTransactionError,
) -> MarkdownIndexMetadataStoreError {
    match error {
        TenantQuotaTransactionError::Quota(error) => {
            MarkdownIndexMetadataStoreError::QuotaExceeded(error)
        }
        TenantQuotaTransactionError::Database(error) => {
            MarkdownIndexMetadataStoreError::internal(error.to_string())
        }
        TenantQuotaTransactionError::Invalid(detail) => {
            MarkdownIndexMetadataStoreError::internal(detail)
        }
    }
}
