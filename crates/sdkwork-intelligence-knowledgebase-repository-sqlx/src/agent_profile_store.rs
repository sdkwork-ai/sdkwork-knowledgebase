use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_agent_profile_store::{
    KnowledgeAgentProfileStore, KnowledgeAgentProfileStoreError,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentBindingRequest, KnowledgeAgentKnowledgeMode,
    KnowledgeAgentProfile, KnowledgeAgentProfileRequest, KnowledgeAgentStatus, KnowledgeFilter,
};
use sdkwork_knowledgebase_contract::{
    default_agent_implementation_id, RIG_AGENT_IMPLEMENTATION_ID,
};
use sqlx::{any::AnyRow, AnyPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::sql_timestamp::{utc_sql_timestamp_text, SqlTimestampDialect};
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const ACTIVE_ROW_STATUS: i64 = 1;
const MAX_AGENT_BINDING_LIST_ROWS: i64 = 200;
const DELETED_ROW_STATUS: i64 = 0;
const INITIAL_VERSION: i64 = 0;

fn profile_json_returning_columns() -> &'static str {
    r#"CAST(model_parameters AS TEXT) AS model_parameters,
                retrieval_profile_id,
                CAST(citation_policy AS TEXT) AS citation_policy,
                memory_policy_ref,
                tool_policy_ref,
                CAST(answer_policy AS TEXT) AS answer_policy"#
}

fn binding_json_returning_columns() -> &'static str {
    r#"CAST(source_filter AS TEXT) AS source_filter,
                CAST(document_filter AS TEXT) AS document_filter"#
}

#[derive(Debug, Clone)]
pub struct PostgresKnowledgeAgentProfileStore {
    pool: AnyPool,
    tenant_id: u64,
    organization_id: u64,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

impl PostgresKnowledgeAgentProfileStore {
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
}

#[async_trait]
impl KnowledgeAgentProfileStore for PostgresKnowledgeAgentProfileStore {
    async fn create_profile(
        &self,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;

        let id = next_i64_id(&self.id_generator).map_err(agent_id_error)?;
        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let retrieval_profile_id = request
            .retrieval_profile_id
            .map(|value| to_i64("retrieval_profile_id", value))
            .transpose()?;
        let now = utc_sql_timestamp_text().map_err(agent_internal_error)?;

        let model_parameters_expr = self.timestamp_dialect.sql_json_expr("$10");
        let citation_policy_expr = self.timestamp_dialect.sql_json_expr("$12");
        let answer_policy_expr = self.timestamp_dialect.sql_json_expr("$15");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$19");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$20");
        let returning_columns = profile_json_returning_columns();
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO kb_agent_profile (
                id,
                uuid,
                tenant_id,
                organization_id,
                name,
                description,
                system_instruction,
                model_provider_id,
                model_id,
                model_parameters,
                retrieval_profile_id,
                citation_policy,
                memory_policy_ref,
                tool_policy_ref,
                answer_policy,
                knowledge_mode,
                agent_implementation_id,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, {model_parameters_expr}, $11, {citation_policy_expr}, $13, $14, {answer_policy_expr}, $16, $17, $18, {created_at_expr}, {updated_at_expr}, $21)
            RETURNING
                id,
                tenant_id,
                name,
                description,
                system_instruction,
                model_provider_id,
                model_id,
                {},
                knowledge_mode,
                agent_implementation_id,
                status
            "#,
            returning_columns,
        )))
        .bind(id)
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(organization_id)
        .bind(request.name)
        .bind(request.description)
        .bind(request.system_instruction)
        .bind(request.model_provider_id)
        .bind(request.model_id)
        .bind(request.model_parameters)
        .bind(retrieval_profile_id)
        .bind(request.citation_policy)
        .bind(request.memory_policy_ref)
        .bind(request.tool_policy_ref)
        .bind(request.answer_policy)
        .bind(knowledge_mode_code(request.knowledge_mode))
        .bind(agent_implementation_id_code(
            &request.agent_implementation_id,
        ))
        .bind(agent_status_code(request.status))
        .bind(now.clone())
        .bind(now)
        .bind(INITIAL_VERSION)
        .fetch_one(&self.pool)
        .await
        .map_err(agent_sqlx_error)?;

        let mut profile = profile_from_row(&row)?;
        profile.bindings = vec![];
        Ok(profile)
    }

    async fn retrieve_profile(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        let mut profile = self.select_profile(profile_id).await?;
        profile.bindings = self.list_bindings(profile_id).await?;
        Ok(profile)
    }

    async fn update_profile(
        &self,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;

        let profile_id_i64 = to_i64("profile_id", profile_id)?;
        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let retrieval_profile_id = request
            .retrieval_profile_id
            .map(|value| to_i64("retrieval_profile_id", value))
            .transpose()?;
        let now = utc_sql_timestamp_text().map_err(agent_internal_error)?;

        let model_parameters_expr = self.timestamp_dialect.sql_json_expr("$6");
        let citation_policy_expr = self.timestamp_dialect.sql_json_expr("$8");
        let answer_policy_expr = self.timestamp_dialect.sql_json_expr("$11");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$15");
        let returning_columns = profile_json_returning_columns();
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE kb_agent_profile
            SET name = $1,
                description = $2,
                system_instruction = $3,
                model_provider_id = $4,
                model_id = $5,
                model_parameters = {model_parameters_expr},
                retrieval_profile_id = $7,
                citation_policy = {citation_policy_expr},
                memory_policy_ref = $9,
                tool_policy_ref = $10,
                answer_policy = {answer_policy_expr},
                knowledge_mode = $12,
                agent_implementation_id = $13,
                status = $14,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $16 AND organization_id = $17 AND id = $18 AND status != $19
            RETURNING
                id,
                tenant_id,
                name,
                description,
                system_instruction,
                model_provider_id,
                model_id,
                {},
                knowledge_mode,
                agent_implementation_id,
                status
            "#,
            returning_columns,
        )))
        .bind(request.name)
        .bind(request.description)
        .bind(request.system_instruction)
        .bind(request.model_provider_id)
        .bind(request.model_id)
        .bind(request.model_parameters)
        .bind(retrieval_profile_id)
        .bind(request.citation_policy)
        .bind(request.memory_policy_ref)
        .bind(request.tool_policy_ref)
        .bind(request.answer_policy)
        .bind(knowledge_mode_code(request.knowledge_mode))
        .bind(agent_implementation_id_code(
            &request.agent_implementation_id,
        ))
        .bind(agent_status_code(request.status))
        .bind(now)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(profile_id_i64)
        .bind(agent_status_code(KnowledgeAgentStatus::Archived))
        .fetch_optional(&self.pool)
        .await
        .map_err(agent_sqlx_error)?
        .ok_or(KnowledgeAgentProfileStoreError::NotFound(profile_id))?;

        let mut profile = profile_from_row(&row)?;
        profile.bindings = self.list_bindings(profile_id).await?;
        Ok(profile)
    }

    async fn delete_profile(&self, profile_id: u64) -> Result<(), KnowledgeAgentProfileStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id_i64 = to_i64("profile_id", profile_id)?;
        let now = utc_sql_timestamp_text().map_err(agent_internal_error)?;

        // Archive the profile and its bindings in one transaction so a partial failure can
        // never leave an active binding for an archived profile.
        let mut transaction = self.pool.begin().await.map_err(agent_sqlx_error)?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_agent_profile
            SET status = $1,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND status != $6
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(agent_status_code(KnowledgeAgentStatus::Archived))
            .bind(now.clone())
            .bind(tenant_id)
            .bind(organization_id)
            .bind(profile_id_i64)
            .bind(agent_status_code(KnowledgeAgentStatus::Archived))
            .execute(&mut *transaction)
            .await
            .map_err(agent_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(KnowledgeAgentProfileStoreError::NotFound(profile_id));
        }

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_agent_knowledge_binding
            SET status = $1,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND profile_id = $5 AND status = $6
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(DELETED_ROW_STATUS)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(profile_id_i64)
            .bind(ACTIVE_ROW_STATUS)
            .execute(&mut *transaction)
            .await
            .map_err(agent_sqlx_error)?;

        transaction.commit().await.map_err(agent_sqlx_error)?;
        Ok(())
    }

    async fn list_bindings(
        &self,
        profile_id: u64,
    ) -> Result<Vec<KnowledgeAgentBinding>, KnowledgeAgentProfileStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id_i64 = to_i64("profile_id", profile_id)?;

        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT
                id,
                tenant_id,
                profile_id,
                space_id,
                collection_id,
                {},
                priority,
                top_k,
                min_score,
                enabled
            FROM kb_agent_knowledge_binding
            WHERE tenant_id = $1 AND organization_id = $2 AND profile_id = $3 AND status = $4
            ORDER BY priority DESC, id ASC
            LIMIT $5
            "#,
            binding_json_returning_columns(),
        )))
        .bind(tenant_id)
        .bind(organization_id)
        .bind(profile_id_i64)
        .bind(ACTIVE_ROW_STATUS)
        .bind(MAX_AGENT_BINDING_LIST_ROWS)
        .fetch_all(&self.pool)
        .await
        .map_err(agent_sqlx_error)?;

        rows.into_iter().map(binding_from_row).collect()
    }

    async fn create_binding(
        &self,
        request: KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;
        self.select_profile(request.profile_id).await?;

        let id = next_i64_id(&self.id_generator).map_err(agent_id_error)?;
        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id = to_i64("profile_id", request.profile_id)?;
        let space_id = to_i64("space_id", request.space_id)?;
        let collection_id = request
            .collection_id
            .map(|value| to_i64("collection_id", value))
            .transpose()?;
        let source_filter = option_json(&request.source_filter)?;
        let document_filter = option_json(&request.document_filter)?;
        let now = utc_sql_timestamp_text().map_err(agent_internal_error)?;

        let source_filter_expr = self.timestamp_dialect.sql_json_expr("$8");
        let document_filter_expr = self.timestamp_dialect.sql_json_expr("$9");
        let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$15");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$16");
        let returning_columns = binding_json_returning_columns();
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO kb_agent_knowledge_binding (
                id,
                uuid,
                tenant_id,
                organization_id,
                profile_id,
                space_id,
                collection_id,
                source_filter,
                document_filter,
                priority,
                top_k,
                min_score,
                enabled,
                status,
                created_at,
                updated_at,
                version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, {source_filter_expr}, {document_filter_expr}, $10, $11, $12, $13, $14, {created_at_expr}, {updated_at_expr}, $17)
            RETURNING
                id,
                tenant_id,
                profile_id,
                space_id,
                collection_id,
                {},
                priority,
                top_k,
                min_score,
                enabled
            "#,
            returning_columns,
        )))
        .bind(id)
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(organization_id)
        .bind(profile_id)
        .bind(space_id)
        .bind(collection_id)
        .bind(source_filter)
        .bind(document_filter)
        .bind(i64::from(request.priority))
        .bind(request.top_k.map(i64::from))
        .bind(request.min_score)
        .bind(enabled_code(request.enabled))
        .bind(ACTIVE_ROW_STATUS)
        .bind(now.clone())
        .bind(now)
        .bind(INITIAL_VERSION)
        .fetch_one(&self.pool)
        .await
        .map_err(agent_sqlx_error)?;

        binding_from_row(row)
    }

    async fn update_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, request.tenant_id)?;
        if request.profile_id != profile_id {
            return Err(KnowledgeAgentProfileStoreError::Internal(
                "binding request profile_id must match path profile_id".to_string(),
            ));
        }

        let tenant_id = to_i64("tenant_id", request.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id_i64 = to_i64("profile_id", profile_id)?;
        let binding_id_i64 = to_i64("binding_id", binding_id)?;
        let space_id = to_i64("space_id", request.space_id)?;
        let collection_id = request
            .collection_id
            .map(|value| to_i64("collection_id", value))
            .transpose()?;
        let source_filter = option_json(&request.source_filter)?;
        let document_filter = option_json(&request.document_filter)?;
        let now = utc_sql_timestamp_text().map_err(agent_internal_error)?;

        let source_filter_expr = self.timestamp_dialect.sql_json_expr("$3");
        let document_filter_expr = self.timestamp_dialect.sql_json_expr("$4");
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$9");
        let returning_columns = binding_json_returning_columns();
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE kb_agent_knowledge_binding
            SET space_id = $1,
                collection_id = $2,
                source_filter = {source_filter_expr},
                document_filter = {document_filter_expr},
                priority = $5,
                top_k = $6,
                min_score = $7,
                enabled = $8,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $10 AND organization_id = $11 AND profile_id = $12 AND id = $13 AND status = $14
            RETURNING
                id,
                tenant_id,
                profile_id,
                space_id,
                collection_id,
                {},
                priority,
                top_k,
                min_score,
                enabled
            "#,
            returning_columns,
        )))
        .bind(space_id)
        .bind(collection_id)
        .bind(source_filter)
        .bind(document_filter)
        .bind(i64::from(request.priority))
        .bind(request.top_k.map(i64::from))
        .bind(request.min_score)
        .bind(enabled_code(request.enabled))
        .bind(now)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(profile_id_i64)
        .bind(binding_id_i64)
        .bind(ACTIVE_ROW_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(agent_sqlx_error)?
        .ok_or(KnowledgeAgentProfileStoreError::NotFound(binding_id))?;

        binding_from_row(row)
    }

    async fn delete_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
    ) -> Result<(), KnowledgeAgentProfileStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id_i64 = to_i64("profile_id", profile_id)?;
        let binding_id_i64 = to_i64("binding_id", binding_id)?;
        let now = utc_sql_timestamp_text().map_err(agent_internal_error)?;

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_agent_knowledge_binding
            SET status = $1,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND profile_id = $5 AND id = $6 AND status = $7
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(DELETED_ROW_STATUS)
            .bind(now)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(profile_id_i64)
            .bind(binding_id_i64)
            .bind(ACTIVE_ROW_STATUS)
            .execute(&self.pool)
            .await
            .map_err(agent_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(KnowledgeAgentProfileStoreError::NotFound(binding_id));
        }
        Ok(())
    }

    async fn resolve_profile_id_for_space(
        &self,
        tenant_id: u64,
        space_id: u64,
    ) -> Result<Option<u64>, KnowledgeAgentProfileStoreError> {
        ensure_tenant_scope(self.tenant_id, tenant_id)?;
        let tenant_id_i64 = to_i64("tenant_id", tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let space_id_i64 = to_i64("space_id", space_id)?;

        let row = sqlx::query(
            r#"
            SELECT profile_id
            FROM kb_agent_knowledge_binding
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3 AND enabled = 1 AND status = $4
            ORDER BY priority DESC, id ASC
            LIMIT 1
            "#,
        )
        .bind(tenant_id_i64)
        .bind(organization_id)
        .bind(space_id_i64)
        .bind(ACTIVE_ROW_STATUS)
        .fetch_optional(&self.pool)
        .await
        .map_err(agent_sqlx_error)?;

        Ok(row.and_then(|row| {
            let profile_id = row.get::<i64, _>("profile_id") as u64;
            if profile_id == 0 {
                None
            } else {
                Some(profile_id)
            }
        }))
    }
}

impl PostgresKnowledgeAgentProfileStore {
    async fn select_profile(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        let tenant_id = to_i64("tenant_id", self.tenant_id)?;
        let organization_id = to_i64("organization_id", self.organization_id)?;
        let profile_id_i64 = to_i64("profile_id", profile_id)?;
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT
                id,
                tenant_id,
                name,
                description,
                system_instruction,
                model_provider_id,
                model_id,
                {},
                knowledge_mode,
                agent_implementation_id,
                status
            FROM kb_agent_profile
            WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status != $4
            "#,
            profile_json_returning_columns(),
        )))
        .bind(tenant_id)
        .bind(organization_id)
        .bind(profile_id_i64)
        .bind(agent_status_code(KnowledgeAgentStatus::Archived))
        .fetch_optional(&self.pool)
        .await
        .map_err(agent_sqlx_error)?
        .ok_or(KnowledgeAgentProfileStoreError::NotFound(profile_id))?;

        profile_from_row(&row)
    }
}

fn profile_from_row(
    row: &AnyRow,
) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
    Ok(KnowledgeAgentProfile {
        profile_id: u64_from_row(row, "id")?,
        tenant_id: u64_from_row(row, "tenant_id")?,
        name: row.try_get("name").map_err(agent_sqlx_error)?,
        description: row.try_get("description").map_err(agent_sqlx_error)?,
        system_instruction: row
            .try_get("system_instruction")
            .map_err(agent_sqlx_error)?,
        model_provider_id: row.try_get("model_provider_id").map_err(agent_sqlx_error)?,
        model_id: row.try_get("model_id").map_err(agent_sqlx_error)?,
        model_parameters: row.try_get("model_parameters").map_err(agent_sqlx_error)?,
        retrieval_profile_id: optional_u64_from_row(row, "retrieval_profile_id")?,
        citation_policy: row.try_get("citation_policy").map_err(agent_sqlx_error)?,
        memory_policy_ref: row.try_get("memory_policy_ref").map_err(agent_sqlx_error)?,
        tool_policy_ref: row.try_get("tool_policy_ref").map_err(agent_sqlx_error)?,
        answer_policy: row.try_get("answer_policy").map_err(agent_sqlx_error)?,
        knowledge_mode: knowledge_mode_from_code(
            row.try_get("knowledge_mode").map_err(agent_sqlx_error)?,
        )?,
        agent_implementation_id: agent_implementation_id_from_code(
            row.try_get("agent_implementation_id")
                .map_err(agent_sqlx_error)?,
        )?,
        status: agent_status_from_code(row.try_get("status").map_err(agent_sqlx_error)?)?,
        bindings: vec![],
    })
}

fn binding_from_row(row: AnyRow) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
    Ok(KnowledgeAgentBinding {
        binding_id: u64_from_row(&row, "id")?,
        profile_id: u64_from_row(&row, "profile_id")?,
        tenant_id: u64_from_row(&row, "tenant_id")?,
        space_id: u64_from_row(&row, "space_id")?,
        collection_id: optional_u64_from_row(&row, "collection_id")?,
        source_filter: parse_optional_filter(
            row.try_get("source_filter").map_err(agent_sqlx_error)?,
        )?,
        document_filter: parse_optional_filter(
            row.try_get("document_filter").map_err(agent_sqlx_error)?,
        )?,
        priority: i32_from_row(&row, "priority")?,
        top_k: optional_i64_from_row(&row, "top_k")?.map(|value| value as u32),
        min_score: row.try_get("min_score").map_err(agent_sqlx_error)?,
        enabled: enabled_from_code(row.try_get("enabled").map_err(agent_sqlx_error)?),
    })
}

fn option_json(
    value: &Option<Vec<KnowledgeFilter>>,
) -> Result<Option<String>, KnowledgeAgentProfileStoreError> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| KnowledgeAgentProfileStoreError::Internal(error.to_string()))
}

fn parse_optional_filter(
    value: Option<String>,
) -> Result<Option<Vec<KnowledgeFilter>>, KnowledgeAgentProfileStoreError> {
    value
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|error| KnowledgeAgentProfileStoreError::Internal(error.to_string()))
}

fn ensure_tenant_scope(
    store_tenant_id: u64,
    request_tenant_id: u64,
) -> Result<(), KnowledgeAgentProfileStoreError> {
    if store_tenant_id != request_tenant_id {
        return Err(KnowledgeAgentProfileStoreError::Internal(
            "request tenant_id must match store tenant scope".to_string(),
        ));
    }
    Ok(())
}

fn agent_status_code(value: KnowledgeAgentStatus) -> i64 {
    match value {
        KnowledgeAgentStatus::Draft => 0,
        KnowledgeAgentStatus::Active => 1,
        KnowledgeAgentStatus::Disabled => 2,
        KnowledgeAgentStatus::Archived => 3,
    }
}

fn agent_status_from_code(
    code: i64,
) -> Result<KnowledgeAgentStatus, KnowledgeAgentProfileStoreError> {
    match code {
        0 => Ok(KnowledgeAgentStatus::Draft),
        1 => Ok(KnowledgeAgentStatus::Active),
        2 => Ok(KnowledgeAgentStatus::Disabled),
        3 => Ok(KnowledgeAgentStatus::Archived),
        _ => Err(KnowledgeAgentProfileStoreError::Internal(format!(
            "unknown knowledge agent status code: {code}"
        ))),
    }
}

fn enabled_code(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn enabled_from_code(value: i64) -> bool {
    value != 0
}

fn u64_from_row(row: &AnyRow, column: &str) -> Result<u64, KnowledgeAgentProfileStoreError> {
    let value: i64 = row.try_get(column).map_err(agent_sqlx_error)?;
    u64::try_from(value).map_err(|_| {
        KnowledgeAgentProfileStoreError::Internal(format!("{column} must not be negative"))
    })
}

fn optional_u64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<Option<u64>, KnowledgeAgentProfileStoreError> {
    optional_i64_from_row(row, column)?
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                KnowledgeAgentProfileStoreError::Internal(format!("{column} must not be negative"))
            })
        })
        .transpose()
}

fn i32_from_row(row: &AnyRow, column: &str) -> Result<i32, KnowledgeAgentProfileStoreError> {
    let value: i64 = row.try_get(column).map_err(agent_sqlx_error)?;
    i32::try_from(value).map_err(|_| {
        KnowledgeAgentProfileStoreError::Internal(format!("{column} exceeds int32 range"))
    })
}

fn optional_i64_from_row(
    row: &AnyRow,
    column: &str,
) -> Result<Option<i64>, KnowledgeAgentProfileStoreError> {
    row.try_get(column).map_err(agent_sqlx_error)
}

fn to_i64(field_name: &str, value: u64) -> Result<i64, KnowledgeAgentProfileStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeAgentProfileStoreError::Internal(format!(
            "{field_name} exceeds signed int64 range"
        ))
    })
}

fn agent_internal_error(message: String) -> KnowledgeAgentProfileStoreError {
    KnowledgeAgentProfileStoreError::Internal(message)
}

fn agent_sqlx_error(error: sqlx::Error) -> KnowledgeAgentProfileStoreError {
    KnowledgeAgentProfileStoreError::Internal(error.to_string())
}

fn agent_id_error(error: crate::id::KnowledgeIdGeneratorError) -> KnowledgeAgentProfileStoreError {
    KnowledgeAgentProfileStoreError::Internal(error.to_string())
}

fn knowledge_mode_code(mode: KnowledgeAgentKnowledgeMode) -> &'static str {
    mode.as_str()
}

fn knowledge_mode_from_code(
    value: String,
) -> Result<KnowledgeAgentKnowledgeMode, KnowledgeAgentProfileStoreError> {
    match value.as_str() {
        "okf_bundle" => Ok(KnowledgeAgentKnowledgeMode::OkfBundle),
        "rag" => Ok(KnowledgeAgentKnowledgeMode::Rag),
        "external" => Ok(KnowledgeAgentKnowledgeMode::External),
        other => Err(KnowledgeAgentProfileStoreError::Internal(format!(
            "unsupported knowledge_mode value: {other}"
        ))),
    }
}

fn agent_implementation_id_code(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        RIG_AGENT_IMPLEMENTATION_ID
    } else {
        trimmed
    }
}

fn agent_implementation_id_from_code(
    value: String,
) -> Result<String, KnowledgeAgentProfileStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(default_agent_implementation_id())
    } else {
        Ok(trimmed.to_string())
    }
}
