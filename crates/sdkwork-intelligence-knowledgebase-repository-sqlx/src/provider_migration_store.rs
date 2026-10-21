use std::{str::FromStr, sync::Arc, time::Duration};

use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::{
    knowledge_provider_binding_store::KnowledgeEngineProviderScope,
    knowledge_provider_migration_store::{
        AdvanceClaimedKnowledgeEngineProviderMigration, ClaimedKnowledgeEngineProviderMigration,
        CutoverClaimedKnowledgeEngineProviderMigration, KnowledgeEngineProviderMigrationStore,
        KnowledgeEngineProviderMigrationStoreError,
    },
};
use sdkwork_knowledgebase_contract::{
    knowledge_engine::KnowledgeEngineProviderErrorCategory,
    provider_binding::{
        CreateKnowledgeEngineProviderMigrationOperationRequest,
        KnowledgeEngineProviderBindingState, KnowledgeEngineProviderMigrationOperation,
        KnowledgeEngineProviderMigrationOperationList, KnowledgeEngineProviderMigrationState,
        ListKnowledgeEngineProviderMigrationOperationsRequest,
    },
};
use sdkwork_utils_rust::{is_blank, uuid, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE};
use serde_json::{json, Value};
use sqlx::{Any, AnyPool, Row, Transaction};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    db::sql_timestamp::{utc_sql_timestamp_text, SqlTimestampDialect},
    id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator},
};

const MIGRATION_COLUMNS: &str = r#"
    id, uuid, tenant_id, organization_id, space_id, source_binding_id,
    target_binding_id, operation_state, requested_by, attempt_count,
    TO_CHAR(cutover_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS cutover_at,
    TO_CHAR(observation_until AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS observation_until,
    TO_CHAR(completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS completed_at,
    last_error_category,
    TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS created_at,
    TO_CHAR(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS updated_at,
    version
"#;

#[derive(Clone)]
pub struct SqlxKnowledgeEngineProviderMigrationStore {
    pool: AnyPool,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    dialect: SqlTimestampDialect,
}

impl SqlxKnowledgeEngineProviderMigrationStore {
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

    async fn fetch_operation(
        &self,
        scope: KnowledgeEngineProviderScope,
        operation_id: u64,
    ) -> Result<
        Option<KnowledgeEngineProviderMigrationOperation>,
        KnowledgeEngineProviderMigrationStoreError,
    > {
        let sql = format!(
            "SELECT {MIGRATION_COLUMNS} FROM kb_provider_migration_operation \
             WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("operation_id", operation_id)?)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?
            .map(map_operation)
            .transpose()
    }
}

#[async_trait]
impl KnowledgeEngineProviderMigrationStore for SqlxKnowledgeEngineProviderMigrationStore {
    async fn create_operation(
        &self,
        scope: KnowledgeEngineProviderScope,
        space_id: u64,
        actor_id: &str,
        request: CreateKnowledgeEngineProviderMigrationOperationRequest,
    ) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError>
    {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        require_text("idempotency_key", &request.idempotency_key, 128)?;
        if request.source_binding_id == request.target_binding_id {
            return Err(KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
                "source and target bindings must differ".to_string(),
            ));
        }
        if !(60..=604_800).contains(&request.observation_seconds) {
            return Err(KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
                "observation_seconds must be between 60 and 604800".to_string(),
            ));
        }

        let checkpoint = json!({
            "schemaVersion": 1,
            "expectedSourceVersion": request.expected_source_version,
            "expectedTargetVersion": request.expected_target_version,
            "observationSeconds": request.observation_seconds,
        });
        let checkpoint_text = checkpoint.to_string();
        let mut tx = self.pool.begin().await.map_err(sql_error)?;

        if let Some(existing) =
            operation_by_idempotency_key(&mut tx, scope, request.idempotency_key.trim()).await?
        {
            let existing_checkpoint = operation_checkpoint(&mut tx, scope, existing.id).await?;
            if existing.space_id == space_id
                && existing.source_binding_id == request.source_binding_id
                && existing.target_binding_id == request.target_binding_id
                && existing_checkpoint == checkpoint
            {
                tx.commit().await.map_err(sql_error)?;
                return Ok(existing);
            }
            return Err(KnowledgeEngineProviderMigrationStoreError::Conflict(
                "idempotency key was already used for a different Provider migration".to_string(),
            ));
        }

        let source =
            binding_precondition(&mut tx, scope, request.source_binding_id, space_id).await?;
        let target =
            binding_precondition(&mut tx, scope, request.target_binding_id, space_id).await?;
        if source.state != KnowledgeEngineProviderBindingState::Active
            || source.version != request.expected_source_version
        {
            return Err(KnowledgeEngineProviderMigrationStoreError::Conflict(
                "source binding must be the expected active version".to_string(),
            ));
        }
        if target.state != KnowledgeEngineProviderBindingState::Testing
            || target.version != request.expected_target_version
            || !target.tested
        {
            return Err(KnowledgeEngineProviderMigrationStoreError::Conflict(
                "target binding must be the tested binding at the expected version".to_string(),
            ));
        }

        let id = next_i64_id(&self.id_generator).map_err(|error| {
            KnowledgeEngineProviderMigrationStoreError::Internal(error.to_string())
        })?;
        let operation_uuid = uuid();
        let now = now()?;
        let checkpoint_expr = self.dialect.sql_json_expr("$9");
        let created_at_expr = self.dialect.sql_timestamp_expr("$11");
        let updated_at_expr = self.dialect.sql_timestamp_expr("$12");
        let sql = format!(
            "INSERT INTO kb_provider_migration_operation (\
                id, uuid, tenant_id, organization_id, space_id, source_binding_id,\
                target_binding_id, operation_state, idempotency_key, checkpoint, requested_by,\
                created_at, updated_at, version, status\
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'dry_run', $8, {checkpoint_expr}, $10,\
                {created_at_expr}, {updated_at_expr}, 0, 1)"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .bind(&operation_uuid)
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("space_id", space_id)?)
            .bind(to_i64("source_binding_id", request.source_binding_id)?)
            .bind(to_i64("target_binding_id", request.target_binding_id)?)
            .bind(request.idempotency_key.trim())
            .bind(&checkpoint_text)
            .bind(actor_id.trim())
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(migration_sql_error)?;

        let operation = operation_by_id(&mut tx, scope, from_i64("id", id)?).await?;
        tx.commit().await.map_err(sql_error)?;
        Ok(operation)
    }

    async fn get_operation(
        &self,
        scope: KnowledgeEngineProviderScope,
        operation_id: u64,
    ) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError>
    {
        validate_scope(scope)?;
        self.fetch_operation(scope, operation_id).await?.ok_or(
            KnowledgeEngineProviderMigrationStoreError::NotFound(operation_id),
        )
    }

    async fn list_operations(
        &self,
        scope: KnowledgeEngineProviderScope,
        request: ListKnowledgeEngineProviderMigrationOperationsRequest,
    ) -> Result<
        KnowledgeEngineProviderMigrationOperationList,
        KnowledgeEngineProviderMigrationStoreError,
    > {
        validate_scope(scope)?;
        let page_size = request
            .page_size
            .unwrap_or(DEFAULT_LIST_PAGE_SIZE as u32)
            .clamp(1, MAX_LIST_PAGE_SIZE as u32);
        let cursor = parse_cursor(request.cursor.as_deref())?;
        let sql = format!(
            "SELECT {MIGRATION_COLUMNS} FROM kb_provider_migration_operation \
             WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND status = 1 \
               AND ($4 IS NULL OR operation_state = $4) AND ($5 IS NULL OR id < $5) \
             ORDER BY id DESC LIMIT $6"
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(to_i64("space_id", request.space_id)?)
            .bind(request.operation_state.map(|state| state.as_str()))
            .bind(cursor)
            .bind(i64::from(page_size) + 1)
            .fetch_all(&self.pool)
            .await
            .map_err(sql_error)?;
        let mut items = rows
            .into_iter()
            .map(map_operation)
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > page_size as usize;
        items.truncate(page_size as usize);
        let next_cursor = has_more
            .then(|| items.last().map(|item| item.id.to_string()))
            .flatten();
        Ok(KnowledgeEngineProviderMigrationOperationList { items, next_cursor })
    }

    async fn request_rollback(
        &self,
        scope: KnowledgeEngineProviderScope,
        operation_id: u64,
        actor_id: &str,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError>
    {
        validate_scope(scope)?;
        require_text("actor_id", actor_id, 128)?;
        let now = now()?;
        let updated_at = self.dialect.sql_timestamp_expr("$6");
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE kb_provider_migration_operation SET operation_state = 'rolling_back',\
             claim_owner = NULL, claim_token = NULL, lease_expires_at = NULL,\
             requested_by = $5, completed_at = NULL, updated_at = {updated_at}, version = version + 1 \
             WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND version = $4 \
               AND operation_state IN ('dry_run','preparing','validating','cutover','observing','failed')\
               AND status = 1"
        )))
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(to_i64("operation_id", operation_id)?)
        .bind(to_i64("expected_version", expected_version)?)
        .bind(actor_id.trim())
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(sql_error)?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeEngineProviderMigrationStoreError::Conflict(
                "rollback version or lifecycle precondition failed".to_string(),
            ));
        }
        self.get_operation(scope, operation_id).await
    }

    async fn claim_next(
        &self,
        scope: KnowledgeEngineProviderScope,
        worker_id: &str,
        lease_duration: Duration,
    ) -> Result<
        Option<ClaimedKnowledgeEngineProviderMigration>,
        KnowledgeEngineProviderMigrationStoreError,
    > {
        validate_scope(scope)?;
        require_text("worker_id", worker_id, 128)?;
        if lease_duration < Duration::from_secs(5) || lease_duration > Duration::from_secs(3_600) {
            return Err(KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
                "lease duration must be between 5 and 3600 seconds".to_string(),
            ));
        }
        for _ in 0..3 {
            let now = now()?;
            let lease_expires_at = format_instant(
                OffsetDateTime::now_utc()
                    + time::Duration::try_from(lease_duration).map_err(|error| {
                        KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
                            error.to_string(),
                        )
                    })?,
            )?;
            let now_expr = self.dialect.sql_timestamp_expr("$3");
            let candidate = sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT id, version FROM kb_provider_migration_operation \
                 WHERE tenant_id = $1 AND organization_id = $2 AND status = 1 \
                   AND operation_state NOT IN ('completed','rolled_back','failed') \
                   AND (operation_state <> 'observing' OR observation_until IS NULL OR observation_until <= {now_expr}) \
                   AND (claim_token IS NULL OR lease_expires_at IS NULL OR lease_expires_at <= {now_expr}) \
                 ORDER BY updated_at, id LIMIT 1"
            )))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(&now)
            .fetch_optional(&self.pool)
            .await
            .map_err(sql_error)?;
            let Some(candidate) = candidate else {
                return Ok(None);
            };
            let id: i64 = candidate.try_get("id").map_err(row_error)?;
            let version: i64 = candidate.try_get("version").map_err(row_error)?;
            let claim_token = uuid();
            let lease_expr = self.dialect.sql_timestamp_expr("$7");
            let updated_expr = self.dialect.sql_timestamp_expr("$8");
            let result = sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE kb_provider_migration_operation SET claim_owner = $5, claim_token = $6,\
                 lease_expires_at = {lease_expr}, attempt_count = attempt_count + 1,\
                 updated_at = {updated_expr}, version = version + 1 \
                 WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND version = $4 \
                   AND (claim_token IS NULL OR lease_expires_at IS NULL OR lease_expires_at <= {})",
                self.dialect.sql_timestamp_expr("$9")
            )))
            .bind(to_i64("tenant_id", scope.tenant_id)?)
            .bind(to_i64("organization_id", scope.organization_id)?)
            .bind(id)
            .bind(version)
            .bind(worker_id.trim())
            .bind(&claim_token)
            .bind(&lease_expires_at)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(sql_error)?;
            if result.rows_affected() == 0 {
                continue;
            }
            let operation_id = from_i64("id", id)?;
            let operation = self.get_operation(scope, operation_id).await?;
            let checkpoint = operation_checkpoint_pool(&self.pool, scope, operation_id).await?;
            return Ok(Some(ClaimedKnowledgeEngineProviderMigration {
                operation,
                claim_token,
                checkpoint,
            }));
        }
        Ok(None)
    }

    async fn advance_claimed(
        &self,
        scope: KnowledgeEngineProviderScope,
        operation_id: u64,
        claim_token: &str,
        expected_version: u64,
        transition: AdvanceClaimedKnowledgeEngineProviderMigration,
    ) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError>
    {
        if !transition_allowed(transition.expected_state, transition.next_state) {
            return Err(
                KnowledgeEngineProviderMigrationStoreError::InvalidLifecycle(format!(
                    "{} cannot transition to {}",
                    transition.expected_state.as_str(),
                    transition.next_state.as_str()
                )),
            );
        }
        let now = now()?;
        let checkpoint = transition.checkpoint.to_string();
        let checkpoint_expr = self.dialect.sql_json_expr("$8");
        let observation_expr = self.dialect.sql_timestamp_expr("$9");
        let completed_expr = self.dialect.sql_timestamp_expr("$10");
        let updated_expr = self.dialect.sql_timestamp_expr("$12");
        let lease_now_expr = self.dialect.sql_timestamp_expr("$13");
        let completed_at = matches!(
            transition.next_state,
            KnowledgeEngineProviderMigrationState::Completed
                | KnowledgeEngineProviderMigrationState::RolledBack
                | KnowledgeEngineProviderMigrationState::Failed
        )
        .then_some(now.as_str());
        let error = transition.error_category.map(error_category_str);
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE kb_provider_migration_operation SET operation_state = $7, checkpoint = {checkpoint_expr},\
             observation_until = {observation_expr}, completed_at = {completed_expr},\
             last_error_category = $11, claim_owner = NULL, claim_token = NULL, lease_expires_at = NULL,\
             updated_at = {updated_expr}, version = version + 1 \
             WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND claim_token = $4 \
               AND version = $5 AND operation_state = $6 AND lease_expires_at > {lease_now_expr}"
        )))
        .bind(to_i64("tenant_id", scope.tenant_id)?)
        .bind(to_i64("organization_id", scope.organization_id)?)
        .bind(to_i64("operation_id", operation_id)?)
        .bind(claim_token)
        .bind(to_i64("expected_version", expected_version)?)
        .bind(transition.expected_state.as_str())
        .bind(transition.next_state.as_str())
        .bind(&checkpoint)
        .bind(transition.observation_until.as_deref())
        .bind(completed_at)
        .bind(error)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(sql_error)?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeEngineProviderMigrationStoreError::ClaimLost(
                operation_id,
            ));
        }
        self.get_operation(scope, operation_id).await
    }

    async fn cutover_claimed(
        &self,
        scope: KnowledgeEngineProviderScope,
        command: CutoverClaimedKnowledgeEngineProviderMigration,
    ) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError>
    {
        let CutoverClaimedKnowledgeEngineProviderMigration {
            operation_id,
            claim_token,
            expected_version,
            actor_id,
            observation_until,
            mut checkpoint,
        } = command;
        let mut tx = self.pool.begin().await.map_err(sql_error)?;
        let operation = claimed_operation(
            &mut tx,
            scope,
            operation_id,
            &claim_token,
            expected_version,
            KnowledgeEngineProviderMigrationState::Cutover,
            self.dialect,
        )
        .await?;
        let source_version = checkpoint_u64(&checkpoint, "expectedSourceVersion")?;
        let target_version = checkpoint_u64(&checkpoint, "expectedTargetVersion")?;
        switch_bindings(
            &mut tx,
            scope,
            BindingSwitch {
                operation: &operation,
                actor: &actor_id,
                source_version,
                target_version,
                rollback: false,
            },
            self.dialect,
        )
        .await?;
        checkpoint["sourcePostCutoverVersion"] = json!(source_version + 1);
        checkpoint["targetPostCutoverVersion"] = json!(target_version + 1);
        update_claimed_operation(
            &mut tx,
            scope,
            ClaimedOperationUpdate {
                id: operation_id,
                token: &claim_token,
                version: expected_version,
                from: KnowledgeEngineProviderMigrationState::Cutover,
                to: KnowledgeEngineProviderMigrationState::Observing,
                checkpoint: &checkpoint,
                observation_until: Some(&observation_until),
                set_cutover: true,
                set_completed: false,
            },
            self.dialect,
        )
        .await?;
        let result = operation_by_id(&mut tx, scope, operation_id).await?;
        tx.commit().await.map_err(sql_error)?;
        Ok(result)
    }

    async fn rollback_claimed(
        &self,
        scope: KnowledgeEngineProviderScope,
        operation_id: u64,
        claim_token: &str,
        expected_version: u64,
        actor_id: &str,
        checkpoint: Value,
    ) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError>
    {
        let mut tx = self.pool.begin().await.map_err(sql_error)?;
        let operation = claimed_operation(
            &mut tx,
            scope,
            operation_id,
            claim_token,
            expected_version,
            KnowledgeEngineProviderMigrationState::RollingBack,
            self.dialect,
        )
        .await?;
        if operation.cutover_at.is_some() {
            switch_bindings(
                &mut tx,
                scope,
                BindingSwitch {
                    operation: &operation,
                    actor: actor_id,
                    source_version: checkpoint_u64(&checkpoint, "sourcePostCutoverVersion")?,
                    target_version: checkpoint_u64(&checkpoint, "targetPostCutoverVersion")?,
                    rollback: true,
                },
                self.dialect,
            )
            .await?;
        }
        update_claimed_operation(
            &mut tx,
            scope,
            ClaimedOperationUpdate {
                id: operation_id,
                token: claim_token,
                version: expected_version,
                from: KnowledgeEngineProviderMigrationState::RollingBack,
                to: KnowledgeEngineProviderMigrationState::RolledBack,
                checkpoint: &checkpoint,
                observation_until: operation.observation_until.as_deref(),
                set_cutover: false,
                set_completed: true,
            },
            self.dialect,
        )
        .await?;
        let result = operation_by_id(&mut tx, scope, operation_id).await?;
        tx.commit().await.map_err(sql_error)?;
        Ok(result)
    }
}

#[derive(Debug)]
struct BindingPrecondition {
    state: KnowledgeEngineProviderBindingState,
    version: u64,
    tested: bool,
}

async fn binding_precondition(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    binding_id: u64,
    space_id: u64,
) -> Result<BindingPrecondition, KnowledgeEngineProviderMigrationStoreError> {
    let row = sqlx::query("SELECT lifecycle_state, version, TO_CHAR(last_tested_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS last_tested_at FROM kb_provider_binding WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND id = $4 AND status = 1")
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("space_id", space_id)?).bind(to_i64("binding_id", binding_id)?)
        .fetch_optional(&mut **tx).await.map_err(sql_error)?.ok_or_else(|| KnowledgeEngineProviderMigrationStoreError::InvalidRequest(format!("binding_id={binding_id} is outside the migration space")))?;
    let state: String = row.try_get("lifecycle_state").map_err(row_error)?;
    Ok(BindingPrecondition {
        state: KnowledgeEngineProviderBindingState::from_str(&state).map_err(|_| {
            KnowledgeEngineProviderMigrationStoreError::Internal(format!(
                "unknown binding state={state}"
            ))
        })?,
        version: from_i64("version", row.try_get("version").map_err(row_error)?)?,
        tested: row
            .try_get::<Option<String>, _>("last_tested_at")
            .map_err(row_error)?
            .is_some(),
    })
}

async fn operation_by_id(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    id: u64,
) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {MIGRATION_COLUMNS} FROM kb_provider_migration_operation WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1")))
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("id", id)?).fetch_optional(&mut **tx).await.map_err(sql_error)?;
    row.map(map_operation)
        .transpose()?
        .ok_or(KnowledgeEngineProviderMigrationStoreError::NotFound(id))
}

async fn operation_by_idempotency_key(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    key: &str,
) -> Result<
    Option<KnowledgeEngineProviderMigrationOperation>,
    KnowledgeEngineProviderMigrationStoreError,
> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {MIGRATION_COLUMNS} FROM kb_provider_migration_operation WHERE tenant_id = $1 AND organization_id = $2 AND idempotency_key = $3 AND status = 1")))
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(key).fetch_optional(&mut **tx).await.map_err(sql_error)?;
    row.map(map_operation).transpose()
}

async fn operation_checkpoint(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    id: u64,
) -> Result<Value, KnowledgeEngineProviderMigrationStoreError> {
    let text: String = sqlx::query_scalar("SELECT CAST(checkpoint AS TEXT) FROM kb_provider_migration_operation WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1")
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("id", id)?).fetch_one(&mut **tx).await.map_err(sql_error)?;
    serde_json::from_str(&text)
        .map_err(|error| KnowledgeEngineProviderMigrationStoreError::Internal(error.to_string()))
}

async fn operation_checkpoint_pool(
    pool: &AnyPool,
    scope: KnowledgeEngineProviderScope,
    id: u64,
) -> Result<Value, KnowledgeEngineProviderMigrationStoreError> {
    let text: String = sqlx::query_scalar("SELECT CAST(checkpoint AS TEXT) FROM kb_provider_migration_operation WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = 1")
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("id", id)?).fetch_one(pool).await.map_err(sql_error)?;
    serde_json::from_str(&text)
        .map_err(|error| KnowledgeEngineProviderMigrationStoreError::Internal(error.to_string()))
}

async fn claimed_operation(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    id: u64,
    token: &str,
    version: u64,
    state: KnowledgeEngineProviderMigrationState,
    dialect: SqlTimestampDialect,
) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError> {
    let now = now()?;
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {MIGRATION_COLUMNS} FROM kb_provider_migration_operation WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND claim_token = $4 AND version = $5 AND operation_state = $6 AND lease_expires_at > {} AND status = 1", dialect.sql_timestamp_expr("$7"))))
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("id", id)?).bind(token).bind(to_i64("version", version)?).bind(state.as_str()).bind(now).fetch_optional(&mut **tx).await.map_err(sql_error)?;
    row.map(map_operation)
        .transpose()?
        .ok_or(KnowledgeEngineProviderMigrationStoreError::ClaimLost(id))
}

struct BindingSwitch<'a> {
    operation: &'a KnowledgeEngineProviderMigrationOperation,
    actor: &'a str,
    source_version: u64,
    target_version: u64,
    rollback: bool,
}

async fn switch_bindings(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    command: BindingSwitch<'_>,
    dialect: SqlTimestampDialect,
) -> Result<(), KnowledgeEngineProviderMigrationStoreError> {
    let BindingSwitch {
        operation,
        actor,
        source_version,
        target_version,
        rollback,
    } = command;
    let now = now()?;
    let updated = dialect.sql_timestamp_expr("$8");
    let (
        first_id,
        first_version,
        first_from,
        first_to,
        second_id,
        second_version,
        second_from,
        second_to,
    ) = if rollback {
        (
            operation.target_binding_id,
            target_version,
            "active",
            "disabled",
            operation.source_binding_id,
            source_version,
            "disabled",
            "active",
        )
    } else {
        (
            operation.source_binding_id,
            source_version,
            "active",
            "disabled",
            operation.target_binding_id,
            target_version,
            "testing",
            "active",
        )
    };
    for (id, version, from, to) in [
        (first_id, first_version, first_from, first_to),
        (second_id, second_version, second_from, second_to),
    ] {
        let result = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE kb_provider_binding SET lifecycle_state = $6, updated_by = $7, updated_at = {updated}, version = version + 1 WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND id = $4 AND version = $5 AND lifecycle_state = $9 AND status = 1")))
            .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("space_id", operation.space_id)?).bind(to_i64("binding_id", id)?).bind(to_i64("version", version)?).bind(to).bind(actor).bind(&now).bind(from).execute(&mut **tx).await.map_err(migration_sql_error)?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeEngineProviderMigrationStoreError::Conflict(
                format!("binding_id={id} cutover version or lifecycle precondition failed"),
            ));
        }
    }
    Ok(())
}

struct ClaimedOperationUpdate<'a> {
    id: u64,
    token: &'a str,
    version: u64,
    from: KnowledgeEngineProviderMigrationState,
    to: KnowledgeEngineProviderMigrationState,
    checkpoint: &'a Value,
    observation_until: Option<&'a str>,
    set_cutover: bool,
    set_completed: bool,
}

async fn update_claimed_operation(
    tx: &mut Transaction<'_, Any>,
    scope: KnowledgeEngineProviderScope,
    update: ClaimedOperationUpdate<'_>,
    dialect: SqlTimestampDialect,
) -> Result<(), KnowledgeEngineProviderMigrationStoreError> {
    let ClaimedOperationUpdate {
        id,
        token,
        version,
        from,
        to,
        checkpoint,
        observation_until,
        set_cutover,
        set_completed,
    } = update;
    let now = now()?;
    let checkpoint_expr = dialect.sql_json_expr("$8");
    let cutover_expr = dialect.sql_timestamp_expr("$9");
    let observation_expr = dialect.sql_timestamp_expr("$10");
    let completed_expr = dialect.sql_timestamp_expr("$11");
    let updated_expr = dialect.sql_timestamp_expr("$12");
    let result = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE kb_provider_migration_operation SET operation_state = $7, checkpoint = {checkpoint_expr}, cutover_at = COALESCE(cutover_at, {cutover_expr}), observation_until = {observation_expr}, completed_at = {completed_expr}, claim_owner = NULL, claim_token = NULL, lease_expires_at = NULL, updated_at = {updated_expr}, version = version + 1 WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND claim_token = $4 AND version = $5 AND operation_state = $6")))
        .bind(to_i64("tenant_id", scope.tenant_id)?).bind(to_i64("organization_id", scope.organization_id)?).bind(to_i64("id", id)?).bind(token).bind(to_i64("version", version)?).bind(from.as_str()).bind(to.as_str()).bind(checkpoint.to_string()).bind(set_cutover.then_some(now.as_str())).bind(observation_until).bind(set_completed.then_some(now.as_str())).bind(&now).execute(&mut **tx).await.map_err(sql_error)?;
    if result.rows_affected() != 1 {
        return Err(KnowledgeEngineProviderMigrationStoreError::ClaimLost(id));
    }
    Ok(())
}

fn map_operation(
    row: sqlx::any::AnyRow,
) -> Result<KnowledgeEngineProviderMigrationOperation, KnowledgeEngineProviderMigrationStoreError> {
    let state: String = row.try_get("operation_state").map_err(row_error)?;
    let error: Option<String> = row.try_get("last_error_category").map_err(row_error)?;
    Ok(KnowledgeEngineProviderMigrationOperation {
        id: from_i64("id", row.try_get("id").map_err(row_error)?)?,
        uuid: row.try_get("uuid").map_err(row_error)?,
        tenant_id: from_i64("tenant_id", row.try_get("tenant_id").map_err(row_error)?)?,
        organization_id: from_i64(
            "organization_id",
            row.try_get("organization_id").map_err(row_error)?,
        )?,
        space_id: from_i64("space_id", row.try_get("space_id").map_err(row_error)?)?,
        source_binding_id: from_i64(
            "source_binding_id",
            row.try_get("source_binding_id").map_err(row_error)?,
        )?,
        target_binding_id: from_i64(
            "target_binding_id",
            row.try_get("target_binding_id").map_err(row_error)?,
        )?,
        operation_state: KnowledgeEngineProviderMigrationState::from_str(&state).map_err(|_| {
            KnowledgeEngineProviderMigrationStoreError::Internal(format!(
                "unknown migration state={state}"
            ))
        })?,
        requested_by: row.try_get("requested_by").map_err(row_error)?,
        attempt_count: u32::try_from(row.try_get::<i64, _>("attempt_count").map_err(row_error)?)
            .map_err(|_| {
                KnowledgeEngineProviderMigrationStoreError::Internal(
                    "attempt_count is outside u32 range".to_string(),
                )
            })?,
        cutover_at: row.try_get("cutover_at").map_err(row_error)?,
        observation_until: row.try_get("observation_until").map_err(row_error)?,
        completed_at: row.try_get("completed_at").map_err(row_error)?,
        last_error_category: error.map(|v| parse_error_category(&v)).transpose()?,
        created_at: row.try_get("created_at").map_err(row_error)?,
        updated_at: row.try_get("updated_at").map_err(row_error)?,
        version: from_i64("version", row.try_get("version").map_err(row_error)?)?,
    })
}

fn transition_allowed(
    from: KnowledgeEngineProviderMigrationState,
    to: KnowledgeEngineProviderMigrationState,
) -> bool {
    matches!(
        (from, to),
        (
            KnowledgeEngineProviderMigrationState::DryRun,
            KnowledgeEngineProviderMigrationState::Preparing
        ) | (
            KnowledgeEngineProviderMigrationState::Preparing,
            KnowledgeEngineProviderMigrationState::Validating
        ) | (
            KnowledgeEngineProviderMigrationState::Validating,
            KnowledgeEngineProviderMigrationState::Cutover
        ) | (
            KnowledgeEngineProviderMigrationState::Observing,
            KnowledgeEngineProviderMigrationState::Completed
        ) | (
            KnowledgeEngineProviderMigrationState::DryRun
                | KnowledgeEngineProviderMigrationState::Preparing
                | KnowledgeEngineProviderMigrationState::Validating
                | KnowledgeEngineProviderMigrationState::Cutover
                | KnowledgeEngineProviderMigrationState::Observing
                | KnowledgeEngineProviderMigrationState::RollingBack,
            KnowledgeEngineProviderMigrationState::Failed
        )
    )
}
fn checkpoint_u64(
    value: &Value,
    key: &str,
) -> Result<u64, KnowledgeEngineProviderMigrationStoreError> {
    value.get(key).and_then(Value::as_u64).ok_or_else(|| {
        KnowledgeEngineProviderMigrationStoreError::Internal(format!(
            "migration checkpoint missing {key}"
        ))
    })
}
fn validate_scope(
    scope: KnowledgeEngineProviderScope,
) -> Result<(), KnowledgeEngineProviderMigrationStoreError> {
    if scope.tenant_id == 0
        || scope.tenant_id > i64::MAX as u64
        || scope.organization_id > i64::MAX as u64
    {
        return Err(KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
            "tenant/organization scope is outside signed 64-bit range".to_string(),
        ));
    }
    Ok(())
}
fn require_text(
    field: &str,
    value: &str,
    max: usize,
) -> Result<(), KnowledgeEngineProviderMigrationStoreError> {
    let value = value.trim();
    if is_blank(Some(value)) || value.chars().count() > max {
        return Err(KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
            format!("{field} must contain between 1 and {max} characters"),
        ));
    }
    Ok(())
}
fn parse_cursor(
    value: Option<&str>,
) -> Result<Option<i64>, KnowledgeEngineProviderMigrationStoreError> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.parse::<i64>().map_err(|_| {
                KnowledgeEngineProviderMigrationStoreError::InvalidRequest(
                    "cursor must be a valid migration operation id".to_string(),
                )
            })
        })
        .transpose()
}
fn now() -> Result<String, KnowledgeEngineProviderMigrationStoreError> {
    utc_sql_timestamp_text().map_err(KnowledgeEngineProviderMigrationStoreError::Internal)
}
fn format_instant(
    value: OffsetDateTime,
) -> Result<String, KnowledgeEngineProviderMigrationStoreError> {
    value
        .format(&Rfc3339)
        .map_err(|error| KnowledgeEngineProviderMigrationStoreError::Internal(error.to_string()))
}
fn to_i64(field: &str, value: u64) -> Result<i64, KnowledgeEngineProviderMigrationStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeEngineProviderMigrationStoreError::InvalidRequest(format!(
            "{field} exceeds signed 64-bit range"
        ))
    })
}
fn from_i64(field: &str, value: i64) -> Result<u64, KnowledgeEngineProviderMigrationStoreError> {
    u64::try_from(value).map_err(|_| {
        KnowledgeEngineProviderMigrationStoreError::Internal(format!("{field} is negative"))
    })
}
fn sql_error(error: sqlx::Error) -> KnowledgeEngineProviderMigrationStoreError {
    KnowledgeEngineProviderMigrationStoreError::Internal(error.to_string())
}
fn row_error(error: sqlx::Error) -> KnowledgeEngineProviderMigrationStoreError {
    KnowledgeEngineProviderMigrationStoreError::Internal(error.to_string())
}
fn migration_sql_error(error: sqlx::Error) -> KnowledgeEngineProviderMigrationStoreError {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("unique") {
        KnowledgeEngineProviderMigrationStoreError::Conflict(
            "an active Provider migration already exists for this space or idempotency key"
                .to_string(),
        )
    } else {
        KnowledgeEngineProviderMigrationStoreError::Internal(message)
    }
}
fn error_category_str(value: KnowledgeEngineProviderErrorCategory) -> &'static str {
    match value {
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
) -> Result<KnowledgeEngineProviderErrorCategory, KnowledgeEngineProviderMigrationStoreError> {
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
        _ => Err(KnowledgeEngineProviderMigrationStoreError::Internal(
            format!("unknown provider error category={value}"),
        )),
    }
}
