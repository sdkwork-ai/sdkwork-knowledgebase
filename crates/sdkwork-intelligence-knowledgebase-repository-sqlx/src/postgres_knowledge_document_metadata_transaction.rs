//! Shared SQLite transaction helpers for knowledge document metadata chains.

use sdkwork_intelligence_knowledgebase_service::ports::drive_import_metadata_store::DriveImportMetadataStoreError;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_version_store::CreateKnowledgeDocumentVersionRecord;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_object_ref_store::CreateKnowledgeDriveObjectRefRecord;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_source_store::CreateKnowledgeSourceRecord;
use sdkwork_knowledgebase_contract::document::{
    KnowledgeDocument, KnowledgeDocumentState, KnowledgeDocumentVersion,
    KnowledgeDocumentVersionState, KnowledgeDocumentVisibility,
};
use sdkwork_knowledgebase_contract::drive::KnowledgeDriveObjectRef;
use sdkwork_knowledgebase_contract::source::{KnowledgeSource, KnowledgeSourceType};
use sqlx::{any::AnyRow, Any, Row, Transaction};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::SqlTimestampDialect;
use crate::id::{next_i64_id, KnowledgeIdGenerator};

pub(crate) const METADATA_ACTIVE_STATUS: i64 = 1;
pub(crate) const METADATA_INITIAL_VERSION: i64 = 0;

pub(crate) async fn create_or_get_object_ref_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: u64,
    organization_id: u64,
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    record: &CreateKnowledgeDriveObjectRefRecord,
) -> Result<KnowledgeDriveObjectRef, DriveImportMetadataStoreError> {
    let tenant_id_i64 = to_i64("tenant_id", tenant_id)?;
    let organization_id_i64 = to_i64("organization_id", organization_id)?;
    let space_id = to_i64("space_id", record.space_id)?;
    let size_bytes = to_i64("size_bytes", record.size_bytes)?;
    let id = next_i64_id(id_generator).map_err(id_gen_error)?;
    let now = now_rfc3339()?;
    let created_at_expr = timestamp_dialect.sql_timestamp_expr("$21");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$22");
    let query = format!(
        r#"
        INSERT INTO kb_drive_object_ref (
            id, uuid, tenant_id, organization_id, space_id, drive_provider_kind, drive_space_id, drive_node_id,
            logical_path, drive_storage_provider_id, drive_bucket, drive_object_key,
            drive_object_version, drive_etag, content_type, size_bytes, checksum_sha256_hex,
            object_role, access_mode, status, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, {created_at_expr}, {updated_at_expr}, $23)
        ON CONFLICT DO NOTHING
        RETURNING
            id, space_id, drive_provider_kind, drive_space_id, drive_node_id, logical_path,
            drive_storage_provider_id, drive_bucket, drive_object_key, drive_object_version,
            drive_etag, content_type, size_bytes, checksum_sha256_hex, object_role, access_mode
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(id)
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id_i64)
        .bind(organization_id_i64)
        .bind(space_id)
        .bind(&record.drive_provider_kind)
        .bind(&record.drive_space_id)
        .bind(&record.drive_node_id)
        .bind(&record.logical_path)
        .bind(&record.drive_storage_provider_id)
        .bind(&record.drive_bucket)
        .bind(&record.drive_object_key)
        .bind(&record.drive_object_version)
        .bind(&record.drive_etag)
        .bind(&record.content_type)
        .bind(size_bytes)
        .bind(&record.checksum_sha256_hex)
        .bind(&record.object_role)
        .bind(&record.access_mode)
        .bind(METADATA_ACTIVE_STATUS)
        .bind(now.clone())
        .bind(now)
        .bind(METADATA_INITIAL_VERSION)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sqlx_error)?;

    let object_ref = if let Some(row) = row {
        object_ref_from_row(&row)?
    } else {
        let row = sqlx::query(
            r#"
            SELECT
                id, space_id, drive_provider_kind, drive_space_id, drive_node_id, logical_path,
                drive_storage_provider_id, drive_bucket, drive_object_key, drive_object_version,
                drive_etag, content_type, size_bytes, checksum_sha256_hex, object_role, access_mode
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
        .bind(tenant_id_i64)
        .bind(organization_id_i64)
        .bind(space_id)
        .bind(&record.drive_storage_provider_id)
        .bind(&record.drive_bucket)
        .bind(&record.drive_object_key)
        .bind(&record.drive_object_version)
        .bind(&record.object_role)
        .bind(METADATA_ACTIVE_STATUS)
        .fetch_one(&mut **transaction)
        .await
        .map_err(sqlx_error)?;
        object_ref_from_row(&row)?
    };

    enrich_object_ref_drive_binding_in_transaction(
        transaction,
        tenant_id,
        organization_id,
        timestamp_dialect,
        &object_ref,
        record,
    )
    .await
}

async fn enrich_object_ref_drive_binding_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: u64,
    organization_id: u64,
    timestamp_dialect: SqlTimestampDialect,
    object_ref: &KnowledgeDriveObjectRef,
    record: &CreateKnowledgeDriveObjectRefRecord,
) -> Result<KnowledgeDriveObjectRef, DriveImportMetadataStoreError> {
    let should_update_drive_space =
        object_ref.drive_space_id.is_none() && record.drive_space_id.is_some();
    let should_update_drive_node =
        object_ref.drive_node_id.is_none() && record.drive_node_id.is_some();
    let should_update_logical_path =
        object_ref.logical_path.is_none() && record.logical_path.is_some();
    if !should_update_drive_space && !should_update_drive_node && !should_update_logical_path {
        return Ok(object_ref.clone());
    }

    let tenant_id = to_i64("tenant_id", tenant_id)?;
    let organization_id = to_i64("organization_id", organization_id)?;
    let object_ref_id = to_i64("object_ref_id", object_ref.id)?;
    let now = now_rfc3339()?;
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$4");
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
            id, space_id, drive_provider_kind, drive_space_id, drive_node_id, logical_path,
            drive_storage_provider_id, drive_bucket, drive_object_key, drive_object_version,
            drive_etag, content_type, size_bytes, checksum_sha256_hex, object_role, access_mode
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
        .bind(METADATA_ACTIVE_STATUS)
        .fetch_one(&mut **transaction)
        .await
        .map_err(sqlx_error)?;

    object_ref_from_row(&row)
}

pub(crate) async fn create_or_get_document_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: u64,
    organization_id: u64,
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    record: &CreateKnowledgeDocumentRecord,
) -> Result<KnowledgeDocument, DriveImportMetadataStoreError> {
    validate_document_identity(record)?;
    let tenant_id = to_i64("tenant_id", tenant_id)?;
    let organization_id = to_i64("organization_id", organization_id)?;
    let space_id = to_i64("space_id", record.space_id)?;
    let source_id = record
        .source_id
        .map(|value| to_i64("source_id", value))
        .transpose()?;
    let id = next_i64_id(id_generator).map_err(id_gen_error)?;
    let now = now_rfc3339()?;
    let created_at_expr = timestamp_dialect.sql_timestamp_expr("$16");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$17");
    let query = format!(
        r#"
        INSERT INTO kb_document (
            id, uuid, tenant_id, organization_id, space_id, source_id, identity_scope,
            original_file_drive_node_id, title, mime_type, language, visibility, content_state,
            index_state, status, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, {created_at_expr}, {updated_at_expr}, $18)
        ON CONFLICT DO NOTHING
        RETURNING id, space_id, source_id, original_file_drive_node_id, title, mime_type, language,
                  current_version_id, visibility, content_state, index_state
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(id)
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(source_id)
        .bind(record.identity_scope.as_str())
        .bind(&record.original_file_drive_node_id)
        .bind(&record.title)
        .bind(&record.mime_type)
        .bind(&record.language)
        .bind(document_visibility_code(KnowledgeDocumentVisibility::Space))
        .bind(document_state_code(KnowledgeDocumentState::Ready))
        .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
        .bind(METADATA_ACTIVE_STATUS)
        .bind(now.clone())
        .bind(now)
        .bind(METADATA_INITIAL_VERSION)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sqlx_error)?;

    let document = if let Some(row) = row {
        document_from_row(&row)?
    } else {
        let row = sqlx::query(
            r#"
            SELECT id, space_id, source_id, original_file_drive_node_id, title, mime_type, language,
                   current_version_id, visibility, content_state, index_state
            FROM kb_document
            WHERE tenant_id = $1
              AND organization_id = $2
              AND space_id = $3
              AND identity_scope = $4
              AND (
                  ($5 = 'source_only' AND source_id = $6)
                  OR (
                      $7 = 'source_and_original_drive_node'
                      AND (
                          ($8 IS NULL AND source_id IS NULL)
                          OR ($9 IS NOT NULL AND source_id = $10)
                      )
                      AND COALESCE(original_file_drive_node_id, '') = COALESCE($11, '')
                  )
              )
              AND status = $12
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(record.identity_scope.as_str())
        .bind(record.identity_scope.as_str())
        .bind(source_id)
        .bind(record.identity_scope.as_str())
        .bind(source_id)
        .bind(source_id)
        .bind(source_id)
        .bind(&record.original_file_drive_node_id)
        .bind(METADATA_ACTIVE_STATUS)
        .fetch_one(&mut **transaction)
        .await
        .map_err(sqlx_error)?;
        document_from_row(&row)?
    };

    enrich_document_drive_node_binding_in_transaction(
        transaction,
        tenant_id,
        organization_id,
        timestamp_dialect,
        &document,
        record.original_file_drive_node_id.as_deref(),
    )
    .await
}

async fn enrich_document_drive_node_binding_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: i64,
    organization_id: i64,
    timestamp_dialect: SqlTimestampDialect,
    document: &KnowledgeDocument,
    original_file_drive_node_id: Option<&str>,
) -> Result<KnowledgeDocument, DriveImportMetadataStoreError> {
    let Some(original_file_drive_node_id) = original_file_drive_node_id else {
        return Ok(document.clone());
    };
    if document.original_file_drive_node_id.is_some() {
        return Ok(document.clone());
    }

    let document_id = to_i64("document_id", document.id)?;
    let now = now_rfc3339()?;
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let query = format!(
        r#"
        UPDATE kb_document
        SET original_file_drive_node_id = $1, updated_at = {updated_at_expr}, version = version + 1
        WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
        RETURNING id, space_id, source_id, original_file_drive_node_id, title, mime_type, language,
                  current_version_id, visibility, content_state, index_state
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(original_file_drive_node_id)
        .bind(now)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(METADATA_ACTIVE_STATUS)
        .fetch_one(&mut **transaction)
        .await
        .map_err(sqlx_error)?;

    document_from_row(&row)
}

pub(crate) async fn create_or_get_document_version_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: u64,
    organization_id: u64,
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    record: &CreateKnowledgeDocumentVersionRecord,
) -> Result<KnowledgeDocumentVersion, DriveImportMetadataStoreError> {
    let tenant_id = to_i64("tenant_id", tenant_id)?;
    let organization_id = to_i64("organization_id", organization_id)?;
    let document_id = to_i64("document_id", record.document_id)?;
    let version_no = to_i64("version_no", record.version_no)?;
    let original_object_ref_id = to_i64("original_object_ref_id", record.original_object_ref_id)?;
    let size_bytes = to_i64("size_bytes", record.size_bytes)?;
    let generated_id = next_i64_id(id_generator).map_err(id_gen_error)?;
    let now = now_rfc3339()?;
    let submitted_at_expr = timestamp_dialect.sql_timestamp_expr("$13");
    let created_at_expr = timestamp_dialect.sql_timestamp_expr("$15");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$16");
    let query = format!(
        r#"
        INSERT INTO kb_document_version (
            id, uuid, tenant_id, organization_id, document_id, version_no, original_object_ref_id,
            checksum_sha256_hex, size_bytes, mime_type, parse_state, index_state,
            submitted_at, status, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {submitted_at_expr}, $14, {created_at_expr}, {updated_at_expr}, $17)
        ON CONFLICT DO NOTHING
        RETURNING
            id, document_id, version_no, original_object_ref_id, checksum_sha256_hex,
            size_bytes, mime_type, parse_state, index_state
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(generated_id)
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(version_no)
        .bind(original_object_ref_id)
        .bind(&record.checksum_sha256_hex)
        .bind(size_bytes)
        .bind(&record.mime_type)
        .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
        .bind(version_state_code(KnowledgeDocumentVersionState::Pending))
        .bind(now.clone())
        .bind(METADATA_ACTIVE_STATUS)
        .bind(now.clone())
        .bind(now)
        .bind(METADATA_INITIAL_VERSION)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sqlx_error)?;

    if let Some(row) = row {
        return document_version_from_row(&row);
    }

    let row = sqlx::query(
        r#"
        SELECT id, document_id, version_no, original_object_ref_id, checksum_sha256_hex,
               size_bytes, mime_type, parse_state, index_state
        FROM kb_document_version
        WHERE tenant_id = $1 AND organization_id = $2 AND document_id = $3 AND version_no = $4 AND status = $5
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(document_id)
    .bind(version_no)
    .bind(METADATA_ACTIVE_STATUS)
    .fetch_one(&mut **transaction)
    .await
    .map_err(sqlx_error)?;

    document_version_from_row(&row)
}

pub(crate) async fn bind_document_current_version_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: u64,
    organization_id: u64,
    timestamp_dialect: SqlTimestampDialect,
    document_id: u64,
    version_id: u64,
    version_no: u64,
) -> Result<(), DriveImportMetadataStoreError> {
    let tenant_id = to_i64("tenant_id", tenant_id)?;
    let organization_id = to_i64("organization_id", organization_id)?;
    let document_id = to_i64("document_id", document_id)?;
    let version_id = to_i64("version_id", version_id)?;
    let version_no = to_i64("version_no", version_no)?;
    let now = now_rfc3339()?;
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let query = format!(
        r#"
        UPDATE kb_document
        SET current_version_id = $1, updated_at = {updated_at_expr}, version = version + 1
        WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status = $6
          AND (current_version_id IS NULL OR current_version_id <= $1)
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(version_id)
        .bind(now)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(document_id)
        .bind(METADATA_ACTIVE_STATUS)
        .execute(&mut **transaction)
        .await
        .map_err(sqlx_error)?;
    let _ = version_no;
    Ok(())
}

pub(crate) async fn create_or_get_source_in_transaction(
    transaction: &mut Transaction<'_, Any>,
    tenant_id: u64,
    organization_id: u64,
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
    record: &CreateKnowledgeSourceRecord,
) -> Result<KnowledgeSource, DriveImportMetadataStoreError> {
    let tenant_id = to_i64("tenant_id", tenant_id)?;
    let organization_id = to_i64("organization_id", organization_id)?;
    let space_id = to_i64("space_id", record.space_id)?;
    let id = next_i64_id(id_generator).map_err(id_gen_error)?;
    let now = now_rfc3339()?;
    let source_type_value = record.source_type.as_str().to_string();
    let metadata_expr = timestamp_dialect.sql_json_expr("$10");
    let created_at_expr = timestamp_dialect.sql_timestamp_expr("$12");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$13");
    let query = format!(
        r#"
        INSERT INTO kb_source (
            id, uuid, tenant_id, organization_id, space_id, source_type, provider, drive_bucket, drive_prefix,
            metadata, status, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, {metadata_expr}, $11, {created_at_expr}, {updated_at_expr}, $14)
        ON CONFLICT DO NOTHING
        RETURNING id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(id)
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .bind(source_type_value)
        .bind(&record.provider)
        .bind(&record.drive_bucket)
        .bind(&record.drive_prefix)
        .bind(&record.connector_metadata_json)
        .bind(METADATA_ACTIVE_STATUS)
        .bind(now.clone())
        .bind(now)
        .bind(METADATA_INITIAL_VERSION)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(sqlx_error)?;

    if let Some(row) = row {
        return source_from_row(&row);
    }

    let row = sqlx::query(
        r#"
        SELECT id, space_id, source_type, provider, drive_bucket, drive_prefix, CAST(metadata AS TEXT) AS metadata
        FROM kb_source
        WHERE tenant_id = $1
          AND organization_id = $2
          AND space_id = $3
          AND source_type = $4
          AND COALESCE(provider, '') = COALESCE($5, '')
          AND COALESCE(drive_bucket, '') = COALESCE($6, '')
          AND COALESCE(drive_prefix, '') = COALESCE($7, '')
          AND status = $8
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(space_id)
    .bind(record.source_type.as_str())
    .bind(&record.provider)
    .bind(&record.drive_bucket)
    .bind(&record.drive_prefix)
    .bind(METADATA_ACTIVE_STATUS)
    .fetch_one(&mut **transaction)
    .await
    .map_err(sqlx_error)?;

    source_from_row(&row)
}

pub(crate) fn source_from_row(
    row: &AnyRow,
) -> Result<KnowledgeSource, DriveImportMetadataStoreError> {
    Ok(KnowledgeSource {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(sqlx_error)?)?,
        source_type: source_type_from_value(
            &row.try_get::<String, _>("source_type")
                .map_err(sqlx_error)?,
        )?,
        provider: row.try_get("provider").map_err(sqlx_error)?,
        drive_bucket: row.try_get("drive_bucket").map_err(sqlx_error)?,
        drive_prefix: row.try_get("drive_prefix").map_err(sqlx_error)?,
        connector_metadata_json: row.try_get("metadata").map_err(sqlx_error)?,
    })
}

fn source_type_from_value(
    value: &str,
) -> Result<KnowledgeSourceType, DriveImportMetadataStoreError> {
    match value {
        "upload" => Ok(KnowledgeSourceType::Upload),
        "drive_object" => Ok(KnowledgeSourceType::DriveObject),
        "drive_folder" => Ok(KnowledgeSourceType::DriveFolder),
        "url" => Ok(KnowledgeSourceType::Url),
        "connector" => Ok(KnowledgeSourceType::Connector),
        "api" => Ok(KnowledgeSourceType::Api),
        _ => Err(DriveImportMetadataStoreError::Internal(format!(
            "unknown knowledge source type: {value}"
        ))),
    }
}

pub(crate) fn validate_document_identity(
    record: &CreateKnowledgeDocumentRecord,
) -> Result<(), DriveImportMetadataStoreError> {
    if record.identity_scope == KnowledgeDocumentIdentityScope::SourceOnly
        && record.source_id.is_none()
    {
        return Err(DriveImportMetadataStoreError::InvalidRequest(
            "source_only document identity requires source_id".to_string(),
        ));
    }
    Ok(())
}
pub(crate) fn document_from_row(
    row: &AnyRow,
) -> Result<KnowledgeDocument, DriveImportMetadataStoreError> {
    let visibility_code: i64 = row.try_get("visibility").map_err(sqlx_error)?;
    let content_state_code: i64 = row.try_get("content_state").map_err(sqlx_error)?;
    let index_state_code: i64 = row.try_get("index_state").map_err(sqlx_error)?;
    Ok(KnowledgeDocument {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(sqlx_error)?)?,
        source_id: row
            .try_get::<Option<i64>, _>("source_id")
            .map_err(sqlx_error)?
            .map(|value| from_i64("source_id", value))
            .transpose()?,
        original_file_drive_node_id: row
            .try_get("original_file_drive_node_id")
            .map_err(sqlx_error)?,
        title: row.try_get("title").map_err(sqlx_error)?,
        mime_type: row.try_get("mime_type").map_err(sqlx_error)?,
        language: row.try_get("language").map_err(sqlx_error)?,
        current_version_id: row
            .try_get::<Option<i64>, _>("current_version_id")
            .map_err(sqlx_error)?
            .map(|value| from_i64("current_version_id", value))
            .transpose()?,
        visibility: document_visibility_from_code(visibility_code)?,
        content_state: document_state_from_code(content_state_code)?,
        index_state: version_state_from_code_for_document(index_state_code)?,
    })
}

pub(crate) fn document_version_from_row(
    row: &AnyRow,
) -> Result<KnowledgeDocumentVersion, DriveImportMetadataStoreError> {
    Ok(KnowledgeDocumentVersion {
        id: from_i64("id", row.try_get("id").map_err(sqlx_error)?)?,
        document_id: from_i64(
            "document_id",
            row.try_get("document_id").map_err(sqlx_error)?,
        )?,
        version_no: from_i64("version_no", row.try_get("version_no").map_err(sqlx_error)?)?,
        original_object_ref_id: from_i64(
            "original_object_ref_id",
            row.try_get("original_object_ref_id").map_err(sqlx_error)?,
        )?,
        checksum_sha256_hex: row.try_get("checksum_sha256_hex").map_err(sqlx_error)?,
        size_bytes: from_i64("size_bytes", row.try_get("size_bytes").map_err(sqlx_error)?)?,
        mime_type: row.try_get("mime_type").map_err(sqlx_error)?,
        parse_state: version_state_from_code(row.try_get("parse_state").map_err(sqlx_error)?)?,
        index_state: version_state_from_code(row.try_get("index_state").map_err(sqlx_error)?)?,
    })
}

pub(crate) fn object_ref_from_row(
    row: &AnyRow,
) -> Result<KnowledgeDriveObjectRef, DriveImportMetadataStoreError> {
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
pub(crate) fn version_state_code(state: KnowledgeDocumentVersionState) -> i64 {
    match state {
        KnowledgeDocumentVersionState::Pending => 0,
        KnowledgeDocumentVersionState::Running => 1,
        KnowledgeDocumentVersionState::Succeeded => 2,
        KnowledgeDocumentVersionState::Failed => 3,
    }
}

pub(crate) fn version_state_from_code_for_document(
    code: i64,
) -> Result<KnowledgeDocumentVersionState, DriveImportMetadataStoreError> {
    version_state_from_code(code)
}

pub(crate) fn version_state_from_code(
    code: i64,
) -> Result<KnowledgeDocumentVersionState, DriveImportMetadataStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentVersionState::Pending),
        1 => Ok(KnowledgeDocumentVersionState::Running),
        2 => Ok(KnowledgeDocumentVersionState::Succeeded),
        3 => Ok(KnowledgeDocumentVersionState::Failed),
        _ => Err(DriveImportMetadataStoreError::Internal(format!(
            "invalid document version state code: {code}"
        ))),
    }
}

pub(crate) fn document_visibility_code(visibility: KnowledgeDocumentVisibility) -> i64 {
    match visibility {
        KnowledgeDocumentVisibility::Private => 0,
        KnowledgeDocumentVisibility::Space => 1,
        KnowledgeDocumentVisibility::Organization => 2,
        KnowledgeDocumentVisibility::Public => 3,
    }
}

pub(crate) fn document_visibility_from_code(
    code: i64,
) -> Result<KnowledgeDocumentVisibility, DriveImportMetadataStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentVisibility::Private),
        1 => Ok(KnowledgeDocumentVisibility::Space),
        2 => Ok(KnowledgeDocumentVisibility::Organization),
        3 => Ok(KnowledgeDocumentVisibility::Public),
        _ => Err(DriveImportMetadataStoreError::Internal(format!(
            "invalid document visibility code: {code}"
        ))),
    }
}

pub(crate) fn document_state_code(state: KnowledgeDocumentState) -> i64 {
    match state {
        KnowledgeDocumentState::Draft => 0,
        KnowledgeDocumentState::Ready => 1,
        KnowledgeDocumentState::Archived => 2,
        KnowledgeDocumentState::Deleted => 3,
    }
}

pub(crate) fn document_state_from_code(
    code: i64,
) -> Result<KnowledgeDocumentState, DriveImportMetadataStoreError> {
    match code {
        0 => Ok(KnowledgeDocumentState::Draft),
        1 => Ok(KnowledgeDocumentState::Ready),
        2 => Ok(KnowledgeDocumentState::Archived),
        3 => Ok(KnowledgeDocumentState::Deleted),
        _ => Err(DriveImportMetadataStoreError::Internal(format!(
            "invalid document state code: {code}"
        ))),
    }
}

pub(crate) fn now_rfc3339() -> Result<String, DriveImportMetadataStoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| DriveImportMetadataStoreError::Internal(error.to_string()))
}

pub(crate) fn to_i64(field: &str, value: u64) -> Result<i64, DriveImportMetadataStoreError> {
    i64::try_from(value)
        .map_err(|_| DriveImportMetadataStoreError::Internal(format!("{field} is out of range")))
}

pub(crate) fn from_i64(field: &str, value: i64) -> Result<u64, DriveImportMetadataStoreError> {
    u64::try_from(value)
        .map_err(|_| DriveImportMetadataStoreError::Internal(format!("{field} is negative")))
}

pub(crate) fn id_gen_error(
    error: crate::KnowledgeIdGeneratorError,
) -> DriveImportMetadataStoreError {
    DriveImportMetadataStoreError::Internal(error.to_string())
}

pub(crate) fn sqlx_error(error: sqlx::Error) -> DriveImportMetadataStoreError {
    DriveImportMetadataStoreError::Internal(error.to_string())
}
