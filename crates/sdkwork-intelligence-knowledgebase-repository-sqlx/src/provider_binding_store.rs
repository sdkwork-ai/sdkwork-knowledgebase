use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderBindingStoreError,
    KnowledgeEngineProviderScope, RecordKnowledgeEngineProviderTestResult,
    ResolvedKnowledgeEngineProviderCredential,
};
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineCapability, KnowledgeEngineProviderErrorCategory,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    CreateKnowledgeEngineProviderBindingRequest,
    CreateKnowledgeEngineProviderCredentialReferenceRequest, KnowledgeEngineProviderBinding,
    KnowledgeEngineProviderBindingList, KnowledgeEngineProviderBindingState,
    KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderCredentialReferenceList,
    KnowledgeEngineProviderCredentialRotationState, ListKnowledgeEngineProviderBindingsRequest,
    ListKnowledgeEngineProviderCredentialReferencesRequest,
    RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    RotateKnowledgeEngineProviderCredentialReferenceRequest,
    UpdateKnowledgeEngineProviderBindingRequest,
};
use sdkwork_utils_rust::{sha256_hash, uuid, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE};
use sqlx::{AnyPool, Row};

use crate::db::sql_timestamp::{utc_sql_timestamp_text, SqlTimestampDialect};
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const BINDING_COLUMNS: &str = r#"
    id, uuid, tenant_id, organization_id, space_id, implementation_id,
    remote_resource_type, remote_resource_id, credential_reference_id,
    lifecycle_state, capability_snapshot::text AS capability_snapshot, capability_snapshot_version,
    TO_CHAR(last_tested_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS last_tested_at,
    TO_CHAR(activated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS activated_at,
    TO_CHAR(disabled_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS disabled_at,
    last_error_category, created_by, updated_by,
    TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS created_at,
    TO_CHAR(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS updated_at,
    version
"#;
const CREDENTIAL_COLUMNS: &str = r#"
    id, uuid, tenant_id, organization_id, implementation_id, display_name, rotation_state,
    TO_CHAR(last_rotated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS last_rotated_at,
    created_by, updated_by,
    TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS created_at,
    TO_CHAR(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS updated_at,
    version
"#;

#[derive(Clone)]
pub struct SqlxKnowledgeEngineProviderBindingStore {
    pool: AnyPool,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    dialect: SqlTimestampDialect,
}

impl SqlxKnowledgeEngineProviderBindingStore {
    pub fn new(pool: AnyPool) -> Self {
        Self::with_id_generator(pool, default_knowledge_id_generator())
    }

    pub fn with_id_generator(pool: AnyPool, id_generator: Arc<dyn KnowledgeIdGenerator>) -> Self {
        Self {
            pool,
            id_generator,
            dialect: SqlTimestampDialect::default(),
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }

    async fn require_space(
        &self,
        scope: KnowledgeEngineProviderScope,
        space_id: u64,
    ) -> Result<(), KnowledgeEngineProviderBindingStoreError> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM kb_space WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        )
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(to_i64("space_id", space_id)?)
        .fetch_optional(&self.pool)
        .await
        .map_err(sql_error)?;
        if exists.is_none() {
            return Err(KnowledgeEngineProviderBindingStoreError::InvalidRequest(
                format!("space_id={space_id} is outside the Provider scope or inactive"),
            ));
        }
        Ok(())
    }

    async fn require_credential_match(
        &self,
        scope: KnowledgeEngineProviderScope,
        credential_reference_id: Option<u64>,
        implementation_id: &str,
    ) -> Result<(), KnowledgeEngineProviderBindingStoreError> {
        let Some(credential_reference_id) = credential_reference_id else {
            return Ok(());
        };
        self.resolve_credential_reference(scope, credential_reference_id, implementation_id)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl KnowledgeEngineProviderBindingStore for SqlxKnowledgeEngineProviderBindingStore {
    async fn create_credential_reference(
        &self,
        scope: KnowledgeEngineProviderScope,
        actor_id: &str,
        request: CreateKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        require_text("implementation_id", &request.implementation_id, 128)?;
        require_text("display_name", &request.display_name, 256)?;
        require_text("reference_locator", &request.reference_locator, 2_048)?;

        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let uuid = uuid();
        let now = now()?;
        let fingerprint = sha256_hash(request.reference_locator.as_bytes());
        let created_at = self.dialect.sql_timestamp_expr("$11");
        let updated_at = self.dialect.sql_timestamp_expr("$12");
        let query = format!(
            r#"
            INSERT INTO kb_provider_credential_reference (
                id, uuid, tenant_id, organization_id, implementation_id, display_name,
                reference_locator, reference_fingerprint, rotation_state, created_by, updated_by,
                created_at, updated_at, status, version
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, 'current', $9, $10,
                {created_at}, {updated_at}, 1, 0
            )
            RETURNING {CREDENTIAL_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(uuid)
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(request.implementation_id.trim())
            .bind(request.display_name.trim())
            .bind(request.reference_locator.trim())
            .bind(fingerprint)
            .bind(actor_id.trim())
            .bind(actor_id.trim())
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(binding_sql_error)?;
        credential_from_row(&row)
    }

    async fn resolve_credential_reference(
        &self,
        scope: KnowledgeEngineProviderScope,
        credential_reference_id: u64,
        implementation_id: &str,
    ) -> Result<ResolvedKnowledgeEngineProviderCredential, KnowledgeEngineProviderBindingStoreError>
    {
        validate_scope(scope)?;
        require_text("implementation_id", implementation_id, 128)?;
        let row = sqlx::query(
            r#"
            SELECT id, implementation_id, reference_locator, version
            FROM kb_provider_credential_reference
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND implementation_id = $4 AND rotation_state <> 'revoked' AND status = 1
            "#,
        )
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(to_i64("credential_reference_id", credential_reference_id)?)
        .bind(implementation_id.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(sql_error)?
        .ok_or(
            KnowledgeEngineProviderBindingStoreError::CredentialUnavailable(
                credential_reference_id,
            ),
        )?;

        Ok(ResolvedKnowledgeEngineProviderCredential {
            credential_reference_id: from_i64("id", row.try_get("id").map_err(sqlx_row_error)?)?,
            implementation_id: row.try_get("implementation_id").map_err(sqlx_row_error)?,
            reference_locator: row.try_get("reference_locator").map_err(sqlx_row_error)?,
            version: from_i64("version", row.try_get("version").map_err(sqlx_row_error)?)?,
        })
    }

    async fn get_credential_reference(
        &self,
        scope: KnowledgeEngineProviderScope,
        credential_reference_id: u64,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        validate_scope(scope)?;
        let query = format!(
            "SELECT {CREDENTIAL_COLUMNS} FROM kb_provider_credential_reference WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("credential_reference_id", credential_reference_id)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?
            .ok_or(KnowledgeEngineProviderBindingStoreError::NotFound(
                credential_reference_id,
            ))?;
        credential_from_row(&row)
    }

    async fn list_credential_references(
        &self,
        scope: KnowledgeEngineProviderScope,
        request: ListKnowledgeEngineProviderCredentialReferencesRequest,
    ) -> Result<
        KnowledgeEngineProviderCredentialReferenceList,
        KnowledgeEngineProviderBindingStoreError,
    > {
        validate_scope(scope)?;
        if let Some(value) = request.implementation_id.as_deref() {
            require_text("implementation_id", value, 128)?;
        }
        let page_size = request
            .page_size
            .unwrap_or(DEFAULT_LIST_PAGE_SIZE as u32)
            .clamp(1, MAX_LIST_PAGE_SIZE as u32);
        let fetch_limit = i64::from(page_size) + 1;
        let cursor = parse_cursor(request.cursor.as_deref())?;
        let implementation_id = request.implementation_id.as_deref().map(str::trim);
        let rotation_state = request.rotation_state.map(|state| state.as_str());
        let query = format!(
            r#"
            SELECT {CREDENTIAL_COLUMNS}
            FROM kb_provider_credential_reference
            WHERE tenant_id = $1 AND organization_id = $2 AND status = 1
              AND ($3 IS NULL OR implementation_id = $3)
              AND ($4 IS NULL OR rotation_state = $4)
              AND ($5 IS NULL OR id < $5)
            ORDER BY id DESC
            LIMIT $6
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(implementation_id)
            .bind(rotation_state)
            .bind(cursor)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;
        let has_more = rows.len() > page_size as usize;
        let items = rows
            .iter()
            .take(page_size as usize)
            .map(credential_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.id.to_string()))
            .flatten();
        Ok(KnowledgeEngineProviderCredentialReferenceList { items, next_cursor })
    }

    async fn rotate_credential_reference(
        &self,
        scope: KnowledgeEngineProviderScope,
        credential_reference_id: u64,
        actor_id: &str,
        request: RotateKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        require_text("reference_locator", &request.reference_locator, 2_048)?;
        let now = now()?;
        let fingerprint = sha256_hash(request.reference_locator.as_bytes());
        let last_rotated_at = self.dialect.sql_timestamp_expr("$8");
        let updated_at = self.dialect.sql_timestamp_expr("$9");
        let query = format!(
            r#"
            UPDATE kb_provider_credential_reference
            SET reference_locator = $4,
                reference_fingerprint = $5,
                rotation_state = 'current',
                last_rotated_at = {last_rotated_at},
                updated_by = $6,
                updated_at = {updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND version = $7 AND rotation_state <> 'revoked' AND status = 1
            RETURNING {CREDENTIAL_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("credential_reference_id", credential_reference_id)?)
            .bind(request.reference_locator.trim())
            .bind(fingerprint)
            .bind(actor_id.trim())
            .bind(to_i64("expected_version", request.expected_version)?)
            .bind(&now)
            .bind(&now)
            .fetch_optional(&self.pool)
            .await
            .map_err(binding_sql_error)?;
        match row {
            Some(row) => credential_from_row(&row),
            None => credential_write_conflict(self, scope, credential_reference_id).await,
        }
    }

    async fn revoke_credential_reference(
        &self,
        scope: KnowledgeEngineProviderScope,
        credential_reference_id: u64,
        actor_id: &str,
        request: RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError>
    {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        let now = now()?;
        let updated_at = self.dialect.sql_timestamp_expr("$6");
        let query = format!(
            r#"
            UPDATE kb_provider_credential_reference
            SET rotation_state = 'revoked',
                updated_by = $4,
                updated_at = {updated_at},
                version = version + 1
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND version = $5 AND rotation_state <> 'revoked' AND status = 1
            RETURNING {CREDENTIAL_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("credential_reference_id", credential_reference_id)?)
            .bind(actor_id.trim())
            .bind(to_i64("expected_version", request.expected_version)?)
            .bind(&now)
            .fetch_optional(&self.pool)
            .await
            .map_err(binding_sql_error)?;
        match row {
            Some(row) => credential_from_row(&row),
            None => credential_write_conflict(self, scope, credential_reference_id).await,
        }
    }

    async fn create_binding(
        &self,
        scope: KnowledgeEngineProviderScope,
        actor_id: &str,
        request: CreateKnowledgeEngineProviderBindingRequest,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        require_text("implementation_id", &request.implementation_id, 128)?;
        require_text("remote_resource_type", &request.remote_resource_type, 64)?;
        require_text("remote_resource_id", &request.remote_resource_id, 512)?;
        self.require_space(scope, request.space_id).await?;
        self.require_credential_match(
            scope,
            request.credential_reference_id,
            &request.implementation_id,
        )
        .await?;

        let id = next_i64_id(&self.id_generator).map_err(id_error)?;
        let uuid = uuid();
        let now = now()?;
        let capabilities_json = "[]";
        let capabilities = self.dialect.sql_json_expr("$10");
        let created_at = self.dialect.sql_timestamp_expr("$13");
        let updated_at = self.dialect.sql_timestamp_expr("$14");
        let query = format!(
            r#"
            INSERT INTO kb_provider_binding (
                id, uuid, tenant_id, organization_id, space_id, implementation_id,
                remote_resource_type, remote_resource_id, credential_reference_id,
                capability_snapshot, lifecycle_state, created_by, updated_by,
                created_at, updated_at, status, version
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                {capabilities}, 'draft', $11, $12, {created_at}, {updated_at}, 1, 0
            )
            RETURNING {BINDING_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(id)
            .bind(uuid)
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("space_id", request.space_id)?)
            .bind(request.implementation_id.trim())
            .bind(request.remote_resource_type.trim())
            .bind(request.remote_resource_id.trim())
            .bind(optional_to_i64(
                "credential_reference_id",
                request.credential_reference_id,
            )?)
            .bind(capabilities_json)
            .bind(actor_id.trim())
            .bind(actor_id.trim())
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(binding_sql_error)?;
        binding_from_row(&row)
    }

    async fn get_binding(
        &self,
        scope: KnowledgeEngineProviderScope,
        binding_id: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        validate_scope(scope)?;
        let query = format!(
            "SELECT {BINDING_COLUMNS} FROM kb_provider_binding WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1",
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("binding_id", binding_id)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?
            .ok_or(KnowledgeEngineProviderBindingStoreError::NotFound(
                binding_id,
            ))?;
        binding_from_row(&row)
    }

    async fn get_active_binding_for_space(
        &self,
        scope: KnowledgeEngineProviderScope,
        space_id: u64,
    ) -> Result<Option<KnowledgeEngineProviderBinding>, KnowledgeEngineProviderBindingStoreError>
    {
        validate_scope(scope)?;
        let query = format!(
            "SELECT {BINDING_COLUMNS} FROM kb_provider_binding WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND lifecycle_state = 'active' AND status = 1",
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("space_id", space_id)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?;
        row.as_ref().map(binding_from_row).transpose()
    }

    async fn list_bindings(
        &self,
        scope: KnowledgeEngineProviderScope,
        request: ListKnowledgeEngineProviderBindingsRequest,
    ) -> Result<KnowledgeEngineProviderBindingList, KnowledgeEngineProviderBindingStoreError> {
        validate_scope(scope)?;
        let page_size = request
            .page_size
            .unwrap_or(DEFAULT_LIST_PAGE_SIZE as u32)
            .clamp(1, MAX_LIST_PAGE_SIZE as u32);
        let fetch_limit = i64::from(page_size) + 1;
        let cursor = parse_cursor(request.cursor.as_deref())?;
        let space_id = optional_to_i64("space_id", request.space_id)?;
        let lifecycle_state = request.lifecycle_state.map(|state| state.as_str());
        let query = format!(
            r#"
            SELECT {BINDING_COLUMNS}
            FROM kb_provider_binding
            WHERE tenant_id = $1 AND organization_id = $2 AND status = 1
              AND ($3 IS NULL OR space_id = $3)
              AND ($4 IS NULL OR lifecycle_state = $4)
              AND ($5 IS NULL OR id < $5)
            ORDER BY id DESC
            LIMIT $6
            "#,
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(space_id)
            .bind(lifecycle_state)
            .bind(cursor)
            .bind(fetch_limit)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;
        let has_more = rows.len() > page_size as usize;
        let items = rows
            .iter()
            .take(page_size as usize)
            .map(binding_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.id.to_string()))
            .flatten();
        Ok(KnowledgeEngineProviderBindingList { items, next_cursor })
    }

    async fn update_draft_binding(
        &self,
        scope: KnowledgeEngineProviderScope,
        binding_id: u64,
        actor_id: &str,
        request: UpdateKnowledgeEngineProviderBindingRequest,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        if request.clear_credential_reference && request.credential_reference_id.is_some() {
            return Err(KnowledgeEngineProviderBindingStoreError::InvalidRequest(
                "credential_reference_id and clear_credential_reference are mutually exclusive"
                    .to_string(),
            ));
        }
        if let Some(value) = request.remote_resource_type.as_deref() {
            require_text("remote_resource_type", value, 64)?;
        }
        if let Some(value) = request.remote_resource_id.as_deref() {
            require_text("remote_resource_id", value, 512)?;
        }
        let current = self.get_binding(scope, binding_id).await?;
        self.require_credential_match(
            scope,
            request.credential_reference_id,
            &current.implementation_id,
        )
        .await?;
        let credential_reference_id = if request.clear_credential_reference {
            None
        } else {
            request
                .credential_reference_id
                .or(current.credential_reference_id)
        };
        let now = now()?;
        let updated_at = self.dialect.sql_timestamp_expr("$4");
        let query = format!(
            r#"
            UPDATE kb_provider_binding
            SET remote_resource_type = COALESCE($1, remote_resource_type),
                remote_resource_id = COALESCE($2, remote_resource_id),
                credential_reference_id = $3,
                updated_at = {updated_at}, updated_by = $5, version = version + 1
            WHERE tenant_id = $6 AND organization_id = $7 AND id = $8
              AND lifecycle_state = 'draft' AND version = $9 AND status = 1
            RETURNING {BINDING_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(request.remote_resource_type.as_deref().map(str::trim))
            .bind(request.remote_resource_id.as_deref().map(str::trim))
            .bind(optional_to_i64(
                "credential_reference_id",
                credential_reference_id,
            )?)
            .bind(&now)
            .bind(actor_id.trim())
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("binding_id", binding_id)?)
            .bind(to_i64("expected_version", request.expected_version)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(binding_sql_error)?
            .ok_or_else(|| version_or_lifecycle_conflict(binding_id))?;
        binding_from_row(&row)
    }

    async fn begin_binding_test(
        &self,
        scope: KnowledgeEngineProviderScope,
        binding_id: u64,
        actor_id: &str,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        transition_binding(
            self,
            scope,
            binding_id,
            actor_id,
            expected_version,
            "testing",
            "lifecycle_state IN ('draft', 'failed', 'degraded')",
        )
        .await
    }

    async fn record_binding_test_result(
        &self,
        scope: KnowledgeEngineProviderScope,
        binding_id: u64,
        result: RecordKnowledgeEngineProviderTestResult,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        validate_scope(scope)?;
        require_text("updated_by", &result.updated_by, 128)?;
        let now = now()?;
        let capabilities_json = serde_json::to_string(&result.capabilities).map_err(|error| {
            KnowledgeEngineProviderBindingStoreError::Internal(error.to_string())
        })?;
        let state = if result.error_category.is_some() {
            "failed"
        } else {
            "testing"
        };
        let error_category = result.error_category.map(error_category_str);
        let capabilities = self.dialect.sql_json_expr("$1");
        let tested_at = self.dialect.sql_timestamp_expr("$4");
        let updated_at = self.dialect.sql_timestamp_expr("$5");
        let query = format!(
            r#"
            UPDATE kb_provider_binding
            SET capability_snapshot = {capabilities},
                capability_snapshot_version = capability_snapshot_version + 1,
                lifecycle_state = $2, last_error_category = $3,
                last_tested_at = {tested_at}, updated_at = {updated_at},
                updated_by = $6, version = version + 1
            WHERE tenant_id = $7 AND organization_id = $8 AND id = $9
              AND lifecycle_state = 'testing' AND version = $10 AND status = 1
            RETURNING {BINDING_COLUMNS}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(capabilities_json)
            .bind(state)
            .bind(error_category)
            .bind(&now)
            .bind(&now)
            .bind(result.updated_by.trim())
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("binding_id", binding_id)?)
            .bind(to_i64("expected_version", result.expected_version)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(binding_sql_error)?
            .ok_or_else(|| version_or_lifecycle_conflict(binding_id))?;
        binding_from_row(&row)
    }

    async fn activate_binding(
        &self,
        scope: KnowledgeEngineProviderScope,
        binding_id: u64,
        actor_id: &str,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        let now = now()?;
        let mut transaction = self.pool.begin().await.map_err(sql_error)?;
        let target = sqlx::query(
            r#"
            SELECT space_id, capability_snapshot::text AS capability_snapshot
            FROM kb_provider_binding
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
              AND lifecycle_state = 'testing' AND last_tested_at IS NOT NULL
              AND version = $4 AND status = 1
            "#,
        )
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(to_i64("binding_id", binding_id)?)
        .bind(to_i64("expected_version", expected_version)?)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(sql_error)?
        .ok_or_else(|| version_or_lifecycle_conflict(binding_id))?;
        let space_id: i64 = target.try_get("space_id").map_err(sqlx_row_error)?;
        let capabilities_json: String = target
            .try_get("capability_snapshot")
            .map_err(sqlx_row_error)?;
        let capabilities: Vec<KnowledgeEngineCapability> = serde_json::from_str(&capabilities_json)
            .map_err(|error| {
                KnowledgeEngineProviderBindingStoreError::Internal(error.to_string())
            })?;
        if !capabilities.contains(&KnowledgeEngineCapability::Health)
            || !capabilities.contains(&KnowledgeEngineCapability::Search)
        {
            return Err(KnowledgeEngineProviderBindingStoreError::InvalidLifecycle(
                "activation requires tested health and search capabilities".to_string(),
            ));
        }

        let deactivated_at = self.dialect.sql_timestamp_expr("$1");
        let deactivate_query = format!(
            r#"
            UPDATE kb_provider_binding
            SET lifecycle_state = 'disabled', disabled_at = {deactivated_at},
                updated_at = {deactivated_at}, updated_by = $2, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND space_id = $5
              AND id <> $6 AND lifecycle_state = 'active' AND status = 1
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(deactivate_query.as_str()))
            .bind(&now)
            .bind(actor_id.trim())
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(space_id)
            .bind(to_i64("binding_id", binding_id)?)
            .execute(&mut *transaction)
            .await
            .map_err(binding_sql_error)?;

        let activated_at = self.dialect.sql_timestamp_expr("$1");
        let activate_query = format!(
            r#"
            UPDATE kb_provider_binding
            SET lifecycle_state = 'active', activated_at = {activated_at},
                disabled_at = NULL, last_error_category = NULL,
                updated_at = {activated_at}, updated_by = $2, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5
              AND lifecycle_state = 'testing' AND version = $6 AND status = 1
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(activate_query.as_str()))
            .bind(&now)
            .bind(actor_id.trim())
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("binding_id", binding_id)?)
            .bind(to_i64("expected_version", expected_version)?)
            .execute(&mut *transaction)
            .await
            .map_err(binding_sql_error)?;
        if updated.rows_affected() != 1 {
            return Err(version_or_lifecycle_conflict(binding_id));
        }
        transaction.commit().await.map_err(sql_error)?;
        self.get_binding(scope, binding_id).await
    }

    async fn disable_binding(
        &self,
        scope: KnowledgeEngineProviderScope,
        binding_id: u64,
        actor_id: &str,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
        transition_binding(
            self,
            scope,
            binding_id,
            actor_id,
            expected_version,
            "disabled",
            "lifecycle_state <> 'disabled'",
        )
        .await
    }
}

async fn transition_binding(
    store: &SqlxKnowledgeEngineProviderBindingStore,
    scope: KnowledgeEngineProviderScope,
    binding_id: u64,
    actor_id: &str,
    expected_version: u64,
    target_state: &str,
    source_predicate: &str,
) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
    validate_scope(scope)?;
    require_text("actor_id", actor_id, 128)?;
    let now = now()?;
    let updated_at = store.dialect.sql_timestamp_expr("$2");
    let disabled_at = if target_state == "disabled" {
        updated_at.clone()
    } else {
        "NULL".to_string()
    };
    let query = format!(
        r#"
        UPDATE kb_provider_binding
        SET lifecycle_state = $1, updated_at = {updated_at},
            disabled_at = {disabled_at}, updated_by = $3, version = version + 1
        WHERE tenant_id = $4 AND organization_id = $5 AND id = $6
          AND {source_predicate} AND version = $7 AND status = 1
        RETURNING {BINDING_COLUMNS}
        "#,
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(target_state)
        .bind(&now)
        .bind(actor_id.trim())
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(to_i64("binding_id", binding_id)?)
        .bind(to_i64("expected_version", expected_version)?)
        .fetch_optional(&store.pool)
        .await
        .map_err(binding_sql_error)?
        .ok_or_else(|| version_or_lifecycle_conflict(binding_id))?;
    binding_from_row(&row)
}

fn binding_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingStoreError> {
    let lifecycle_state: String = row.try_get("lifecycle_state").map_err(sqlx_row_error)?;
    let capabilities_json: String = row.try_get("capability_snapshot").map_err(sqlx_row_error)?;
    let error_category: Option<String> =
        row.try_get("last_error_category").map_err(sqlx_row_error)?;
    Ok(KnowledgeEngineProviderBinding {
        id: from_i64("id", row.try_get("id").map_err(sqlx_row_error)?)?,
        uuid: row.try_get("uuid").map_err(sqlx_row_error)?,
        tenant_id: from_i64(
            "tenant_id",
            row.try_get("tenant_id").map_err(sqlx_row_error)?,
        )?,
        organization_id: from_i64(
            "organization_id",
            row.try_get("organization_id").map_err(sqlx_row_error)?,
        )?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(sqlx_row_error)?)?,
        implementation_id: row.try_get("implementation_id").map_err(sqlx_row_error)?,
        remote_resource_type: row
            .try_get("remote_resource_type")
            .map_err(sqlx_row_error)?,
        remote_resource_id: row.try_get("remote_resource_id").map_err(sqlx_row_error)?,
        credential_reference_id: optional_from_i64(
            "credential_reference_id",
            row.try_get("credential_reference_id")
                .map_err(sqlx_row_error)?,
        )?,
        lifecycle_state: KnowledgeEngineProviderBindingState::from_str(&lifecycle_state).map_err(
            |_| {
                KnowledgeEngineProviderBindingStoreError::Internal(format!(
                    "unknown provider binding lifecycle_state={lifecycle_state}"
                ))
            },
        )?,
        capability_snapshot: serde_json::from_str(&capabilities_json).map_err(|error| {
            KnowledgeEngineProviderBindingStoreError::Internal(format!(
                "invalid provider capability snapshot: {error}"
            ))
        })?,
        capability_snapshot_version: from_i64(
            "capability_snapshot_version",
            row.try_get("capability_snapshot_version")
                .map_err(sqlx_row_error)?,
        )?,
        last_tested_at: row.try_get("last_tested_at").map_err(sqlx_row_error)?,
        activated_at: row.try_get("activated_at").map_err(sqlx_row_error)?,
        disabled_at: row.try_get("disabled_at").map_err(sqlx_row_error)?,
        last_error_category: error_category
            .as_deref()
            .map(parse_error_category)
            .transpose()?,
        created_by: row.try_get("created_by").map_err(sqlx_row_error)?,
        updated_by: row.try_get("updated_by").map_err(sqlx_row_error)?,
        created_at: row.try_get("created_at").map_err(sqlx_row_error)?,
        updated_at: row.try_get("updated_at").map_err(sqlx_row_error)?,
        version: from_i64("version", row.try_get("version").map_err(sqlx_row_error)?)?,
    })
}

fn credential_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError> {
    let rotation_state: String = row.try_get("rotation_state").map_err(sqlx_row_error)?;
    Ok(KnowledgeEngineProviderCredentialReference {
        id: from_i64("id", row.try_get("id").map_err(sqlx_row_error)?)?,
        uuid: row.try_get("uuid").map_err(sqlx_row_error)?,
        tenant_id: from_i64(
            "tenant_id",
            row.try_get("tenant_id").map_err(sqlx_row_error)?,
        )?,
        organization_id: from_i64(
            "organization_id",
            row.try_get("organization_id").map_err(sqlx_row_error)?,
        )?,
        implementation_id: row.try_get("implementation_id").map_err(sqlx_row_error)?,
        display_name: row.try_get("display_name").map_err(sqlx_row_error)?,
        rotation_state: KnowledgeEngineProviderCredentialRotationState::from_str(&rotation_state)
            .map_err(|_| {
            KnowledgeEngineProviderBindingStoreError::Internal(format!(
                "unknown provider credential rotation_state={rotation_state}"
            ))
        })?,
        last_rotated_at: row.try_get("last_rotated_at").map_err(sqlx_row_error)?,
        created_by: row.try_get("created_by").map_err(sqlx_row_error)?,
        updated_by: row.try_get("updated_by").map_err(sqlx_row_error)?,
        created_at: row.try_get("created_at").map_err(sqlx_row_error)?,
        updated_at: row.try_get("updated_at").map_err(sqlx_row_error)?,
        version: from_i64("version", row.try_get("version").map_err(sqlx_row_error)?)?,
    })
}

fn error_category_str(category: KnowledgeEngineProviderErrorCategory) -> &'static str {
    match category {
        KnowledgeEngineProviderErrorCategory::Authentication => "authentication",
        KnowledgeEngineProviderErrorCategory::PermissionDenied => "permission_denied",
        KnowledgeEngineProviderErrorCategory::RateLimited => "rate_limited",
        KnowledgeEngineProviderErrorCategory::Timeout => "timeout",
        KnowledgeEngineProviderErrorCategory::Unavailable => "unavailable",
        KnowledgeEngineProviderErrorCategory::CircuitOpen => "circuit_open",
        KnowledgeEngineProviderErrorCategory::BulkheadSaturated => "bulkhead_saturated",
        KnowledgeEngineProviderErrorCategory::InvalidResponse => "invalid_response",
        KnowledgeEngineProviderErrorCategory::ResponseTooLarge => "response_too_large",
        KnowledgeEngineProviderErrorCategory::InvalidTarget => "invalid_target",
        KnowledgeEngineProviderErrorCategory::NotFound => "not_found",
        KnowledgeEngineProviderErrorCategory::Validation => "validation",
        KnowledgeEngineProviderErrorCategory::Unsupported => "unsupported",
        KnowledgeEngineProviderErrorCategory::Internal => "internal",
    }
}

fn parse_error_category(
    value: &str,
) -> Result<KnowledgeEngineProviderErrorCategory, KnowledgeEngineProviderBindingStoreError> {
    match value {
        "authentication" => Ok(KnowledgeEngineProviderErrorCategory::Authentication),
        "permission_denied" => Ok(KnowledgeEngineProviderErrorCategory::PermissionDenied),
        "rate_limited" => Ok(KnowledgeEngineProviderErrorCategory::RateLimited),
        "timeout" => Ok(KnowledgeEngineProviderErrorCategory::Timeout),
        "unavailable" => Ok(KnowledgeEngineProviderErrorCategory::Unavailable),
        "circuit_open" => Ok(KnowledgeEngineProviderErrorCategory::CircuitOpen),
        "bulkhead_saturated" => Ok(KnowledgeEngineProviderErrorCategory::BulkheadSaturated),
        "invalid_response" => Ok(KnowledgeEngineProviderErrorCategory::InvalidResponse),
        "response_too_large" => Ok(KnowledgeEngineProviderErrorCategory::ResponseTooLarge),
        "invalid_target" => Ok(KnowledgeEngineProviderErrorCategory::InvalidTarget),
        "not_found" => Ok(KnowledgeEngineProviderErrorCategory::NotFound),
        "validation" => Ok(KnowledgeEngineProviderErrorCategory::Validation),
        "unsupported" => Ok(KnowledgeEngineProviderErrorCategory::Unsupported),
        "internal" => Ok(KnowledgeEngineProviderErrorCategory::Internal),
        _ => Err(KnowledgeEngineProviderBindingStoreError::Internal(format!(
            "unknown provider error category={value}"
        ))),
    }
}

fn validate_scope(
    scope: KnowledgeEngineProviderScope,
) -> Result<(), KnowledgeEngineProviderBindingStoreError> {
    if scope.tenant_id == 0 || scope.tenant_id > i64::MAX as u64 {
        return Err(KnowledgeEngineProviderBindingStoreError::InvalidRequest(
            "tenant_id must be a positive signed 64-bit integer".to_string(),
        ));
    }
    if scope.organization_id > i64::MAX as u64 {
        return Err(KnowledgeEngineProviderBindingStoreError::InvalidRequest(
            "organization_id exceeds the signed 64-bit range".to_string(),
        ));
    }
    Ok(())
}

fn require_text(
    field: &str,
    value: &str,
    max_length: usize,
) -> Result<(), KnowledgeEngineProviderBindingStoreError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_length {
        return Err(KnowledgeEngineProviderBindingStoreError::InvalidRequest(
            format!("{field} must contain between 1 and {max_length} characters"),
        ));
    }
    Ok(())
}

fn parse_cursor(
    cursor: Option<&str>,
) -> Result<Option<i64>, KnowledgeEngineProviderBindingStoreError> {
    cursor
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                KnowledgeEngineProviderBindingStoreError::InvalidRequest(
                    "cursor must be a valid provider binding id".to_string(),
                )
            })
        })
        .transpose()
}

fn now() -> Result<String, KnowledgeEngineProviderBindingStoreError> {
    utc_sql_timestamp_text().map_err(KnowledgeEngineProviderBindingStoreError::Internal)
}

fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeEngineProviderBindingStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeEngineProviderBindingStoreError::InvalidRequest(format!(
            "{field} exceeds the signed 64-bit range"
        ))
    })
}

fn optional_to_i64(
    field: &str,
    value: Option<u64>,
) -> Result<Option<i64>, KnowledgeEngineProviderBindingStoreError> {
    value.map(|value| to_i64(field, value)).transpose()
}

fn from_i64(field: &str, value: i64) -> Result<u64, KnowledgeEngineProviderBindingStoreError> {
    u64::try_from(value).map_err(|_| {
        KnowledgeEngineProviderBindingStoreError::Internal(format!("{field} is negative"))
    })
}

fn optional_from_i64(
    field: &str,
    value: Option<i64>,
) -> Result<Option<u64>, KnowledgeEngineProviderBindingStoreError> {
    value.map(|value| from_i64(field, value)).transpose()
}

fn version_or_lifecycle_conflict(binding_id: u64) -> KnowledgeEngineProviderBindingStoreError {
    KnowledgeEngineProviderBindingStoreError::Conflict(format!(
        "binding_id={binding_id} version or lifecycle precondition failed"
    ))
}

async fn credential_write_conflict(
    store: &SqlxKnowledgeEngineProviderBindingStore,
    scope: KnowledgeEngineProviderScope,
    credential_reference_id: u64,
) -> Result<KnowledgeEngineProviderCredentialReference, KnowledgeEngineProviderBindingStoreError> {
    let current = store
        .get_credential_reference(scope, credential_reference_id)
        .await?;
    if current.rotation_state == KnowledgeEngineProviderCredentialRotationState::Revoked {
        return Err(KnowledgeEngineProviderBindingStoreError::InvalidLifecycle(
            format!("credential_reference_id={credential_reference_id} is revoked"),
        ));
    }
    Err(KnowledgeEngineProviderBindingStoreError::Conflict(format!(
        "credential_reference_id={credential_reference_id} version precondition failed"
    )))
}

fn id_error(error: crate::KnowledgeIdGeneratorError) -> KnowledgeEngineProviderBindingStoreError {
    KnowledgeEngineProviderBindingStoreError::Internal(error.to_string())
}

fn sql_error(error: sqlx::Error) -> KnowledgeEngineProviderBindingStoreError {
    KnowledgeEngineProviderBindingStoreError::Internal(error.to_string())
}

fn binding_sql_error(error: sqlx::Error) -> KnowledgeEngineProviderBindingStoreError {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("unique") {
        KnowledgeEngineProviderBindingStoreError::Conflict(
            "provider binding uniqueness constraint failed".to_string(),
        )
    } else if message.to_ascii_lowercase().contains("foreign key") {
        KnowledgeEngineProviderBindingStoreError::InvalidRequest(
            "provider binding scope reference is invalid".to_string(),
        )
    } else {
        KnowledgeEngineProviderBindingStoreError::Internal(message)
    }
}

fn sqlx_row_error(error: sqlx::Error) -> KnowledgeEngineProviderBindingStoreError {
    KnowledgeEngineProviderBindingStoreError::Internal(error.to_string())
}
