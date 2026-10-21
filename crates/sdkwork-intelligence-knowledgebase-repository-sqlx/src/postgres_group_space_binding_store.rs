use async_trait::async_trait;
use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_group_space_binding_store::{
    ArchiveGroupKnowledgeSpaceCommand, GroupKnowledgeSpaceArchiveReservation,
    GroupKnowledgeSpaceMembershipChange, GroupKnowledgeSpaceMembershipSyncReservation,
    GroupKnowledgeSpaceReservation, GroupKnowledgeSpaceScope, GroupKnowledgeSpaceTarget,
    KnowledgeGroupSpaceBindingStore, KnowledgeGroupSpaceBindingStoreError,
    ReserveGroupKnowledgeSpaceRequest, SynchronizeGroupKnowledgeSpaceMembersCommand,
};
use sdkwork_knowledgebase_contract::group_space::{
    GroupKnowledgeSpaceAclProjectionState, GroupKnowledgeSpaceBinding,
    GroupKnowledgeSpaceLifecycleState, GroupKnowledgeSpaceMember, GroupKnowledgeSpaceMemberRole,
    GroupKnowledgeSpacePrincipalKind, GROUP_KNOWLEDGE_SPACE_ACTOR_ID_MAX_LENGTH,
    GROUP_KNOWLEDGE_SPACE_BINDING_UUID_MAX_LENGTH,
    GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH, GROUP_KNOWLEDGE_SPACE_GROUP_NAME_MAX_LENGTH,
    GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS,
    GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH, GROUP_KNOWLEDGE_SPACE_SPACE_UUID_MAX_LENGTH,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_contract::space::{KnowledgeSpace, KnowledgeSpaceStatus};
use sdkwork_utils_rust::{is_blank, sha256_hash};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use uuid::Uuid;

use crate::db::sql_timestamp::{utc_sql_timestamp_text, SqlTimestampDialect};
use crate::id::{default_knowledge_id_generator, next_i64_id, KnowledgeIdGenerator};

const ACTIVE_STATUS: i64 = 1;
const INACTIVE_STATUS: i64 = 0;
const SPACE_PROVISIONING_STATUS: i64 = 0;
const SPACE_ACTIVE_STATUS: i64 = 1;
const SPACE_ARCHIVED_STATUS: i64 = 2;
const SPACE_DELETED_STATUS: i64 = 3;
const INITIAL_VERSION: i64 = 0;
const PROVISIONING_LEASE_SECONDS: i64 = 300;
const MEMBERSHIP_PROJECTION_PENDING: &str = "pending";
const MEMBERSHIP_PROJECTION_FAILED: &str = "failed";
const MEMBERSHIP_PROJECTION_COMPLETED: &str = "completed";
const MAX_GROUP_SCOPE_ID: u64 = i64::MAX as u64;

#[derive(Debug, Clone)]
pub struct PostgresGroupKnowledgeSpaceBindingStore {
    pool: AnyPool,
    id_generator: Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: SqlTimestampDialect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupMembershipProjection {
    id: u64,
    binding_id: u64,
    payload_sha256_hex: String,
    target_membership_epoch: u64,
    projection_state: String,
    projection_lease_token: Option<String>,
}

struct GroupKnowledgeSpaceStoreOperationContext<'a> {
    id_generator: &'a Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: &'a SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    now: &'a str,
}

impl PostgresGroupKnowledgeSpaceBindingStore {
    pub fn new(pool: AnyPool) -> Self {
        Self::with_id_generator(pool, default_knowledge_id_generator())
    }

    pub fn with_id_generator(pool: AnyPool, id_generator: Arc<dyn KnowledgeIdGenerator>) -> Self {
        Self {
            pool,
            id_generator,
            timestamp_dialect: SqlTimestampDialect::default(),
        }
    }

    pub fn with_database_engine(mut self, database_engine: DatabaseEngine) -> Self {
        self.timestamp_dialect = SqlTimestampDialect::from_database_engine(database_engine);
        self
    }

    async fn get_bound_group_space(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding: &GroupKnowledgeSpaceBinding,
    ) -> Result<Option<KnowledgeSpace>, KnowledgeGroupSpaceBindingStoreError> {
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let space = fetch_binding_space(&mut transaction, scope, binding).await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(space)
    }

    async fn list_resumable_group_space_archives_for_scope(
        &self,
        tenant_id: u64,
        organization_id: u64,
        limit: u32,
    ) -> Result<Vec<ArchiveGroupKnowledgeSpaceCommand>, KnowledgeGroupSpaceBindingStoreError> {
        const MAX_ARCHIVE_WORK_BATCH: u32 = 200;
        validate_tenant_id(tenant_id)?;
        validate_scope(GroupKnowledgeSpaceScope {
            tenant_id,
            organization_id,
        })?;
        if limit == 0 || limit > MAX_ARCHIVE_WORK_BATCH {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                format!("archive work limit must be between 1 and {MAX_ARCHIVE_WORK_BATCH}"),
            ));
        }

        let now = group_now()?;
        let rows = {
            let lease_expiry_expr = self.timestamp_dialect.sql_timestamp_expr("$3");
            let query = format!(
                r#"
                SELECT id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
                       group_name, lifecycle_state, acl_projection_state,
                       provisioning_idempotency_key_sha256_hex, membership_epoch,
                       upstream_link_generation, version, archive_source_event_id,
                       archive_payload_sha256_hex, archive_lease_token, archive_lease_until,
                       archive_acl_cursor, archive_acl_pages_processed,
                       archive_acl_cleanup_completed_at,
                       last_source_event_id, last_error_code, created_by, updated_by,
                       created_at, updated_at, archived_at, archived_by, deleted_at
                FROM kb_group_knowledge_space_binding
                WHERE tenant_id = $1 AND organization_id = $2
                  AND lifecycle_state = 'archiving'
                  AND archive_source_event_id IS NOT NULL
                  AND archive_payload_sha256_hex IS NOT NULL
                  AND archived_by IS NOT NULL
                  AND (archive_lease_token IS NULL OR archive_lease_until < {lease_expiry_expr})
                ORDER BY updated_at ASC, id ASC
                LIMIT $4
                "#,
            );
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(group_to_i64("tenant_id", tenant_id)?)
                .bind(group_to_i64("organization_id", organization_id)?)
                .bind(&now)
                .bind(i64::from(limit))
                .fetch_all(&self.pool)
                .await
                .map_err(group_sqlx_error)?
        };

        rows.into_iter()
            .map(|row| archive_command_from_binding(group_binding_from_row(&row)?))
            .collect()
    }

    async fn reserve_inner(
        &self,
        request: &ReserveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceReservation, KnowledgeGroupSpaceBindingStoreError> {
        let scope = request.scope;
        let tenant_id = group_to_i64("tenant_id", scope.tenant_id)?;
        let organization_id = group_to_i64("organization_id", scope.organization_id)?;
        let fingerprint = reservation_fingerprint(request);
        let now = group_now()?;
        let lease_until = group_lease_until()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;

        if let Some((binding_id, existing_fingerprint)) = sqlx::query(
            r#"
            SELECT binding_id, payload_sha256_hex
            FROM kb_group_knowledge_space_event_inbox
            WHERE tenant_id = $1 AND organization_id = $2 AND source_event_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(&request.source_event_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(group_sqlx_error)?
        .map(|row| {
            Ok::<_, KnowledgeGroupSpaceBindingStoreError>((
                row.try_get::<Option<i64>, _>("binding_id")
                    .map_err(group_sqlx_error)?,
                row.try_get::<String, _>("payload_sha256_hex")
                    .map_err(group_sqlx_error)?,
            ))
        })
        .transpose()?
        {
            if existing_fingerprint != fingerprint {
                return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "source_event_id was already applied with a different payload".to_string(),
                ));
            }
            let binding_id = binding_id.ok_or_else(|| {
                KnowledgeGroupSpaceBindingStoreError::Internal(
                    "group knowledge space inbox event has no binding id".to_string(),
                )
            })?;
            let mut binding = fetch_binding_by_id(
                &mut transaction,
                scope,
                group_from_i64("binding_id", binding_id)?,
            )
            .await?;
            let mut requires_provisioning = false;
            let mut lease_token = None;
            if binding.lifecycle_state == GroupKnowledgeSpaceLifecycleState::Provisioning {
                lease_token = claim_provisioning_lease(
                    &mut transaction,
                    &self.timestamp_dialect,
                    scope,
                    binding.id,
                    &request.created_by,
                    &now,
                    &lease_until,
                )
                .await?;
                requires_provisioning = lease_token.is_some();
                if requires_provisioning {
                    binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
                }
            }
            let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceReservation {
                binding,
                space,
                requires_provisioning,
                provisioning_lease_token: lease_token,
            });
        }

        // IM persists one stable creation idempotency key before it calls Knowledgebase. The
        // caller can crash after a successful ensure response but before storing the returned
        // binding id, then retry under a fresh source event. Resolve that durable identity before
        // lifecycle branching so an archived terminal binding is returned rather than recreated.
        let provisioning_idempotency_key_sha256_hex =
            sha256_hash(request.provisioning_idempotency_key.as_bytes());
        if let Some(binding) = fetch_binding_by_provisioning_idempotency_key(
            &mut transaction,
            scope,
            &provisioning_idempotency_key_sha256_hex,
        )
        .await?
        {
            if binding.conversation_id != request.conversation_id {
                return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "provisioning idempotency key belongs to another group conversation"
                        .to_string(),
                ));
            }
            append_inbox_event(
                &mut transaction,
                &operation_context,
                binding.id,
                &request.source_event_id,
                "group_space_ensure_idempotent_replay",
                &fingerprint,
            )
            .await?;
            let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceReservation {
                binding,
                space,
                requires_provisioning: false,
                provisioning_lease_token: None,
            });
        }

        let existing =
            fetch_binding_by_conversation(&mut transaction, scope, &request.conversation_id)
                .await?;

        let (binding, space, requires_provisioning, provisioning_lease_token) = match existing {
            Some(mut binding) => match binding.lifecycle_state {
                GroupKnowledgeSpaceLifecycleState::Active => {
                    append_inbox_event(
                        &mut transaction,
                        &operation_context,
                        binding.id,
                        &request.source_event_id,
                        "group_space_ensure",
                        &fingerprint,
                    )
                    .await?;
                    let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
                    (binding, space, false, None)
                }
                GroupKnowledgeSpaceLifecycleState::Provisioning => {
                    let token = claim_provisioning_lease(
                        &mut transaction,
                        &self.timestamp_dialect,
                        scope,
                        binding.id,
                        &request.created_by,
                        &now,
                        &lease_until,
                    )
                    .await?;
                    append_inbox_event(
                        &mut transaction,
                        &operation_context,
                        binding.id,
                        &request.source_event_id,
                        "group_space_ensure",
                        &fingerprint,
                    )
                    .await?;
                    if token.is_some() {
                        binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
                    }
                    let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
                    (binding, space, token.is_some(), token)
                }
                GroupKnowledgeSpaceLifecycleState::Failed => {
                    let (binding, space, token) = reset_binding_for_provisioning(
                        &mut transaction,
                        &operation_context,
                        binding,
                        request,
                        &lease_until,
                    )
                    .await?;
                    append_inbox_event(
                        &mut transaction,
                        &operation_context,
                        binding.id,
                        &request.source_event_id,
                        "group_space_ensure",
                        &fingerprint,
                    )
                    .await?;
                    append_outbox_event(
                        &mut transaction,
                        &operation_context,
                        binding.id,
                        "knowledge.group_space.provisioning_started",
                        &group_event_payload(&binding),
                    )
                    .await?;
                    (binding, Some(space), true, Some(token))
                }
                GroupKnowledgeSpaceLifecycleState::Archiving
                | GroupKnowledgeSpaceLifecycleState::Archived
                | GroupKnowledgeSpaceLifecycleState::Deleted => {
                    return Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                        "a terminal group knowledge space cannot be provisioned again".to_string(),
                    ));
                }
            },
            None => {
                let binding_id = next_group_id(&self.id_generator)?;
                let space = insert_group_space(
                    &mut transaction,
                    &self.id_generator,
                    &self.timestamp_dialect,
                    scope,
                    &request.group_name,
                    &now,
                )
                .await?;
                let token = Uuid::new_v4().to_string();
                let idempotency_key_hash =
                    sha256_hash(request.provisioning_idempotency_key.as_bytes());
                let created_at_expr = self.timestamp_dialect.sql_timestamp_expr("$18");
                let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$19");
                let lease_until_expr = self.timestamp_dialect.sql_timestamp_expr("$13");
                let query = format!(
                    r#"
                    INSERT INTO kb_group_knowledge_space_binding (
                        id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
                        group_name, lifecycle_state, acl_projection_state,
                        provisioning_idempotency_key_sha256_hex, provisioning_lease_token,
                        provisioning_lease_until, membership_epoch, last_source_event_id,
                        created_by, updated_by, created_at, updated_at, version
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, {lease_until_expr}, $14, $15, $16, $17, {created_at_expr}, {updated_at_expr}, $20)
                    "#,
                );
                sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                    .bind(group_to_i64("binding_id", binding_id)?)
                    .bind(Uuid::new_v4().to_string())
                    .bind(tenant_id)
                    .bind(organization_id)
                    .bind(&request.conversation_id)
                    .bind(group_to_i64("space_id", space.id)?)
                    .bind(&space.uuid)
                    .bind(&request.group_name)
                    .bind(GroupKnowledgeSpaceLifecycleState::Provisioning.as_str())
                    .bind(GroupKnowledgeSpaceAclProjectionState::Pending.as_str())
                    .bind(idempotency_key_hash)
                    .bind(&token)
                    .bind(&lease_until)
                    .bind(group_to_i64("membership_epoch", request.membership_epoch)?)
                    .bind(&request.source_event_id)
                    .bind(&request.created_by)
                    .bind(&request.created_by)
                    .bind(&now)
                    .bind(&now)
                    .bind(INITIAL_VERSION)
                    .execute(&mut *transaction)
                    .await
                    .map_err(group_sqlx_error)?;
                replace_active_members(
                    &mut transaction,
                    &operation_context,
                    binding_id,
                    request.membership_epoch,
                    &request.members,
                )
                .await?;
                let binding = fetch_binding_by_id(&mut transaction, scope, binding_id).await?;
                append_inbox_event(
                    &mut transaction,
                    &operation_context,
                    binding.id,
                    &request.source_event_id,
                    "group_space_ensure",
                    &fingerprint,
                )
                .await?;
                append_outbox_event(
                    &mut transaction,
                    &operation_context,
                    binding.id,
                    "knowledge.group_space.provisioning_started",
                    &group_event_payload(&binding),
                )
                .await?;
                (binding, Some(space), true, Some(token))
            }
        };

        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(GroupKnowledgeSpaceReservation {
            binding,
            space,
            requires_provisioning,
            provisioning_lease_token,
        })
    }
}

#[async_trait]
impl KnowledgeGroupSpaceBindingStore for PostgresGroupKnowledgeSpaceBindingStore {
    async fn reserve_group_space(
        &self,
        request: ReserveGroupKnowledgeSpaceRequest,
    ) -> Result<GroupKnowledgeSpaceReservation, KnowledgeGroupSpaceBindingStoreError> {
        validate_reservation_request(&request)?;
        match self.reserve_inner(&request).await {
            Err(error @ KnowledgeGroupSpaceBindingStoreError::Conflict(_)) => {
                // A competing transaction may have committed the unique conversation binding. The
                // durable unique index is the concurrency authority; read back the winner.
                let binding = self
                    .get_group_space(request.scope, &request.conversation_id)
                    .await?;
                if matches!(
                    binding.lifecycle_state,
                    GroupKnowledgeSpaceLifecycleState::Active
                        | GroupKnowledgeSpaceLifecycleState::Provisioning
                ) {
                    let space = self.get_bound_group_space(request.scope, &binding).await?;
                    Ok(GroupKnowledgeSpaceReservation {
                        binding,
                        space,
                        requires_provisioning: false,
                        provisioning_lease_token: None,
                    })
                } else {
                    Err(error)
                }
            }
            result => result,
        }
    }

    async fn get_group_space(
        &self,
        scope: GroupKnowledgeSpaceScope,
        conversation_id: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(scope)?;
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding = fetch_binding_by_conversation(&mut transaction, scope, conversation_id)
            .await?
            .ok_or_else(|| {
                KnowledgeGroupSpaceBindingStoreError::NotFound(conversation_id.to_string())
            })?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn find_group_space_for_space_in_tenant(
        &self,
        tenant_id: u64,
        organization_id: u64,
        space_id: u64,
    ) -> Result<Option<GroupKnowledgeSpaceBinding>, KnowledgeGroupSpaceBindingStoreError> {
        if tenant_id == 0 {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "tenant_id is required".to_string(),
            ));
        }
        let tenant_id = group_to_i64("tenant_id", tenant_id)?;
        let organization_id = group_to_i64("organization_id", organization_id)?;
        let space_id = group_to_i64("space_id", space_id)?;
        let row = sqlx::query(
            r#"
            SELECT id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
                   group_name, lifecycle_state, acl_projection_state,
                   provisioning_idempotency_key_sha256_hex, membership_epoch,
                   upstream_link_generation, version, archive_source_event_id,
                   archive_payload_sha256_hex, archive_lease_token, archive_lease_until,
                   archive_acl_cursor, archive_acl_pages_processed,
                   archive_acl_cleanup_completed_at,
                   last_source_event_id, last_error_code, created_by, updated_by,
                   created_at, updated_at, archived_at, archived_by, deleted_at
            FROM kb_group_knowledge_space_binding
            WHERE tenant_id = $1 AND organization_id = $2 AND space_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(space_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(group_sqlx_error)?;
        row.map(|row| group_binding_from_row(&row)).transpose()
    }

    async fn list_active_group_members(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<Vec<GroupKnowledgeSpaceMember>, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(scope)?;
        list_active_members(&self.pool, scope, binding_id).await
    }

    async fn has_unsettled_group_membership_projection(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<bool, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(scope)?;
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) AS projection_count
            FROM kb_group_knowledge_space_membership_projection
            WHERE tenant_id = $1 AND organization_id = $2 AND binding_id = $3
              AND projection_state IN ($4, $5)
            "#,
        )
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(MEMBERSHIP_PROJECTION_FAILED)
        .fetch_one(&self.pool)
        .await
        .map_err(group_sqlx_error)?;
        let projection_count: i64 = row.try_get("projection_count").map_err(group_sqlx_error)?;
        Ok(projection_count > 0)
    }

    async fn is_group_membership_projection_lease_current(
        &self,
        command: &SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
    ) -> Result<bool, KnowledgeGroupSpaceBindingStoreError> {
        validate_membership_command(command)?;
        validate_group_text(
            "synchronization_lease_token",
            synchronization_lease_token,
            64,
        )?;
        let binding = self
            .get_group_space(command.scope, &command.conversation_id)
            .await?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Active
            || binding.acl_projection_state != GroupKnowledgeSpaceAclProjectionState::Active
        {
            return Ok(false);
        }
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) AS projection_count
            FROM kb_group_knowledge_space_membership_projection
            WHERE tenant_id = $1 AND organization_id = $2 AND binding_id = $3
              AND source_event_id = $4 AND payload_sha256_hex = $5
              AND target_membership_epoch = $6 AND projection_state = $7
              AND projection_lease_token = $8
            "#,
        )
        .bind(group_to_i64("tenant_id", command.scope.tenant_id)?)
        .bind(group_to_i64(
            "organization_id",
            command.scope.organization_id,
        )?)
        .bind(group_to_i64("binding_id", binding.id)?)
        .bind(&command.source_event_id)
        .bind(membership_command_fingerprint(command))
        .bind(group_to_i64("membership_epoch", command.membership_epoch)?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(synchronization_lease_token)
        .fetch_one(&self.pool)
        .await
        .map_err(group_sqlx_error)?;
        let projection_count: i64 = row.try_get("projection_count").map_err(group_sqlx_error)?;
        Ok(projection_count == 1)
    }

    async fn has_active_group_membership_projection_lease(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<bool, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(scope)?;
        let now = group_now()?;
        let expiry_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let query = format!(
            r#"
            SELECT COUNT(*) AS projection_count
            FROM kb_group_knowledge_space_membership_projection
            WHERE tenant_id = $1 AND organization_id = $2 AND binding_id = $3
              AND projection_state = $5
              AND projection_lease_token IS NOT NULL
              AND projection_lease_until IS NOT NULL
              AND projection_lease_until >= {expiry_expr}
            "#,
        );
        let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding_id)?)
            .bind(&now)
            .bind(MEMBERSHIP_PROJECTION_PENDING)
            .fetch_one(&self.pool)
            .await
            .map_err(group_sqlx_error)?;
        let projection_count: i64 = row.try_get("projection_count").map_err(group_sqlx_error)?;
        Ok(projection_count > 0)
    }

    async fn mark_group_space_active(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
        provisioning_lease_token: &str,
        updated_by: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(scope)?;
        if is_blank(Some(provisioning_lease_token)) || is_blank(Some(updated_by)) {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "provisioning lease token and updated_by are required".to_string(),
            ));
        }
        let now = group_now()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let query = format!(
            r#"
            UPDATE kb_group_knowledge_space_binding
            SET lifecycle_state = $2,
                acl_projection_state = $3,
                provisioning_lease_token = NULL,
                provisioning_lease_until = NULL,
                last_error_code = NULL,
                last_error_at = NULL,
                updated_by = $4,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $5 AND organization_id = $6 AND id = $7
              AND lifecycle_state = $8 AND provisioning_lease_token = $9
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(&now)
            .bind(GroupKnowledgeSpaceLifecycleState::Active.as_str())
            .bind(GroupKnowledgeSpaceAclProjectionState::Active.as_str())
            .bind(updated_by)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding_id)?)
            .bind(GroupKnowledgeSpaceLifecycleState::Provisioning.as_str())
            .bind(provisioning_lease_token)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "provisioning lease is no longer current".to_string(),
            ));
        }
        let binding = fetch_binding_by_id(&mut transaction, scope, binding_id).await?;
        append_outbox_event(
            &mut transaction,
            &operation_context,
            binding_id,
            "knowledge.group_space.provisioned",
            &group_event_payload(&binding),
        )
        .await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn mark_group_space_failed(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
        provisioning_lease_token: &str,
        error_code: &str,
        updated_by: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(scope)?;
        if is_blank(Some(provisioning_lease_token))
            || is_blank(Some(error_code))
            || is_blank(Some(updated_by))
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "provisioning lease token, error code, and updated_by are required".to_string(),
            ));
        }
        let now = group_now()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding = fetch_binding_by_id(&mut transaction, scope, binding_id).await?;
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Provisioning {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                "only a provisioning group knowledge space can fail provisioning".to_string(),
            ));
        }
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let error_at_expr = self.timestamp_dialect.sql_timestamp_expr("$1");
        let query = format!(
            r#"
            UPDATE kb_group_knowledge_space_binding
            SET lifecycle_state = $2,
                acl_projection_state = $3,
                last_error_code = $4,
                last_error_at = {error_at_expr},
                space_id = NULL,
                space_uuid = NULL,
                provisioning_lease_token = NULL,
                provisioning_lease_until = NULL,
                updated_by = $5,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $6 AND organization_id = $7 AND id = $8
              AND lifecycle_state = $9 AND provisioning_lease_token = $10
            "#,
        );
        let result = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(&now)
            .bind(GroupKnowledgeSpaceLifecycleState::Failed.as_str())
            .bind(GroupKnowledgeSpaceAclProjectionState::Failed.as_str())
            .bind(truncate_error_code(error_code))
            .bind(updated_by)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding_id)?)
            .bind(GroupKnowledgeSpaceLifecycleState::Provisioning.as_str())
            .bind(provisioning_lease_token)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        if result.rows_affected() != 1 {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "provisioning lease is no longer current".to_string(),
            ));
        }
        let member_updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let member_query = format!(
            r#"
            UPDATE kb_group_knowledge_space_member
            SET status = $1, updated_at = {member_updated_at_expr}, version = version + 1
            WHERE tenant_id = $3 AND organization_id = $4 AND binding_id = $5 AND status = $6
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(member_query.as_str()))
            .bind(INACTIVE_STATUS)
            .bind(&now)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding_id)?)
            .bind(ACTIVE_STATUS)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        let binding = fetch_binding_by_id(&mut transaction, scope, binding_id).await?;
        append_outbox_event(
            &mut transaction,
            &operation_context,
            binding_id,
            "knowledge.group_space.provisioning_failed",
            &group_event_payload(&binding),
        )
        .await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn synchronize_group_members(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupSpaceBindingStoreError> {
        validate_members(&command.members)?;
        validate_scope(command.scope)?;
        validate_group_text(
            "conversation_id",
            &command.conversation_id,
            GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
        )?;
        validate_group_text(
            "group_name",
            &command.group_name,
            GROUP_KNOWLEDGE_SPACE_GROUP_NAME_MAX_LENGTH,
        )?;
        validate_group_text(
            "source_event_id",
            &command.source_event_id,
            GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
        )?;
        validate_group_target(&command.target)?;
        let scope = command.scope;
        let fingerprint = membership_command_fingerprint(&command);
        let now = group_now()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        if let Some(row) = sqlx::query(
            r#"
            SELECT binding_id, payload_sha256_hex
            FROM kb_group_knowledge_space_event_inbox
            WHERE tenant_id = $1 AND organization_id = $2 AND source_event_id = $3
            "#,
        )
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(&command.source_event_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(group_sqlx_error)?
        {
            let stored_fingerprint: String = row
                .try_get("payload_sha256_hex")
                .map_err(group_sqlx_error)?;
            if stored_fingerprint != fingerprint {
                return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "source_event_id was already applied with a different payload".to_string(),
                ));
            }
            let binding_id = row
                .try_get::<Option<i64>, _>("binding_id")
                .map_err(group_sqlx_error)?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::Internal(
                        "membership inbox event has no binding".to_string(),
                    )
                })?;
            let binding = fetch_binding_by_id(
                &mut transaction,
                scope,
                group_from_i64("binding_id", binding_id)?,
            )
            .await?;
            ensure_target_matches_binding(&binding, &command.target)?;
            let current_members = list_active_members(&mut *transaction, scope, binding.id).await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceMembershipChange {
                binding,
                previous_members: current_members.clone(),
                current_members,
                requires_acl_projection: false,
            });
        }

        let mut binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if matches!(
            binding.lifecycle_state,
            GroupKnowledgeSpaceLifecycleState::Archiving
                | GroupKnowledgeSpaceLifecycleState::Archived
                | GroupKnowledgeSpaceLifecycleState::Deleted
        ) {
            let change = record_membership_sync_noop(
                &mut transaction,
                &operation_context,
                binding,
                &command,
                &fingerprint,
                "group_space_membership_sync_terminal_noop",
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(change);
        }
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Provisioning {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                "active group memberships must use the durable ACL projection workflow".to_string(),
            ));
        }
        let previous_members = list_active_members(&mut *transaction, scope, binding.id).await?;
        if command.membership_epoch < binding.membership_epoch
            || command.upstream_link_generation < binding.upstream_link_generation
        {
            let change = record_membership_sync_noop(
                &mut transaction,
                &operation_context,
                binding,
                &command,
                &fingerprint,
                "group_space_membership_sync_stale_noop",
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(change);
        }
        let member_set_changed =
            normalized_members(&previous_members) != normalized_members(&command.members);
        if command.membership_epoch == binding.membership_epoch && member_set_changed {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "membership epoch was already applied with a different member set".to_string(),
            ));
        }
        if member_set_changed {
            replace_active_members(
                &mut transaction,
                &operation_context,
                binding.id,
                command.membership_epoch,
                &command.members,
            )
            .await?;
        }
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$7");
        let query = format!(
            r#"
            UPDATE kb_group_knowledge_space_binding
            SET group_name = $1,
                membership_epoch = $2,
                upstream_link_generation = $3,
                last_source_event_id = $4,
                acl_projection_state = $5,
                updated_by = $6,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $8 AND organization_id = $9 AND id = $10
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(&command.group_name)
            .bind(group_to_i64("membership_epoch", command.membership_epoch)?)
            .bind(group_to_i64(
                "upstream_link_generation",
                command.upstream_link_generation,
            )?)
            .bind(&command.source_event_id)
            .bind(binding.acl_projection_state.as_str())
            .bind("im-membership-sync")
            .bind(&now)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding.id)?)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        append_inbox_event(
            &mut transaction,
            &operation_context,
            binding.id,
            &command.source_event_id,
            "group_space_membership_sync",
            &fingerprint,
        )
        .await?;
        if member_set_changed {
            append_outbox_event(
                &mut transaction,
                &operation_context,
                binding.id,
                "knowledge.group_space.membership_synchronized",
                &group_event_payload(&binding),
            )
            .await?;
        }
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(GroupKnowledgeSpaceMembershipChange {
            binding,
            previous_members,
            current_members: command.members,
            // The service projects the Drive delta before this atomic snapshot replacement, so
            // an active binding never transitions through an `active + pending` state.
            requires_acl_projection: false,
        })
    }

    async fn prepare_group_membership_sync(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
    ) -> Result<GroupKnowledgeSpaceMembershipSyncReservation, KnowledgeGroupSpaceBindingStoreError>
    {
        validate_members(&command.members)?;
        validate_scope(command.scope)?;
        validate_group_text(
            "conversation_id",
            &command.conversation_id,
            GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
        )?;
        validate_group_text(
            "group_name",
            &command.group_name,
            GROUP_KNOWLEDGE_SPACE_GROUP_NAME_MAX_LENGTH,
        )?;
        validate_group_text(
            "source_event_id",
            &command.source_event_id,
            GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
        )?;
        validate_group_target(&command.target)?;

        let scope = command.scope;
        let fingerprint = membership_command_fingerprint(&command);
        let now = group_now()?;
        let lease_until = group_lease_until()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;

        if let Some(row) = sqlx::query(
            r#"
            SELECT binding_id, payload_sha256_hex
            FROM kb_group_knowledge_space_event_inbox
            WHERE tenant_id = $1 AND organization_id = $2 AND source_event_id = $3
            "#,
        )
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(&command.source_event_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(group_sqlx_error)?
        {
            let stored_fingerprint: String = row
                .try_get("payload_sha256_hex")
                .map_err(group_sqlx_error)?;
            if stored_fingerprint != fingerprint {
                return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "source_event_id was already applied with a different payload".to_string(),
                ));
            }
            let binding_id = row
                .try_get::<Option<i64>, _>("binding_id")
                .map_err(group_sqlx_error)?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::Internal(
                        "membership inbox event has no binding".to_string(),
                    )
                })?;
            let binding = fetch_binding_by_id(
                &mut transaction,
                scope,
                group_from_i64("binding_id", binding_id)?,
            )
            .await?;
            ensure_target_matches_binding(&binding, &command.target)?;
            let current_members = list_active_members(&mut *transaction, scope, binding.id).await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceMembershipSyncReservation {
                binding,
                previous_members: current_members.clone(),
                current_members,
                requires_acl_projection: false,
                synchronization_lease_token: None,
            });
        }

        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if matches!(
            binding.lifecycle_state,
            GroupKnowledgeSpaceLifecycleState::Archiving
                | GroupKnowledgeSpaceLifecycleState::Archived
                | GroupKnowledgeSpaceLifecycleState::Deleted
        ) {
            let change = record_membership_sync_noop(
                &mut transaction,
                &operation_context,
                binding,
                &command,
                &fingerprint,
                "group_space_membership_sync_terminal_noop",
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceMembershipSyncReservation {
                binding: change.binding,
                previous_members: change.previous_members,
                current_members: change.current_members,
                requires_acl_projection: false,
                synchronization_lease_token: None,
            });
        }
        if command.membership_epoch < binding.membership_epoch
            || command.upstream_link_generation < binding.upstream_link_generation
        {
            let change = record_membership_sync_noop(
                &mut transaction,
                &operation_context,
                binding,
                &command,
                &fingerprint,
                "group_space_membership_sync_stale_noop",
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceMembershipSyncReservation {
                binding: change.binding,
                previous_members: change.previous_members,
                current_members: change.current_members,
                requires_acl_projection: false,
                synchronization_lease_token: None,
            });
        }
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Active
            || binding.acl_projection_state != GroupKnowledgeSpaceAclProjectionState::Active
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                "only an active group knowledge space with an active ACL projection can synchronize members"
                    .to_string(),
            ));
        }

        if let Some(projection) =
            fetch_membership_projection_by_source(&mut transaction, scope, &command.source_event_id)
                .await?
        {
            if projection.binding_id != binding.id
                || projection.payload_sha256_hex != fingerprint
                || projection.target_membership_epoch != command.membership_epoch
            {
                return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "source_event_id was already reserved with a different membership projection"
                        .to_string(),
                ));
            }
            if projection.projection_state == MEMBERSHIP_PROJECTION_COMPLETED {
                return Err(KnowledgeGroupSpaceBindingStoreError::Internal(
                    "completed membership projection is missing its inbox event".to_string(),
                ));
            }
            let synchronization_lease_token = claim_membership_projection_lease(
                &mut transaction,
                &self.timestamp_dialect,
                scope,
                projection.id,
                binding.id,
                &now,
                &lease_until,
            )
            .await?
            .ok_or_else(|| {
                KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "group membership projection is already being processed".to_string(),
                )
            })?;
            let previous_members =
                list_active_members(&mut *transaction, scope, binding.id).await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceMembershipSyncReservation {
                binding,
                previous_members,
                current_members: command.members,
                requires_acl_projection: true,
                synchronization_lease_token: Some(synchronization_lease_token),
            });
        }

        if fetch_unsettled_membership_projection(&mut transaction, scope, binding.id)
            .await?
            .is_some()
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "a prior group membership projection is not settled".to_string(),
            ));
        }

        let previous_members = list_active_members(&mut *transaction, scope, binding.id).await?;
        let member_set_changed =
            normalized_members(&previous_members) != normalized_members(&command.members);
        if command.membership_epoch == binding.membership_epoch && member_set_changed {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "membership epoch was already applied with a different member set".to_string(),
            ));
        }

        if !member_set_changed {
            update_binding_membership_metadata(
                &mut transaction,
                &operation_context,
                binding.id,
                &command.group_name,
                command.membership_epoch,
                command.upstream_link_generation,
                &command.source_event_id,
            )
            .await?;
            let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
            append_inbox_event(
                &mut transaction,
                &operation_context,
                binding.id,
                &command.source_event_id,
                "group_space_membership_sync",
                &fingerprint,
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceMembershipSyncReservation {
                binding,
                previous_members: previous_members.clone(),
                current_members: previous_members,
                requires_acl_projection: false,
                synchronization_lease_token: None,
            });
        }

        let synchronization_lease_token = insert_membership_projection(
            &mut transaction,
            &operation_context,
            binding.id,
            &command.source_event_id,
            &fingerprint,
            command.membership_epoch,
            &lease_until,
        )
        .await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(GroupKnowledgeSpaceMembershipSyncReservation {
            binding,
            previous_members,
            current_members: command.members,
            requires_acl_projection: true,
            synchronization_lease_token: Some(synchronization_lease_token),
        })
    }

    async fn complete_group_membership_sync(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
    ) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupSpaceBindingStoreError> {
        validate_members(&command.members)?;
        validate_scope(command.scope)?;
        validate_group_text(
            "conversation_id",
            &command.conversation_id,
            GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
        )?;
        validate_group_text(
            "group_name",
            &command.group_name,
            GROUP_KNOWLEDGE_SPACE_GROUP_NAME_MAX_LENGTH,
        )?;
        validate_group_text(
            "source_event_id",
            &command.source_event_id,
            GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
        )?;
        validate_group_target(&command.target)?;
        validate_group_text(
            "synchronization_lease_token",
            synchronization_lease_token,
            64,
        )?;

        let scope = command.scope;
        let fingerprint = membership_command_fingerprint(&command);
        let now = group_now()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if matches!(
            binding.lifecycle_state,
            GroupKnowledgeSpaceLifecycleState::Archiving
                | GroupKnowledgeSpaceLifecycleState::Archived
                | GroupKnowledgeSpaceLifecycleState::Deleted
        ) {
            let change = record_membership_sync_noop(
                &mut transaction,
                &operation_context,
                binding,
                &command,
                &fingerprint,
                "group_space_membership_sync_terminal_noop",
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(change);
        }
        if command.membership_epoch < binding.membership_epoch
            || command.upstream_link_generation < binding.upstream_link_generation
        {
            let change = record_membership_sync_noop(
                &mut transaction,
                &operation_context,
                binding,
                &command,
                &fingerprint,
                "group_space_membership_sync_stale_noop",
            )
            .await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(change);
        }
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Active
            || binding.acl_projection_state != GroupKnowledgeSpaceAclProjectionState::Active
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                "group membership projection can only complete for an active binding".to_string(),
            ));
        }
        let projection = fetch_membership_projection_by_source(
            &mut transaction,
            scope,
            &command.source_event_id,
        )
        .await?
        .ok_or_else(|| {
            KnowledgeGroupSpaceBindingStoreError::NotFound(
                "group membership projection".to_string(),
            )
        })?;
        if projection.binding_id != binding.id
            || projection.payload_sha256_hex != fingerprint
            || projection.target_membership_epoch != command.membership_epoch
            || projection.projection_state != MEMBERSHIP_PROJECTION_PENDING
            || projection.projection_lease_token.as_deref() != Some(synchronization_lease_token)
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group membership projection lease is no longer current".to_string(),
            ));
        }
        let previous_members = list_active_members(&mut *transaction, scope, binding.id).await?;
        let member_set_changed =
            normalized_members(&previous_members) != normalized_members(&command.members);
        if command.membership_epoch == binding.membership_epoch && member_set_changed {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "membership epoch was already applied with a different member set".to_string(),
            ));
        }

        replace_active_members(
            &mut transaction,
            &operation_context,
            binding.id,
            command.membership_epoch,
            &command.members,
        )
        .await?;
        update_binding_membership_metadata(
            &mut transaction,
            &operation_context,
            binding.id,
            &command.group_name,
            command.membership_epoch,
            command.upstream_link_generation,
            &command.source_event_id,
        )
        .await?;
        mark_membership_projection_completed(
            &mut transaction,
            &self.timestamp_dialect,
            scope,
            projection.id,
            binding.id,
            synchronization_lease_token,
            &now,
        )
        .await?;
        let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        append_inbox_event(
            &mut transaction,
            &operation_context,
            binding.id,
            &command.source_event_id,
            "group_space_membership_sync",
            &fingerprint,
        )
        .await?;
        append_outbox_event(
            &mut transaction,
            &operation_context,
            binding.id,
            "knowledge.group_space.membership_synchronized",
            &group_event_payload(&binding),
        )
        .await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(GroupKnowledgeSpaceMembershipChange {
            binding,
            previous_members,
            current_members: command.members,
            requires_acl_projection: false,
        })
    }

    async fn fail_group_membership_sync(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
        error_code: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_scope(command.scope)?;
        validate_group_text(
            "conversation_id",
            &command.conversation_id,
            GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
        )?;
        validate_group_text(
            "source_event_id",
            &command.source_event_id,
            GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
        )?;
        validate_group_target(&command.target)?;
        validate_group_text(
            "synchronization_lease_token",
            synchronization_lease_token,
            64,
        )?;
        validate_group_text("error_code", error_code, 64)?;

        let scope = command.scope;
        let now = group_now()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if matches!(
            binding.lifecycle_state,
            GroupKnowledgeSpaceLifecycleState::Archiving
                | GroupKnowledgeSpaceLifecycleState::Archived
                | GroupKnowledgeSpaceLifecycleState::Deleted
        ) {
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(binding);
        }
        let projection = fetch_membership_projection_by_source(
            &mut transaction,
            scope,
            &command.source_event_id,
        )
        .await?
        .ok_or_else(|| {
            KnowledgeGroupSpaceBindingStoreError::NotFound(
                "group membership projection".to_string(),
            )
        })?;
        if projection.binding_id != binding.id
            || projection.projection_state != MEMBERSHIP_PROJECTION_PENDING
            || projection.projection_lease_token.as_deref() != Some(synchronization_lease_token)
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group membership projection lease is no longer current".to_string(),
            ));
        }
        mark_membership_projection_failed(
            &mut transaction,
            &operation_context,
            projection.id,
            binding.id,
            synchronization_lease_token,
            error_code,
        )
        .await?;
        update_binding_membership_projection_error(
            &mut transaction,
            &self.timestamp_dialect,
            scope,
            binding.id,
            error_code,
            &now,
        )
        .await?;
        let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn settle_group_membership_sync_after_archive(
        &self,
        command: SynchronizeGroupKnowledgeSpaceMembersCommand,
        synchronization_lease_token: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_membership_command(&command)?;
        validate_group_text(
            "synchronization_lease_token",
            synchronization_lease_token,
            64,
        )?;

        let scope = command.scope;
        let now = group_now()?;
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if !matches!(
            binding.lifecycle_state,
            GroupKnowledgeSpaceLifecycleState::Archiving
                | GroupKnowledgeSpaceLifecycleState::Archived
                | GroupKnowledgeSpaceLifecycleState::Deleted
        ) {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                "archive compensation can only settle an archived group membership projection"
                    .to_string(),
            ));
        }
        let projection = fetch_membership_projection_by_source(
            &mut transaction,
            scope,
            &command.source_event_id,
        )
        .await?
        .ok_or_else(|| {
            KnowledgeGroupSpaceBindingStoreError::NotFound(
                "group membership projection".to_string(),
            )
        })?;
        if projection.binding_id != binding.id
            || projection.projection_state != MEMBERSHIP_PROJECTION_PENDING
            || projection.projection_lease_token.as_deref() != Some(synchronization_lease_token)
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group membership projection lease is no longer current".to_string(),
            ));
        }
        mark_membership_projection_completed(
            &mut transaction,
            &self.timestamp_dialect,
            scope,
            projection.id,
            binding.id,
            synchronization_lease_token,
            &now,
        )
        .await?;
        let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn mark_acl_projection_active(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
    ) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
        update_acl_projection_state(
            &self.pool,
            &self.timestamp_dialect,
            scope,
            binding_id,
            GroupKnowledgeSpaceAclProjectionState::Active,
            None,
        )
        .await
    }

    async fn mark_acl_projection_failed(
        &self,
        scope: GroupKnowledgeSpaceScope,
        binding_id: u64,
        error_code: &str,
    ) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
        update_acl_projection_state(
            &self.pool,
            &self.timestamp_dialect,
            scope,
            binding_id,
            GroupKnowledgeSpaceAclProjectionState::Failed,
            Some(error_code),
        )
        .await
    }

    async fn list_resumable_group_space_archives(
        &self,
        scope: GroupKnowledgeSpaceScope,
        limit: u32,
    ) -> Result<Vec<ArchiveGroupKnowledgeSpaceCommand>, KnowledgeGroupSpaceBindingStoreError> {
        self.list_resumable_group_space_archives_for_scope(
            scope.tenant_id,
            scope.organization_id,
            limit,
        )
        .await
    }

    async fn list_resumable_group_space_archives_for_tenant(
        &self,
        tenant_id: u64,
        organization_id: u64,
        limit: u32,
    ) -> Result<Vec<ArchiveGroupKnowledgeSpaceCommand>, KnowledgeGroupSpaceBindingStoreError> {
        self.list_resumable_group_space_archives_for_scope(tenant_id, organization_id, limit)
            .await
    }

    async fn begin_group_space_archive(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
    ) -> Result<GroupKnowledgeSpaceArchiveReservation, KnowledgeGroupSpaceBindingStoreError> {
        validate_archive_command(&command)?;
        let scope = command.scope;
        let fingerprint = archive_command_fingerprint(&command);
        let now = group_now()?;
        let lease_until = group_lease_until()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;

        if let Some(row) = sqlx::query(
            r#"
            SELECT binding_id, payload_sha256_hex
            FROM kb_group_knowledge_space_event_inbox
            WHERE tenant_id = $1 AND organization_id = $2 AND source_event_id = $3
            "#,
        )
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(&command.source_event_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(group_sqlx_error)?
        {
            let existing_fingerprint: String = row
                .try_get("payload_sha256_hex")
                .map_err(group_sqlx_error)?;
            if existing_fingerprint != fingerprint {
                return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                    "source_event_id was already applied with a different payload".to_string(),
                ));
            }
            let binding_id = row
                .try_get::<Option<i64>, _>("binding_id")
                .map_err(group_sqlx_error)?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::Internal(
                        "archive inbox event has no binding".to_string(),
                    )
                })?;
            let binding = fetch_binding_by_id(
                &mut transaction,
                scope,
                group_from_i64("binding_id", binding_id)?,
            )
            .await?;
            ensure_target_matches_binding(&binding, &command.target)?;
            let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(GroupKnowledgeSpaceArchiveReservation {
                binding,
                space,
                requires_archive: false,
                archive_lease_token: None,
            });
        }

        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;

        match binding.lifecycle_state {
            GroupKnowledgeSpaceLifecycleState::Archived
            | GroupKnowledgeSpaceLifecycleState::Deleted => {
                append_inbox_event(
                    &mut transaction,
                    &operation_context,
                    binding.id,
                    &command.source_event_id,
                    "group_space_archive_terminal_noop",
                    &fingerprint,
                )
                .await?;
                let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
                transaction.commit().await.map_err(group_sqlx_error)?;
                Ok(GroupKnowledgeSpaceArchiveReservation {
                    binding,
                    space,
                    requires_archive: false,
                    archive_lease_token: None,
                })
            }
            GroupKnowledgeSpaceLifecycleState::Archiving => {
                if binding.archive_source_event_id.as_deref()
                    != Some(command.source_event_id.as_str())
                    || binding.archive_payload_sha256_hex.as_deref() != Some(fingerprint.as_str())
                {
                    return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                        "a different archive event is already converging for this group knowledge space"
                            .to_string(),
                    ));
                }
                let archive_lease_token = claim_group_space_archive_lease(
                    &mut transaction,
                    &operation_context,
                    binding.id,
                    &command.source_event_id,
                    &fingerprint,
                    &lease_until,
                )
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::Conflict(
                        "group archive is already being processed".to_string(),
                    )
                })?;
                cancel_membership_projections_for_archive(
                    &mut transaction,
                    &self.timestamp_dialect,
                    scope,
                    binding.id,
                    &now,
                )
                .await?;
                let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
                let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
                transaction.commit().await.map_err(group_sqlx_error)?;
                Ok(GroupKnowledgeSpaceArchiveReservation {
                    binding,
                    space,
                    requires_archive: true,
                    archive_lease_token: Some(archive_lease_token),
                })
            }
            GroupKnowledgeSpaceLifecycleState::Active
            | GroupKnowledgeSpaceLifecycleState::Provisioning => {
                let archive_lease_token = Uuid::new_v4().to_string();
                let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$9");
                let archive_lease_until_expr = self.timestamp_dialect.sql_timestamp_expr("$6");
                let query = format!(
                    r#"
                    UPDATE kb_group_knowledge_space_binding
                    SET lifecycle_state = $1,
                        acl_projection_state = $2,
                        archive_source_event_id = $3,
                        archive_payload_sha256_hex = $4,
                        archive_lease_token = $5,
                        archive_lease_until = {archive_lease_until_expr},
                        archive_acl_cursor = NULL,
                        archive_acl_pages_processed = 0,
                        archive_acl_cleanup_completed_at = NULL,
                        archived_by = $7,
                        upstream_link_generation = $8,
                        last_source_event_id = $3,
                        last_error_code = NULL,
                        last_error_at = NULL,
                        updated_by = $7,
                        updated_at = {updated_at_expr},
                        version = version + 1
                    WHERE tenant_id = $10 AND organization_id = $11 AND id = $12
                      AND lifecycle_state = $13
                    "#,
                );
                let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                    .bind(GroupKnowledgeSpaceLifecycleState::Archiving.as_str())
                    .bind(GroupKnowledgeSpaceAclProjectionState::Pending.as_str())
                    .bind(&command.source_event_id)
                    .bind(&fingerprint)
                    .bind(&archive_lease_token)
                    .bind(&lease_until)
                    .bind(&command.archived_by)
                    .bind(group_to_i64(
                        "upstream_link_generation",
                        command.upstream_link_generation,
                    )?)
                    .bind(&now)
                    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
                    .bind(group_to_i64("organization_id", scope.organization_id)?)
                    .bind(group_to_i64("binding_id", binding.id)?)
                    .bind(binding.lifecycle_state.as_str())
                    .execute(&mut *transaction)
                    .await
                    .map_err(group_sqlx_error)?;
                if updated.rows_affected() != 1 {
                    return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                        "group knowledge space changed before archive could start".to_string(),
                    ));
                }
                cancel_membership_projections_for_archive(
                    &mut transaction,
                    &self.timestamp_dialect,
                    scope,
                    binding.id,
                    &now,
                )
                .await?;
                let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
                let space = fetch_binding_space(&mut transaction, scope, &binding).await?;
                transaction.commit().await.map_err(group_sqlx_error)?;
                Ok(GroupKnowledgeSpaceArchiveReservation {
                    binding,
                    space,
                    requires_archive: true,
                    archive_lease_token: Some(archive_lease_token),
                })
            }
            GroupKnowledgeSpaceLifecycleState::Failed => {
                Err(KnowledgeGroupSpaceBindingStoreError::InvalidLifecycle(
                    "a failed group knowledge space has no archiveable target".to_string(),
                ))
            }
        }
    }

    async fn complete_group_space_archive(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
        archive_lease_token: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_archive_command(&command)?;
        validate_group_text("archive_lease_token", archive_lease_token, 64)?;
        let scope = command.scope;
        let fingerprint = archive_command_fingerprint(&command);
        let now = group_now()?;
        let operation_context = GroupKnowledgeSpaceStoreOperationContext {
            id_generator: &self.id_generator,
            timestamp_dialect: &self.timestamp_dialect,
            scope,
            now: &now,
        };
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if binding.lifecycle_state == GroupKnowledgeSpaceLifecycleState::Archived
            && binding.archive_source_event_id.as_deref() == Some(command.source_event_id.as_str())
            && binding.archive_payload_sha256_hex.as_deref() == Some(fingerprint.as_str())
        {
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(binding);
        }
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Archiving
            || binding.archive_source_event_id.as_deref() != Some(command.source_event_id.as_str())
            || binding.archive_payload_sha256_hex.as_deref() != Some(fingerprint.as_str())
            || binding.archive_lease_token.as_deref() != Some(archive_lease_token)
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive lease is no longer current".to_string(),
            ));
        }

        let active_projection_expiry_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let active_projection_query = format!(
            r#"
            SELECT COUNT(*) AS projection_count
            FROM kb_group_knowledge_space_membership_projection
            WHERE tenant_id = $1 AND organization_id = $2 AND binding_id = $3
              AND projection_state = $5
              AND projection_lease_token IS NOT NULL
              AND projection_lease_until IS NOT NULL
              AND projection_lease_until >= {active_projection_expiry_expr}
            "#,
        );
        let active_projection_row =
            sqlx::query(sqlx::AssertSqlSafe(active_projection_query.as_str()))
                .bind(group_to_i64("tenant_id", scope.tenant_id)?)
                .bind(group_to_i64("organization_id", scope.organization_id)?)
                .bind(group_to_i64("binding_id", binding.id)?)
                .bind(&now)
                .bind(MEMBERSHIP_PROJECTION_PENDING)
                .fetch_one(&mut *transaction)
                .await
                .map_err(group_sqlx_error)?;
        let active_projection_count: i64 = active_projection_row
            .try_get("projection_count")
            .map_err(group_sqlx_error)?;
        if active_projection_count > 0 {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive cannot finalize while an external membership ACL projection is active"
                    .to_string(),
            ));
        }

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let archived_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let query = format!(
            r#"
            UPDATE kb_group_knowledge_space_binding
            SET lifecycle_state = $1,
                acl_projection_state = $2,
                archive_lease_token = NULL,
                archive_lease_until = NULL,
                archived_at = {archived_at_expr},
                last_source_event_id = $3,
                last_error_code = NULL,
                last_error_at = NULL,
                updated_by = $5,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $6 AND organization_id = $7 AND id = $8
              AND lifecycle_state = $9 AND archive_lease_token = $10
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(GroupKnowledgeSpaceLifecycleState::Archived.as_str())
            .bind(GroupKnowledgeSpaceAclProjectionState::Pending.as_str())
            .bind(&command.source_event_id)
            .bind(&now)
            .bind(&command.archived_by)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding.id)?)
            .bind(GroupKnowledgeSpaceLifecycleState::Archiving.as_str())
            .bind(archive_lease_token)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive lease was replaced before completion".to_string(),
            ));
        }
        cancel_membership_projections_for_archive(
            &mut transaction,
            &self.timestamp_dialect,
            scope,
            binding.id,
            &now,
        )
        .await?;
        deactivate_group_members_for_archive(
            &mut transaction,
            &self.timestamp_dialect,
            scope,
            binding.id,
            &now,
        )
        .await?;
        let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        append_inbox_event(
            &mut transaction,
            &operation_context,
            binding.id,
            &command.source_event_id,
            "group_space_archive",
            &fingerprint,
        )
        .await?;
        append_outbox_event(
            &mut transaction,
            &operation_context,
            binding.id,
            "knowledge.group_space.archived",
            &group_event_payload(&binding),
        )
        .await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn advance_group_space_archive_acl_cleanup(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
        archive_lease_token: &str,
        next_cursor: Option<String>,
        cleanup_completed: bool,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        const MAX_ARCHIVE_ACL_CURSOR_LENGTH: usize = 2048;
        const MAX_ARCHIVE_ACL_PAGES: u64 = 100_000;

        validate_archive_command(&command)?;
        validate_group_text("archive_lease_token", archive_lease_token, 64)?;
        if let Some(cursor) = next_cursor.as_deref() {
            validate_group_text("archive_acl_cursor", cursor, MAX_ARCHIVE_ACL_CURSOR_LENGTH)?;
        }
        if cleanup_completed && next_cursor.is_some() {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "completed archive ACL cleanup cannot retain a pagination cursor".to_string(),
            ));
        }
        let scope = command.scope;
        let fingerprint = archive_command_fingerprint(&command);
        let now = group_now()?;
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Archiving
            || binding.archive_source_event_id.as_deref() != Some(command.source_event_id.as_str())
            || binding.archive_payload_sha256_hex.as_deref() != Some(fingerprint.as_str())
            || binding.archive_lease_token.as_deref() != Some(archive_lease_token)
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive lease is no longer current".to_string(),
            ));
        }
        if binding.archive_acl_pages_processed >= MAX_ARCHIVE_ACL_PAGES {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "archive ACL cleanup exceeded its durable safety bound".to_string(),
            ));
        }
        if !cleanup_completed && next_cursor.is_some() && next_cursor == binding.archive_acl_cursor
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "archive ACL pagination cursor did not advance".to_string(),
            ));
        }

        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let cleanup_completed_at_expr = self.timestamp_dialect.sql_timestamp_expr("$4");
        let query = format!(
            r#"
            UPDATE kb_group_knowledge_space_binding
            SET archive_acl_cursor = $1,
                archive_acl_pages_processed = archive_acl_pages_processed + 1,
                archive_acl_cleanup_completed_at = CASE
                    WHEN $2 = 1 THEN {cleanup_completed_at_expr}
                    ELSE NULL
                END,
                archive_lease_token = CASE WHEN $2 = 1 THEN archive_lease_token ELSE NULL END,
                archive_lease_until = CASE WHEN $2 = 1 THEN archive_lease_until ELSE NULL END,
                updated_by = $3,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $5 AND organization_id = $6 AND id = $7
              AND lifecycle_state = $8 AND archive_lease_token = $9
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(next_cursor)
            .bind(if cleanup_completed { 1_i64 } else { 0_i64 })
            .bind(&command.archived_by)
            .bind(&now)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding.id)?)
            .bind(GroupKnowledgeSpaceLifecycleState::Archiving.as_str())
            .bind(archive_lease_token)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive lease changed before ACL progress persisted".to_string(),
            ));
        }
        let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }

    async fn release_group_space_archive_lease(
        &self,
        command: ArchiveGroupKnowledgeSpaceCommand,
        archive_lease_token: &str,
        error_code: &str,
    ) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
        validate_archive_command(&command)?;
        validate_group_text("archive_lease_token", archive_lease_token, 64)?;
        validate_group_text("error_code", error_code, 64)?;
        let scope = command.scope;
        let fingerprint = archive_command_fingerprint(&command);
        let now = group_now()?;
        let mut transaction = self.pool.begin().await.map_err(group_sqlx_error)?;
        let binding =
            fetch_binding_by_conversation(&mut transaction, scope, &command.conversation_id)
                .await?
                .ok_or_else(|| {
                    KnowledgeGroupSpaceBindingStoreError::NotFound(command.conversation_id.clone())
                })?;
        ensure_target_matches_binding(&binding, &command.target)?;
        if binding.lifecycle_state == GroupKnowledgeSpaceLifecycleState::Archived
            && binding.archive_source_event_id.as_deref() == Some(command.source_event_id.as_str())
            && binding.archive_payload_sha256_hex.as_deref() == Some(fingerprint.as_str())
        {
            transaction.commit().await.map_err(group_sqlx_error)?;
            return Ok(binding);
        }
        if binding.lifecycle_state != GroupKnowledgeSpaceLifecycleState::Archiving
            || binding.archive_source_event_id.as_deref() != Some(command.source_event_id.as_str())
            || binding.archive_payload_sha256_hex.as_deref() != Some(fingerprint.as_str())
            || binding.archive_lease_token.as_deref() != Some(archive_lease_token)
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive lease is no longer current".to_string(),
            ));
        }
        let updated_at_expr = self.timestamp_dialect.sql_timestamp_expr("$2");
        let query = format!(
            r#"
            UPDATE kb_group_knowledge_space_binding
            SET archive_lease_token = NULL,
                archive_lease_until = NULL,
                last_error_code = $1,
                last_error_at = {updated_at_expr},
                updated_by = $3,
                updated_at = {updated_at_expr},
                version = version + 1
            WHERE tenant_id = $4 AND organization_id = $5 AND id = $6
              AND lifecycle_state = $7 AND archive_lease_token = $8
            "#,
        );
        let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(truncate_error_code(error_code))
            .bind(&now)
            .bind(&command.archived_by)
            .bind(group_to_i64("tenant_id", scope.tenant_id)?)
            .bind(group_to_i64("organization_id", scope.organization_id)?)
            .bind(group_to_i64("binding_id", binding.id)?)
            .bind(GroupKnowledgeSpaceLifecycleState::Archiving.as_str())
            .bind(archive_lease_token)
            .execute(&mut *transaction)
            .await
            .map_err(group_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
                "group archive lease was replaced before release".to_string(),
            ));
        }
        let binding = fetch_binding_by_id(&mut transaction, scope, binding.id).await?;
        transaction.commit().await.map_err(group_sqlx_error)?;
        Ok(binding)
    }
}

async fn reset_binding_for_provisioning(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding: GroupKnowledgeSpaceBinding,
    request: &ReserveGroupKnowledgeSpaceRequest,
    lease_until: &str,
) -> Result<
    (GroupKnowledgeSpaceBinding, KnowledgeSpace, String),
    KnowledgeGroupSpaceBindingStoreError,
> {
    let space = insert_group_space(
        transaction,
        operation_context.id_generator,
        operation_context.timestamp_dialect,
        operation_context.scope,
        &request.group_name,
        operation_context.now,
    )
    .await?;
    let token = Uuid::new_v4().to_string();
    let idempotency_key_hash = sha256_hash(request.provisioning_idempotency_key.as_bytes());
    let updated_at_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$1");
    let lease_until_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$9");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_binding
        SET space_id = $2,
            space_uuid = $3,
            group_name = $4,
            lifecycle_state = $5,
            acl_projection_state = $6,
            provisioning_idempotency_key_sha256_hex = $7,
            provisioning_lease_token = $8,
            provisioning_lease_until = {lease_until_expr},
            membership_epoch = $9,
            last_source_event_id = $10,
            last_error_code = NULL,
            last_error_at = NULL,
            archived_at = NULL,
            archived_by = NULL,
            deleted_at = NULL,
            deleted_by = NULL,
            updated_by = $11,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $12 AND organization_id = $13 AND id = $14
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(operation_context.now)
        .bind(group_to_i64("space_id", space.id)?)
        .bind(&space.uuid)
        .bind(&request.group_name)
        .bind(GroupKnowledgeSpaceLifecycleState::Provisioning.as_str())
        .bind(GroupKnowledgeSpaceAclProjectionState::Pending.as_str())
        .bind(idempotency_key_hash)
        .bind(&token)
        .bind(lease_until)
        .bind(group_to_i64("membership_epoch", request.membership_epoch)?)
        .bind(&request.source_event_id)
        .bind(&request.created_by)
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(group_to_i64("binding_id", binding.id)?)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    replace_active_members(
        transaction,
        operation_context,
        binding.id,
        request.membership_epoch,
        &request.members,
    )
    .await?;
    let binding = fetch_binding_by_id(transaction, operation_context.scope, binding.id).await?;
    Ok((binding, space, token))
}

async fn claim_provisioning_lease(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
    updated_by: &str,
    now: &str,
    lease_until: &str,
) -> Result<Option<String>, KnowledgeGroupSpaceBindingStoreError> {
    let token = Uuid::new_v4().to_string();
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$1");
    let lease_until_expr = timestamp_dialect.sql_timestamp_expr("$3");
    let lease_expiry_expr = timestamp_dialect.sql_timestamp_expr("$1");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_binding
        SET provisioning_lease_token = $2,
            provisioning_lease_until = {lease_until_expr},
            updated_by = $4,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $5 AND organization_id = $6 AND id = $7
          AND lifecycle_state = $8
          AND (provisioning_lease_until IS NULL OR provisioning_lease_until < {lease_expiry_expr})
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(now)
        .bind(&token)
        .bind(lease_until)
        .bind(updated_by)
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(GroupKnowledgeSpaceLifecycleState::Provisioning.as_str())
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok((updated.rows_affected() == 1).then_some(token))
}

async fn insert_group_space(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    group_name: &str,
    now: &str,
) -> Result<KnowledgeSpace, KnowledgeGroupSpaceBindingStoreError> {
    let id = next_group_id(id_generator)?;
    let uuid = Uuid::new_v4().to_string();
    let created_at_expr = timestamp_dialect.sql_timestamp_expr("$9");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$10");
    let query = format!(
        r#"
        INSERT INTO kb_space (
            id, uuid, tenant_id, organization_id, name, description, drive_space_id, status,
            okf_bundle_initialized, knowledge_mode, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, NULL, NULL, $6, $7, $8, {created_at_expr}, {updated_at_expr}, $11)
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(group_to_i64("space_id", id)?)
        .bind(&uuid)
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_name)
        .bind(SPACE_PROVISIONING_STATUS)
        .bind(INACTIVE_STATUS)
        .bind(KnowledgeAgentKnowledgeMode::OkfBundle.as_str())
        .bind(now)
        .bind(now)
        .bind(INITIAL_VERSION)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok(KnowledgeSpace {
        id,
        uuid,
        name: group_name.to_string(),
        description: None,
        drive_space_id: None,
        status: KnowledgeSpaceStatus::Provisioning,
        okf_bundle_initialized: false,
        knowledge_mode: KnowledgeAgentKnowledgeMode::OkfBundle,
    })
}

async fn replace_active_members(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding_id: u64,
    membership_epoch: u64,
    members: &[GroupKnowledgeSpaceMember],
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    validate_members(members)?;
    let updated_at_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$2");
    let deactivate_query = format!(
        r#"
        UPDATE kb_group_knowledge_space_member
        SET status = $1, updated_at = {updated_at_expr}, version = version + 1
        WHERE tenant_id = $3 AND organization_id = $4 AND binding_id = $5 AND status = $6
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(deactivate_query.as_str()))
        .bind(INACTIVE_STATUS)
        .bind(operation_context.now)
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(ACTIVE_STATUS)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;

    for member in members {
        let id = next_group_id(operation_context.id_generator)?;
        let created_at_expr = operation_context
            .timestamp_dialect
            .sql_timestamp_expr("$12");
        let updated_at_expr = operation_context
            .timestamp_dialect
            .sql_timestamp_expr("$13");
        let query = format!(
            r#"
            INSERT INTO kb_group_knowledge_space_member (
                id, uuid, tenant_id, organization_id, binding_id, principal_kind, actor_id,
                member_role, access_level, membership_epoch, status, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, {created_at_expr}, {updated_at_expr}, $14)
            "#,
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(group_to_i64("member_id", id)?)
            .bind(Uuid::new_v4().to_string())
            .bind(group_to_i64(
                "tenant_id",
                operation_context.scope.tenant_id,
            )?)
            .bind(group_to_i64(
                "organization_id",
                operation_context.scope.organization_id,
            )?)
            .bind(group_to_i64("binding_id", binding_id)?)
            .bind(member.principal_kind.as_str())
            .bind(&member.actor_id)
            .bind(member.role.as_str())
            .bind(member.role.access_level().map(|level| level.as_str()))
            .bind(group_to_i64("membership_epoch", membership_epoch)?)
            .bind(ACTIVE_STATUS)
            .bind(operation_context.now)
            .bind(operation_context.now)
            .bind(INITIAL_VERSION)
            .execute(&mut **transaction)
            .await
            .map_err(group_sqlx_error)?;
    }
    Ok(())
}

async fn append_inbox_event(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding_id: u64,
    source_event_id: &str,
    event_type: &str,
    payload_sha256_hex: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let id = next_group_id(operation_context.id_generator)?;
    let applied_at_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$9");
    let query = format!(
        r#"
        INSERT INTO kb_group_knowledge_space_event_inbox (
            id, uuid, tenant_id, organization_id, source_event_id, event_type, binding_id,
            payload_sha256_hex, applied_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, {applied_at_expr})
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(group_to_i64("inbox_id", id)?)
        .bind(Uuid::new_v4().to_string())
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(source_event_id)
        .bind(event_type)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(payload_sha256_hex)
        .bind(operation_context.now)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok(())
}

async fn record_membership_sync_noop(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding: GroupKnowledgeSpaceBinding,
    command: &SynchronizeGroupKnowledgeSpaceMembersCommand,
    fingerprint: &str,
    event_type: &str,
) -> Result<GroupKnowledgeSpaceMembershipChange, KnowledgeGroupSpaceBindingStoreError> {
    let current_members =
        list_active_members(&mut **transaction, operation_context.scope, binding.id).await?;
    append_inbox_event(
        transaction,
        operation_context,
        binding.id,
        &command.source_event_id,
        event_type,
        fingerprint,
    )
    .await?;
    Ok(GroupKnowledgeSpaceMembershipChange {
        binding,
        previous_members: current_members.clone(),
        current_members,
        requires_acl_projection: false,
    })
}

async fn cancel_membership_projections_for_archive(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
    now: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let expiry_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_membership_projection
        SET projection_state = $1,
            projection_lease_token = NULL,
            projection_lease_until = NULL,
            last_error_code = $3,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $4 AND organization_id = $5 AND binding_id = $6
          AND projection_state IN ($7, $8)
          AND (
              projection_lease_token IS NULL
              OR projection_lease_until IS NULL
              OR projection_lease_until < {expiry_expr}
          )
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(MEMBERSHIP_PROJECTION_COMPLETED)
        .bind(now)
        .bind("group_space_archive_started")
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(MEMBERSHIP_PROJECTION_FAILED)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok(())
}

async fn deactivate_group_members_for_archive(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
    now: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_member
        SET status = $1,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $3 AND organization_id = $4 AND binding_id = $5 AND status = $6
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(INACTIVE_STATUS)
        .bind(now)
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(ACTIVE_STATUS)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok(())
}

async fn claim_group_space_archive_lease(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding_id: u64,
    source_event_id: &str,
    fingerprint: &str,
    lease_until: &str,
) -> Result<Option<String>, KnowledgeGroupSpaceBindingStoreError> {
    let lease_token = Uuid::new_v4().to_string();
    let lease_until_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$2");
    let updated_at_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$3");
    let expiry_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$3");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_binding
        SET archive_lease_token = $1,
            archive_lease_until = {lease_until_expr},
            updated_by = 'group-archive-saga',
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $4 AND organization_id = $5 AND id = $6
          AND lifecycle_state = $7
          AND archive_source_event_id = $8
          AND archive_payload_sha256_hex = $9
          AND (archive_lease_until IS NULL OR archive_lease_until < {expiry_expr})
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(&lease_token)
        .bind(lease_until)
        .bind(operation_context.now)
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(GroupKnowledgeSpaceLifecycleState::Archiving.as_str())
        .bind(source_event_id)
        .bind(fingerprint)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok((updated.rows_affected() == 1).then_some(lease_token))
}

async fn insert_membership_projection(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding_id: u64,
    source_event_id: &str,
    payload_sha256_hex: &str,
    target_membership_epoch: u64,
    lease_until: &str,
) -> Result<String, KnowledgeGroupSpaceBindingStoreError> {
    let id = next_group_id(operation_context.id_generator)?;
    let lease_token = Uuid::new_v4().to_string();
    let lease_until_expr = operation_context
        .timestamp_dialect
        .sql_timestamp_expr("$11");
    let created_at_expr = operation_context
        .timestamp_dialect
        .sql_timestamp_expr("$13");
    let updated_at_expr = operation_context
        .timestamp_dialect
        .sql_timestamp_expr("$14");
    let query = format!(
        r#"
        INSERT INTO kb_group_knowledge_space_membership_projection (
            id, uuid, tenant_id, organization_id, binding_id, source_event_id,
            payload_sha256_hex, target_membership_epoch, projection_state,
            projection_lease_token, projection_lease_until, last_error_code,
            created_at, updated_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, {lease_until_expr}, $12,
                {created_at_expr}, {updated_at_expr}, $15)
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(group_to_i64("membership_projection_id", id)?)
        .bind(Uuid::new_v4().to_string())
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(source_event_id)
        .bind(payload_sha256_hex)
        .bind(group_to_i64(
            "target_membership_epoch",
            target_membership_epoch,
        )?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(&lease_token)
        .bind(lease_until)
        .bind(None::<String>)
        .bind(operation_context.now)
        .bind(operation_context.now)
        .bind(INITIAL_VERSION)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok(lease_token)
}

async fn claim_membership_projection_lease(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    projection_id: u64,
    binding_id: u64,
    now: &str,
    lease_until: &str,
) -> Result<Option<String>, KnowledgeGroupSpaceBindingStoreError> {
    let lease_token = Uuid::new_v4().to_string();
    let lease_until_expr = timestamp_dialect.sql_timestamp_expr("$3");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$4");
    let expiry_expr = timestamp_dialect.sql_timestamp_expr("$4");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_membership_projection
        SET projection_state = $1,
            projection_lease_token = $2,
            projection_lease_until = {lease_until_expr},
            last_error_code = NULL,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $5 AND organization_id = $6 AND id = $7 AND binding_id = $8
          AND projection_state IN ($9, $10)
          AND (projection_lease_until IS NULL OR projection_lease_until < {expiry_expr})
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(&lease_token)
        .bind(lease_until)
        .bind(now)
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("membership_projection_id", projection_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(MEMBERSHIP_PROJECTION_FAILED)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok((updated.rows_affected() == 1).then_some(lease_token))
}

async fn mark_membership_projection_completed(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    projection_id: u64,
    binding_id: u64,
    synchronization_lease_token: &str,
    now: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$1");
    let expiry_expr = timestamp_dialect.sql_timestamp_expr("$1");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_membership_projection
        SET projection_state = $2,
            projection_lease_token = NULL,
            projection_lease_until = NULL,
            last_error_code = NULL,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $3 AND organization_id = $4 AND id = $5 AND binding_id = $6
          AND projection_state = $7
          AND projection_lease_token = $8
          AND projection_lease_until >= {expiry_expr}
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(now)
        .bind(MEMBERSHIP_PROJECTION_COMPLETED)
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("membership_projection_id", projection_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(synchronization_lease_token)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
            "group membership projection lease expired or was replaced".to_string(),
        ));
    }
    Ok(())
}

async fn mark_membership_projection_failed(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    projection_id: u64,
    binding_id: u64,
    synchronization_lease_token: &str,
    error_code: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let updated_at_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$3");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_membership_projection
        SET projection_state = $1,
            projection_lease_token = NULL,
            projection_lease_until = NULL,
            last_error_code = $2,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $4 AND organization_id = $5 AND id = $6 AND binding_id = $7
          AND projection_state = $8
          AND projection_lease_token = $9
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(MEMBERSHIP_PROJECTION_FAILED)
        .bind(truncate_error_code(error_code))
        .bind(operation_context.now)
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(group_to_i64("membership_projection_id", projection_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(MEMBERSHIP_PROJECTION_PENDING)
        .bind(synchronization_lease_token)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
            "group membership projection lease is no longer current".to_string(),
        ));
    }
    Ok(())
}

async fn update_binding_membership_metadata(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding_id: u64,
    group_name: &str,
    membership_epoch: u64,
    upstream_link_generation: u64,
    source_event_id: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let updated_at_expr = operation_context.timestamp_dialect.sql_timestamp_expr("$6");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_binding
        SET group_name = $1,
            membership_epoch = $2,
            upstream_link_generation = $3,
            last_source_event_id = $4,
            last_error_code = NULL,
            last_error_at = NULL,
            updated_by = $5,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $7 AND organization_id = $8 AND id = $9
          AND lifecycle_state = $10 AND acl_projection_state = $11
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(group_name)
        .bind(group_to_i64("membership_epoch", membership_epoch)?)
        .bind(group_to_i64(
            "upstream_link_generation",
            upstream_link_generation,
        )?)
        .bind(source_event_id)
        .bind("im-membership-sync")
        .bind(operation_context.now)
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(GroupKnowledgeSpaceLifecycleState::Active.as_str())
        .bind(GroupKnowledgeSpaceAclProjectionState::Active.as_str())
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
            "group membership binding changed while its ACL projection was running".to_string(),
        ));
    }
    Ok(())
}

async fn update_binding_membership_projection_error(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
    error_code: &str,
    now: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let error_at_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$2");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_binding
        SET last_error_code = $1,
            last_error_at = {error_at_expr},
            updated_by = $3,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $4 AND organization_id = $5 AND id = $6
          AND lifecycle_state = $7 AND acl_projection_state = $8
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(truncate_error_code(error_code))
        .bind(now)
        .bind("group-membership-projection")
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(GroupKnowledgeSpaceLifecycleState::Active.as_str())
        .bind(GroupKnowledgeSpaceAclProjectionState::Active.as_str())
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
            "group membership binding changed while its ACL projection was running".to_string(),
        ));
    }
    Ok(())
}

async fn fetch_membership_projection_by_source(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    scope: GroupKnowledgeSpaceScope,
    source_event_id: &str,
) -> Result<Option<GroupMembershipProjection>, KnowledgeGroupSpaceBindingStoreError> {
    let row = sqlx::query(
        r#"
        SELECT id, binding_id, payload_sha256_hex, target_membership_epoch,
               projection_state, projection_lease_token
        FROM kb_group_knowledge_space_membership_projection
        WHERE tenant_id = $1 AND organization_id = $2 AND source_event_id = $3
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(source_event_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(group_sqlx_error)?;
    row.map(|row| membership_projection_from_row(&row))
        .transpose()
}

async fn fetch_unsettled_membership_projection(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
) -> Result<Option<GroupMembershipProjection>, KnowledgeGroupSpaceBindingStoreError> {
    let row = sqlx::query(
        r#"
        SELECT id, binding_id, payload_sha256_hex, target_membership_epoch,
               projection_state, projection_lease_token
        FROM kb_group_knowledge_space_membership_projection
        WHERE tenant_id = $1 AND organization_id = $2 AND binding_id = $3
          AND projection_state IN ($4, $5)
        ORDER BY id ASC
        LIMIT 1
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(group_to_i64("binding_id", binding_id)?)
    .bind(MEMBERSHIP_PROJECTION_PENDING)
    .bind(MEMBERSHIP_PROJECTION_FAILED)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(group_sqlx_error)?;
    row.map(|row| membership_projection_from_row(&row))
        .transpose()
}

async fn append_outbox_event(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    operation_context: &GroupKnowledgeSpaceStoreOperationContext<'_>,
    binding_id: u64,
    event_type: &str,
    payload_json: &str,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    let id = next_group_id(operation_context.id_generator)?;
    let payload_expr = operation_context.timestamp_dialect.sql_json_expr("$8");
    let created_at_expr = operation_context
        .timestamp_dialect
        .sql_timestamp_expr("$10");
    let query = format!(
        r#"
        INSERT INTO kb_outbox_event (
            id, uuid, tenant_id, organization_id, aggregate_type, aggregate_id, event_type, payload, status,
            created_at, version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, {payload_expr}, $9, {created_at_expr}, $11)
        "#,
    );
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(group_to_i64("outbox_id", id)?)
        .bind(Uuid::new_v4().to_string())
        .bind(group_to_i64(
            "tenant_id",
            operation_context.scope.tenant_id,
        )?)
        .bind(group_to_i64(
            "organization_id",
            operation_context.scope.organization_id,
        )?)
        .bind("group_knowledge_space_binding")
        .bind(group_to_i64("binding_id", binding_id)?)
        .bind(event_type)
        .bind(payload_json)
        .bind(INACTIVE_STATUS)
        .bind(operation_context.now)
        .bind(INITIAL_VERSION)
        .execute(&mut **transaction)
        .await
        .map_err(group_sqlx_error)?;
    Ok(())
}

async fn fetch_binding_by_conversation(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    scope: GroupKnowledgeSpaceScope,
    conversation_id: &str,
) -> Result<Option<GroupKnowledgeSpaceBinding>, KnowledgeGroupSpaceBindingStoreError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
               group_name, lifecycle_state, acl_projection_state,
               provisioning_idempotency_key_sha256_hex, membership_epoch,
               upstream_link_generation, version, archive_source_event_id,
               archive_payload_sha256_hex, archive_lease_token, archive_lease_until,
               archive_acl_cursor, archive_acl_pages_processed,
               archive_acl_cleanup_completed_at,
               last_source_event_id, last_error_code, created_by, updated_by,
               created_at, updated_at, archived_at, archived_by, deleted_at
        FROM kb_group_knowledge_space_binding
        WHERE tenant_id = $1 AND organization_id = $2 AND conversation_id = $3
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(conversation_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(group_sqlx_error)?;
    row.map(|row| group_binding_from_row(&row)).transpose()
}

async fn fetch_binding_by_provisioning_idempotency_key(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    scope: GroupKnowledgeSpaceScope,
    provisioning_idempotency_key_sha256_hex: &str,
) -> Result<Option<GroupKnowledgeSpaceBinding>, KnowledgeGroupSpaceBindingStoreError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
               group_name, lifecycle_state, acl_projection_state,
               provisioning_idempotency_key_sha256_hex, membership_epoch,
               upstream_link_generation, version, archive_source_event_id,
               archive_payload_sha256_hex, archive_lease_token, archive_lease_until,
               archive_acl_cursor, archive_acl_pages_processed,
               archive_acl_cleanup_completed_at,
               last_source_event_id, last_error_code, created_by, updated_by,
               created_at, updated_at, archived_at, archived_by, deleted_at
        FROM kb_group_knowledge_space_binding
        WHERE tenant_id = $1 AND organization_id = $2
          AND provisioning_idempotency_key_sha256_hex = $3
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(provisioning_idempotency_key_sha256_hex)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(group_sqlx_error)?;
    row.map(|row| group_binding_from_row(&row)).transpose()
}

async fn fetch_binding_by_id(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
    let row = sqlx::query(
        r#"
        SELECT id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
               group_name, lifecycle_state, acl_projection_state,
               provisioning_idempotency_key_sha256_hex, membership_epoch,
               upstream_link_generation, version, archive_source_event_id,
               archive_payload_sha256_hex, archive_lease_token, archive_lease_until,
               archive_acl_cursor, archive_acl_pages_processed,
               archive_acl_cleanup_completed_at,
               last_source_event_id, last_error_code, created_by, updated_by,
               created_at, updated_at, archived_at, archived_by, deleted_at
        FROM kb_group_knowledge_space_binding
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(group_to_i64("binding_id", binding_id)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(group_sqlx_error)?;
    row.map(|row| group_binding_from_row(&row))
        .transpose()?
        .ok_or_else(|| KnowledgeGroupSpaceBindingStoreError::NotFound(binding_id.to_string()))
}

async fn fetch_binding_space(
    transaction: &mut sqlx::Transaction<'_, sqlx::Any>,
    scope: GroupKnowledgeSpaceScope,
    binding: &GroupKnowledgeSpaceBinding,
) -> Result<Option<KnowledgeSpace>, KnowledgeGroupSpaceBindingStoreError> {
    let Some(space_id) = binding.space_id else {
        return Ok(None);
    };
    let row = sqlx::query(
        r#"
        SELECT id, uuid, name, description, drive_space_id, status, okf_bundle_initialized, knowledge_mode
        FROM kb_space
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(group_to_i64("space_id", space_id)?)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(group_sqlx_error)?;
    row.map(|row| group_space_from_row(&row)).transpose()
}

async fn list_active_members<'e, E>(
    executor: E,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
) -> Result<Vec<GroupKnowledgeSpaceMember>, KnowledgeGroupSpaceBindingStoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Any>,
{
    let rows = sqlx::query(
        r#"
        SELECT principal_kind, actor_id, member_role, access_level
        FROM kb_group_knowledge_space_member
        WHERE tenant_id = $1 AND organization_id = $2 AND binding_id = $3 AND status = $4
        ORDER BY actor_id ASC
        LIMIT $5
        "#,
    )
    .bind(group_to_i64("tenant_id", scope.tenant_id)?)
    .bind(group_to_i64("organization_id", scope.organization_id)?)
    .bind(group_to_i64("binding_id", binding_id)?)
    .bind(ACTIVE_STATUS)
    .bind(
        i64::try_from(GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS + 1).map_err(|_| {
            KnowledgeGroupSpaceBindingStoreError::Internal(
                "group membership snapshot limit exceeds the database boundary".to_string(),
            )
        })?,
    )
    .fetch_all(executor)
    .await
    .map_err(group_sqlx_error)?;
    if rows.len() > GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            format!(
                "stored group membership snapshot exceeds the {} member limit",
                GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS
            ),
        ));
    }
    rows.into_iter()
        .map(|row| group_member_from_row(&row))
        .collect()
}

async fn update_acl_projection_state(
    pool: &AnyPool,
    timestamp_dialect: &SqlTimestampDialect,
    scope: GroupKnowledgeSpaceScope,
    binding_id: u64,
    state: GroupKnowledgeSpaceAclProjectionState,
    error_code: Option<&str>,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    validate_scope(scope)?;
    let now = group_now()?;
    let updated_at_expr = timestamp_dialect.sql_timestamp_expr("$1");
    let error_at_expr = timestamp_dialect.sql_timestamp_expr("$1");
    let query = format!(
        r#"
        UPDATE kb_group_knowledge_space_binding
        SET acl_projection_state = $2,
            last_error_code = $3,
            last_error_at = CASE WHEN $3 IS NULL THEN NULL ELSE {error_at_expr} END,
            updated_by = $4,
            updated_at = {updated_at_expr},
            version = version + 1
        WHERE tenant_id = $5 AND organization_id = $6 AND id = $7
          AND (lifecycle_state <> 'active' OR $2 = 'active')
        "#,
    );
    let updated = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(&now)
        .bind(state.as_str())
        .bind(error_code.map(truncate_error_code))
        .bind("group-acl-projection")
        .bind(group_to_i64("tenant_id", scope.tenant_id)?)
        .bind(group_to_i64("organization_id", scope.organization_id)?)
        .bind(group_to_i64("binding_id", binding_id)?)
        .execute(pool)
        .await
        .map_err(group_sqlx_error)?;
    if updated.rows_affected() != 1 {
        return Err(KnowledgeGroupSpaceBindingStoreError::NotFound(
            binding_id.to_string(),
        ));
    }
    Ok(())
}

fn group_binding_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<GroupKnowledgeSpaceBinding, KnowledgeGroupSpaceBindingStoreError> {
    let lifecycle_state: String = row.try_get("lifecycle_state").map_err(group_sqlx_error)?;
    let acl_projection_state: String = row
        .try_get("acl_projection_state")
        .map_err(group_sqlx_error)?;
    Ok(GroupKnowledgeSpaceBinding {
        id: group_from_i64("id", row.try_get("id").map_err(group_sqlx_error)?)?,
        uuid: row.try_get("uuid").map_err(group_sqlx_error)?,
        tenant_id: group_from_i64(
            "tenant_id",
            row.try_get("tenant_id").map_err(group_sqlx_error)?,
        )?,
        organization_id: group_from_i64(
            "organization_id",
            row.try_get("organization_id").map_err(group_sqlx_error)?,
        )?,
        conversation_id: row.try_get("conversation_id").map_err(group_sqlx_error)?,
        space_id: row
            .try_get::<Option<i64>, _>("space_id")
            .map_err(group_sqlx_error)?
            .map(|value| group_from_i64("space_id", value))
            .transpose()?,
        space_uuid: row.try_get("space_uuid").map_err(group_sqlx_error)?,
        group_name: row.try_get("group_name").map_err(group_sqlx_error)?,
        lifecycle_state: lifecycle_state.parse().map_err(|()| {
            KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                "unsupported group knowledge space lifecycle: {lifecycle_state}"
            ))
        })?,
        acl_projection_state: acl_projection_state.parse().map_err(|()| {
            KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                "unsupported group knowledge space ACL state: {acl_projection_state}"
            ))
        })?,
        provisioning_idempotency_key_sha256_hex: row
            .try_get("provisioning_idempotency_key_sha256_hex")
            .map_err(group_sqlx_error)?,
        membership_epoch: group_from_i64(
            "membership_epoch",
            row.try_get("membership_epoch").map_err(group_sqlx_error)?,
        )?,
        upstream_link_generation: group_from_i64(
            "upstream_link_generation",
            row.try_get("upstream_link_generation")
                .map_err(group_sqlx_error)?,
        )?,
        version: group_from_i64("version", row.try_get("version").map_err(group_sqlx_error)?)?,
        archive_source_event_id: row
            .try_get("archive_source_event_id")
            .map_err(group_sqlx_error)?,
        archive_payload_sha256_hex: row
            .try_get("archive_payload_sha256_hex")
            .map_err(group_sqlx_error)?,
        archive_lease_token: row
            .try_get("archive_lease_token")
            .map_err(group_sqlx_error)?,
        archive_lease_until: row
            .try_get("archive_lease_until")
            .map_err(group_sqlx_error)?,
        archive_acl_cursor: row
            .try_get("archive_acl_cursor")
            .map_err(group_sqlx_error)?,
        archive_acl_pages_processed: group_from_i64(
            "archive_acl_pages_processed",
            row.try_get("archive_acl_pages_processed")
                .map_err(group_sqlx_error)?,
        )?,
        archive_acl_cleanup_completed_at: row
            .try_get("archive_acl_cleanup_completed_at")
            .map_err(group_sqlx_error)?,
        last_source_event_id: row
            .try_get("last_source_event_id")
            .map_err(group_sqlx_error)?,
        last_error_code: row.try_get("last_error_code").map_err(group_sqlx_error)?,
        created_by: row.try_get("created_by").map_err(group_sqlx_error)?,
        updated_by: row.try_get("updated_by").map_err(group_sqlx_error)?,
        created_at: row.try_get("created_at").map_err(group_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(group_sqlx_error)?,
        archived_at: row.try_get("archived_at").map_err(group_sqlx_error)?,
        archived_by: row.try_get("archived_by").map_err(group_sqlx_error)?,
        deleted_at: row.try_get("deleted_at").map_err(group_sqlx_error)?,
    })
}

fn membership_projection_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<GroupMembershipProjection, KnowledgeGroupSpaceBindingStoreError> {
    Ok(GroupMembershipProjection {
        id: group_from_i64("id", row.try_get("id").map_err(group_sqlx_error)?)?,
        binding_id: group_from_i64(
            "binding_id",
            row.try_get("binding_id").map_err(group_sqlx_error)?,
        )?,
        payload_sha256_hex: row
            .try_get("payload_sha256_hex")
            .map_err(group_sqlx_error)?,
        target_membership_epoch: group_from_i64(
            "target_membership_epoch",
            row.try_get("target_membership_epoch")
                .map_err(group_sqlx_error)?,
        )?,
        projection_state: row.try_get("projection_state").map_err(group_sqlx_error)?,
        projection_lease_token: row
            .try_get("projection_lease_token")
            .map_err(group_sqlx_error)?,
    })
}

fn group_member_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<GroupKnowledgeSpaceMember, KnowledgeGroupSpaceBindingStoreError> {
    let principal_kind: String = row.try_get("principal_kind").map_err(group_sqlx_error)?;
    let role: String = row.try_get("member_role").map_err(group_sqlx_error)?;
    let access_level: Option<String> = row.try_get("access_level").map_err(group_sqlx_error)?;
    Ok(GroupKnowledgeSpaceMember {
        principal_kind: principal_kind.parse().map_err(|()| {
            KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                "unsupported group member principal kind: {principal_kind}"
            ))
        })?,
        actor_id: row.try_get("actor_id").map_err(group_sqlx_error)?,
        role: role.parse().map_err(|()| {
            KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                "unsupported group member role: {role}"
            ))
        })?,
        access_level: access_level
            .map(|value| {
                value.parse().map_err(|()| {
                    KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                        "unsupported group member access level: {value}"
                    ))
                })
            })
            .transpose()?,
    })
}

fn group_space_from_row(
    row: &sqlx::any::AnyRow,
) -> Result<KnowledgeSpace, KnowledgeGroupSpaceBindingStoreError> {
    let status: i64 = row.try_get("status").map_err(group_sqlx_error)?;
    let knowledge_mode: Option<String> = row.try_get("knowledge_mode").map_err(group_sqlx_error)?;
    let knowledge_mode = match knowledge_mode.as_deref().unwrap_or("okf_bundle") {
        "okf_bundle" => KnowledgeAgentKnowledgeMode::OkfBundle,
        "rag" => KnowledgeAgentKnowledgeMode::Rag,
        "external" => KnowledgeAgentKnowledgeMode::External,
        value => {
            return Err(KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                "unsupported knowledge space mode: {value}"
            )))
        }
    };
    let status = match status {
        SPACE_PROVISIONING_STATUS => KnowledgeSpaceStatus::Provisioning,
        SPACE_ACTIVE_STATUS => KnowledgeSpaceStatus::Active,
        SPACE_ARCHIVED_STATUS => KnowledgeSpaceStatus::Archived,
        SPACE_DELETED_STATUS => KnowledgeSpaceStatus::Deleted,
        value => {
            return Err(KnowledgeGroupSpaceBindingStoreError::Internal(format!(
                "unsupported knowledge space status: {value}"
            )))
        }
    };
    Ok(KnowledgeSpace {
        id: group_from_i64("id", row.try_get("id").map_err(group_sqlx_error)?)?,
        uuid: row.try_get("uuid").map_err(group_sqlx_error)?,
        name: row.try_get("name").map_err(group_sqlx_error)?,
        description: row.try_get("description").map_err(group_sqlx_error)?,
        drive_space_id: row.try_get("drive_space_id").map_err(group_sqlx_error)?,
        status,
        okf_bundle_initialized: row
            .try_get::<i64, _>("okf_bundle_initialized")
            .map_err(group_sqlx_error)?
            != 0,
        knowledge_mode,
    })
}

fn validate_reservation_request(
    request: &ReserveGroupKnowledgeSpaceRequest,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    validate_scope(request.scope)?;
    validate_group_text(
        "conversation_id",
        &request.conversation_id,
        GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
    )?;
    validate_group_text(
        "group_name",
        &request.group_name,
        GROUP_KNOWLEDGE_SPACE_GROUP_NAME_MAX_LENGTH,
    )?;
    validate_group_text(
        "source_event_id",
        &request.source_event_id,
        GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
    )?;
    validate_group_text(
        "provisioning_idempotency_key",
        &request.provisioning_idempotency_key,
        512,
    )?;
    validate_group_text(
        "created_by",
        &request.created_by,
        GROUP_KNOWLEDGE_SPACE_ACTOR_ID_MAX_LENGTH,
    )?;
    validate_members(&request.members)
}

fn validate_group_target(
    target: &GroupKnowledgeSpaceTarget,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    if target.knowledgebase_binding_id == 0 || target.knowledge_space_id == 0 {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "knowledgebase binding id and knowledge space id are required".to_string(),
        ));
    }
    if target.knowledgebase_binding_id > MAX_GROUP_SCOPE_ID
        || target.knowledge_space_id > MAX_GROUP_SCOPE_ID
    {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "knowledgebase binding id and knowledge space id must fit signed BIGINT".to_string(),
        ));
    }
    validate_group_text(
        "knowledgebase_binding_uuid",
        &target.knowledgebase_binding_uuid,
        GROUP_KNOWLEDGE_SPACE_BINDING_UUID_MAX_LENGTH,
    )?;
    validate_group_text(
        "knowledge_space_uuid",
        &target.knowledge_space_uuid,
        GROUP_KNOWLEDGE_SPACE_SPACE_UUID_MAX_LENGTH,
    )
}

fn validate_membership_command(
    command: &SynchronizeGroupKnowledgeSpaceMembersCommand,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    validate_members(&command.members)?;
    validate_scope(command.scope)?;
    validate_group_text(
        "conversation_id",
        &command.conversation_id,
        GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
    )?;
    validate_group_text(
        "group_name",
        &command.group_name,
        GROUP_KNOWLEDGE_SPACE_GROUP_NAME_MAX_LENGTH,
    )?;
    validate_group_text(
        "source_event_id",
        &command.source_event_id,
        GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
    )?;
    validate_group_target(&command.target)
}

fn validate_archive_command(
    command: &ArchiveGroupKnowledgeSpaceCommand,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    validate_scope(command.scope)?;
    validate_group_text(
        "conversation_id",
        &command.conversation_id,
        GROUP_KNOWLEDGE_SPACE_CONVERSATION_ID_MAX_LENGTH,
    )?;
    validate_group_text(
        "source_event_id",
        &command.source_event_id,
        GROUP_KNOWLEDGE_SPACE_SOURCE_EVENT_ID_MAX_LENGTH,
    )?;
    validate_group_text(
        "archived_by",
        &command.archived_by,
        GROUP_KNOWLEDGE_SPACE_ACTOR_ID_MAX_LENGTH,
    )?;
    validate_group_target(&command.target)
}

fn ensure_target_matches_binding(
    binding: &GroupKnowledgeSpaceBinding,
    target: &GroupKnowledgeSpaceTarget,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    if binding.id != target.knowledgebase_binding_id
        || binding.uuid != target.knowledgebase_binding_uuid
        || binding.space_id != Some(target.knowledge_space_id)
        || binding.space_uuid.as_deref() != Some(target.knowledge_space_uuid.as_str())
    {
        return Err(KnowledgeGroupSpaceBindingStoreError::Conflict(
            "group knowledge space target does not match the immutable binding fence".to_string(),
        ));
    }
    Ok(())
}

fn archive_command_from_binding(
    binding: GroupKnowledgeSpaceBinding,
) -> Result<ArchiveGroupKnowledgeSpaceCommand, KnowledgeGroupSpaceBindingStoreError> {
    let scope = GroupKnowledgeSpaceScope {
        tenant_id: binding.tenant_id,
        organization_id: binding.organization_id,
    };
    let command = ArchiveGroupKnowledgeSpaceCommand {
        scope,
        conversation_id: binding.conversation_id,
        source_event_id: binding.archive_source_event_id.ok_or_else(|| {
            KnowledgeGroupSpaceBindingStoreError::Internal(
                "archiving group binding has no source event id".to_string(),
            )
        })?,
        target: GroupKnowledgeSpaceTarget {
            knowledgebase_binding_id: binding.id,
            knowledgebase_binding_uuid: binding.uuid,
            knowledge_space_id: binding.space_id.ok_or_else(|| {
                KnowledgeGroupSpaceBindingStoreError::Internal(
                    "archiving group binding has no knowledge space id".to_string(),
                )
            })?,
            knowledge_space_uuid: binding.space_uuid.ok_or_else(|| {
                KnowledgeGroupSpaceBindingStoreError::Internal(
                    "archiving group binding has no knowledge space UUID".to_string(),
                )
            })?,
        },
        membership_epoch: binding.membership_epoch,
        upstream_link_generation: binding.upstream_link_generation,
        archived_by: binding.archived_by.ok_or_else(|| {
            KnowledgeGroupSpaceBindingStoreError::Internal(
                "archiving group binding has no audit actor".to_string(),
            )
        })?,
    };
    validate_archive_command(&command)?;
    Ok(command)
}

fn validate_scope(
    scope: GroupKnowledgeSpaceScope,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    validate_tenant_id(scope.tenant_id)?;
    if scope.organization_id > MAX_GROUP_SCOPE_ID {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "organization_id exceeds the signed BIGINT group knowledge space boundary".to_string(),
        ));
    }
    Ok(())
}

fn validate_tenant_id(tenant_id: u64) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    if tenant_id == 0 {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "tenant_id is required".to_string(),
        ));
    }
    if tenant_id > MAX_GROUP_SCOPE_ID {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "tenant_id exceeds the signed BIGINT group knowledge space boundary".to_string(),
        ));
    }
    Ok(())
}

fn validate_group_text(
    field: &str,
    value: &str,
    maximum_length: usize,
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    if is_blank(Some(value)) || value.len() > maximum_length {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            format!("{field} is required and must not exceed {maximum_length} characters"),
        ));
    }
    Ok(())
}

fn validate_members(
    members: &[GroupKnowledgeSpaceMember],
) -> Result<(), KnowledgeGroupSpaceBindingStoreError> {
    if members.is_empty() {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "at least one group member is required".to_string(),
        ));
    }
    if members.len() > GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            format!(
                "group membership snapshot must contain no more than {} members",
                GROUP_KNOWLEDGE_SPACE_MEMBER_SNAPSHOT_MAX_ITEMS
            ),
        ));
    }
    let mut owner_count = 0usize;
    let mut actors = std::collections::BTreeSet::new();
    for member in members {
        if member.principal_kind != GroupKnowledgeSpacePrincipalKind::User {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "group knowledge space members must use user principals".to_string(),
            ));
        }
        validate_group_text(
            "member.actor_id",
            &member.actor_id,
            GROUP_KNOWLEDGE_SPACE_ACTOR_ID_MAX_LENGTH,
        )?;
        if !actors.insert(member.actor_id.as_str()) {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "member actor ids must be unique".to_string(),
            ));
        }
        if member.role == GroupKnowledgeSpaceMemberRole::Owner {
            owner_count += 1;
        }
        if member
            .access_level
            .is_some_and(|access_level| Some(access_level) != member.role.access_level())
        {
            return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
                "member access level does not match the IM group role policy".to_string(),
            ));
        }
    }
    if owner_count != 1 {
        return Err(KnowledgeGroupSpaceBindingStoreError::InvalidRequest(
            "exactly one group owner is required".to_string(),
        ));
    }
    Ok(())
}

fn reservation_fingerprint(request: &ReserveGroupKnowledgeSpaceRequest) -> String {
    let mut values = vec![
        "ensure".to_string(),
        request.conversation_id.clone(),
        request.group_name.clone(),
        request.membership_epoch.to_string(),
        sha256_hash(request.provisioning_idempotency_key.as_bytes()),
    ];
    values.extend(normalized_members(&request.members));
    sha256_hash(values.join("\u{0}").as_bytes())
}

fn membership_command_fingerprint(
    command: &SynchronizeGroupKnowledgeSpaceMembersCommand,
) -> String {
    let mut values = vec![
        "membership_sync".to_string(),
        command.conversation_id.clone(),
        command.group_name.clone(),
        command.target.knowledgebase_binding_id.to_string(),
        command.target.knowledgebase_binding_uuid.clone(),
        command.target.knowledge_space_id.to_string(),
        command.target.knowledge_space_uuid.clone(),
        command.membership_epoch.to_string(),
        command.upstream_link_generation.to_string(),
    ];
    values.extend(normalized_members(&command.members));
    sha256_hash(values.join("\u{0}").as_bytes())
}

fn archive_command_fingerprint(command: &ArchiveGroupKnowledgeSpaceCommand) -> String {
    let values = [
        "archive".to_string(),
        command.conversation_id.clone(),
        command.target.knowledgebase_binding_id.to_string(),
        command.target.knowledgebase_binding_uuid.clone(),
        command.target.knowledge_space_id.to_string(),
        command.target.knowledge_space_uuid.clone(),
        command.membership_epoch.to_string(),
        command.upstream_link_generation.to_string(),
        command.archived_by.clone(),
    ];
    sha256_hash(values.join("\u{0}").as_bytes())
}

fn normalized_members(members: &[GroupKnowledgeSpaceMember]) -> Vec<String> {
    let mut values: Vec<String> = members
        .iter()
        .map(|member| {
            format!(
                "{}:{}:{}:{}",
                member.principal_kind.as_str(),
                member.actor_id,
                member.role.as_str(),
                member
                    .role
                    .access_level()
                    .map(|value| value.as_str())
                    .unwrap_or("none")
            )
        })
        .collect();
    values.sort();
    values
}

fn group_event_payload(binding: &GroupKnowledgeSpaceBinding) -> String {
    serde_json::json!({
        "bindingId": binding.id.to_string(),
        "conversationId": binding.conversation_id,
        "spaceId": binding.space_id.map(|id| id.to_string()),
        "lifecycleState": binding.lifecycle_state.as_str(),
        "membershipEpoch": binding.membership_epoch.to_string(),
        "version": binding.version.to_string(),
    })
    .to_string()
}

fn truncate_error_code(error_code: &str) -> String {
    error_code.chars().take(64).collect()
}

fn group_now() -> Result<String, KnowledgeGroupSpaceBindingStoreError> {
    utc_sql_timestamp_text().map_err(KnowledgeGroupSpaceBindingStoreError::Internal)
}

fn group_lease_until() -> Result<String, KnowledgeGroupSpaceBindingStoreError> {
    (OffsetDateTime::now_utc() + Duration::seconds(PROVISIONING_LEASE_SECONDS))
        .format(&Rfc3339)
        .map_err(|error| KnowledgeGroupSpaceBindingStoreError::Internal(error.to_string()))
}

fn next_group_id(
    id_generator: &Arc<dyn KnowledgeIdGenerator>,
) -> Result<u64, KnowledgeGroupSpaceBindingStoreError> {
    let id = next_i64_id(id_generator)
        .map_err(|error| KnowledgeGroupSpaceBindingStoreError::Internal(error.to_string()))?;
    group_from_i64("id", id)
}

fn group_to_i64(field: &str, value: u64) -> Result<i64, KnowledgeGroupSpaceBindingStoreError> {
    i64::try_from(value).map_err(|_| {
        KnowledgeGroupSpaceBindingStoreError::InvalidRequest(format!("{field} is out of range"))
    })
}

fn group_from_i64(field: &str, value: i64) -> Result<u64, KnowledgeGroupSpaceBindingStoreError> {
    u64::try_from(value).map_err(|_| {
        KnowledgeGroupSpaceBindingStoreError::Internal(format!("{field} must not be negative"))
    })
}

fn group_sqlx_error(error: sqlx::Error) -> KnowledgeGroupSpaceBindingStoreError {
    let message = error.to_string();
    if message.contains("UNIQUE") || message.contains("unique") {
        KnowledgeGroupSpaceBindingStoreError::Conflict(message)
    } else {
        KnowledgeGroupSpaceBindingStoreError::Internal(message)
    }
}
