mod support;

use std::time::Duration;

use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    PostgresKnowledgeSpaceStore, SqlxKnowledgeEngineProviderBindingStore,
    SqlxKnowledgeEngineProviderMigrationStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::{
    knowledge_provider_binding_store::{
        KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderScope,
        RecordKnowledgeEngineProviderTestResult,
    },
    knowledge_provider_migration_store::{
        AdvanceClaimedKnowledgeEngineProviderMigration,
        CutoverClaimedKnowledgeEngineProviderMigration, KnowledgeEngineProviderMigrationStore,
        KnowledgeEngineProviderMigrationStoreError,
    },
    knowledge_space_store::{CreateKnowledgeSpaceRecord, KnowledgeSpaceStore},
};
use sdkwork_knowledgebase_contract::{
    knowledge_engine::KnowledgeEngineCapability,
    provider_binding::{
        CreateKnowledgeEngineProviderBindingRequest,
        CreateKnowledgeEngineProviderMigrationOperationRequest, KnowledgeEngineProviderBinding,
        KnowledgeEngineProviderBindingState, KnowledgeEngineProviderMigrationState,
        ListKnowledgeEngineProviderMigrationOperationsRequest,
    },
    rag::KnowledgeAgentKnowledgeMode,
};
use serde_json::json;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

async fn clean_migration_scope(pool: &sqlx::AnyPool, tenant_id: u64, organization_id: u64) {
    for statement in [
        "DELETE FROM kb_provider_migration_operation WHERE tenant_id = $1 AND organization_id = $2",
        "DELETE FROM kb_provider_binding WHERE tenant_id = $1 AND organization_id = $2",
        "DELETE FROM kb_provider_credential_reference WHERE tenant_id = $1 AND organization_id = $2",
        "DELETE FROM kb_source WHERE tenant_id = $1 AND organization_id = $2",
        "DELETE FROM kb_space WHERE tenant_id = $1 AND organization_id = $2",
    ] {
        sqlx::query(statement)
            .bind(tenant_id as i64)
            .bind(organization_id as i64)
            .execute(pool)
            .await
            .expect("clean provider migration fixtures");
    }
}

#[tokio::test]
async fn provider_migration_is_idempotent_claimed_versioned_and_reversible() {
    let Some(pool) = migration_test_pool(110_001, 71).await else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    clean_migration_scope(&pool, 110_001, 71).await;
    let scope = KnowledgeEngineProviderScope {
        tenant_id: 110_001,
        organization_id: 71,
    };
    let space_store =
        PostgresKnowledgeSpaceStore::new(pool.clone(), scope.tenant_id, scope.organization_id);
    let space = space_store
        .create_space(CreateKnowledgeSpaceRecord {
            name: "Provider migration space".to_string(),
            description: None,
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::External,
        })
        .await
        .expect("create space");
    let binding_store = SqlxKnowledgeEngineProviderBindingStore::new(pool.clone());
    let source = create_tested_binding(
        &binding_store,
        scope,
        space.id,
        "engine.knowledge.external.dify",
        "source-dataset",
    )
    .await;
    let source = binding_store
        .activate_binding(scope, source.id, "tenant-admin", source.version)
        .await
        .expect("activate source");
    let target = create_tested_binding(
        &binding_store,
        scope,
        space.id,
        "engine.knowledge.external.ragflow",
        "target-dataset",
    )
    .await;
    let migration_store = SqlxKnowledgeEngineProviderMigrationStore::new(pool.clone());
    let request = CreateKnowledgeEngineProviderMigrationOperationRequest {
        source_binding_id: source.id,
        target_binding_id: target.id,
        idempotency_key: "provider-migration-001".to_string(),
        expected_source_version: source.version,
        expected_target_version: target.version,
        observation_seconds: 60,
    };

    let operation = migration_store
        .create_operation(scope, space.id, "tenant-admin", request.clone())
        .await
        .expect("create migration");
    let replay = migration_store
        .create_operation(scope, space.id, "tenant-admin", request)
        .await
        .expect("idempotent replay");
    assert_eq!(operation.id, replay.id);

    let page = migration_store
        .list_operations(
            scope,
            ListKnowledgeEngineProviderMigrationOperationsRequest {
                space_id: space.id,
                operation_state: Some(KnowledgeEngineProviderMigrationState::DryRun),
                cursor: None,
                page_size: Some(1),
            },
        )
        .await
        .expect("list migration");
    assert_eq!(page.items.len(), 1);

    let mut claimed = migration_store
        .claim_next(scope, "worker-a", Duration::from_secs(30))
        .await
        .expect("claim dry run")
        .expect("claimable operation");
    assert!(migration_store
        .claim_next(scope, "worker-b", Duration::from_secs(30))
        .await
        .expect("second claim")
        .is_none());

    for (from, to) in [
        (
            KnowledgeEngineProviderMigrationState::DryRun,
            KnowledgeEngineProviderMigrationState::Preparing,
        ),
        (
            KnowledgeEngineProviderMigrationState::Preparing,
            KnowledgeEngineProviderMigrationState::Validating,
        ),
        (
            KnowledgeEngineProviderMigrationState::Validating,
            KnowledgeEngineProviderMigrationState::Cutover,
        ),
    ] {
        let checkpoint = claimed.checkpoint.clone();
        migration_store
            .advance_claimed(
                scope,
                operation.id,
                &claimed.claim_token,
                claimed.operation.version,
                AdvanceClaimedKnowledgeEngineProviderMigration {
                    expected_state: from,
                    next_state: to,
                    checkpoint,
                    observation_until: None,
                    error_category: None,
                },
            )
            .await
            .expect("advance migration");
        claimed = migration_store
            .claim_next(scope, "worker-a", Duration::from_secs(30))
            .await
            .expect("claim next phase")
            .expect("next phase claim");
    }

    let observation_until = (OffsetDateTime::now_utc() + time::Duration::seconds(60))
        .format(&Rfc3339)
        .expect("observation timestamp");
    let cutover = migration_store
        .cutover_claimed(
            scope,
            CutoverClaimedKnowledgeEngineProviderMigration {
                operation_id: operation.id,
                claim_token: claimed.claim_token,
                expected_version: claimed.operation.version,
                actor_id: "tenant-admin".to_string(),
                observation_until,
                checkpoint: claimed.checkpoint,
            },
        )
        .await
        .expect("atomic cutover");
    assert_eq!(
        cutover.operation_state,
        KnowledgeEngineProviderMigrationState::Observing
    );
    assert_eq!(
        binding_store
            .get_active_binding_for_space(scope, space.id)
            .await
            .expect("active binding")
            .expect("target active")
            .id,
        target.id
    );
    assert_eq!(
        binding_store
            .get_binding(scope, source.id)
            .await
            .expect("source retained")
            .lifecycle_state,
        KnowledgeEngineProviderBindingState::Disabled
    );
    assert!(migration_store
        .claim_next(scope, "worker-observer", Duration::from_secs(30))
        .await
        .expect("check observation claim")
        .is_none());

    let rolling_back = migration_store
        .request_rollback(scope, operation.id, "tenant-admin", cutover.version)
        .await
        .expect("request rollback");
    assert_eq!(
        rolling_back.operation_state,
        KnowledgeEngineProviderMigrationState::RollingBack
    );
    let rollback_claim = migration_store
        .claim_next(scope, "worker-b", Duration::from_secs(30))
        .await
        .expect("claim rollback")
        .expect("rollback claim");
    let rolled_back = migration_store
        .rollback_claimed(
            scope,
            operation.id,
            &rollback_claim.claim_token,
            rollback_claim.operation.version,
            "tenant-admin",
            rollback_claim.checkpoint,
        )
        .await
        .expect("atomic rollback");
    assert_eq!(
        rolled_back.operation_state,
        KnowledgeEngineProviderMigrationState::RolledBack
    );
    assert_eq!(
        binding_store
            .get_active_binding_for_space(scope, space.id)
            .await
            .expect("active binding")
            .expect("source restored")
            .id,
        source.id
    );
}

#[tokio::test]
async fn provider_migration_rejects_stale_claim_token_after_lease_recovery() {
    let Some((pool, scope, space_id, source, target)) = migration_fixture("stale-claim").await
    else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    let store = SqlxKnowledgeEngineProviderMigrationStore::new(pool.clone());
    let operation = store
        .create_operation(
            scope,
            space_id,
            "tenant-admin",
            CreateKnowledgeEngineProviderMigrationOperationRequest {
                source_binding_id: source.id,
                target_binding_id: target.id,
                idempotency_key: "stale-claim-operation".to_string(),
                expected_source_version: source.version,
                expected_target_version: target.version,
                observation_seconds: 60,
            },
        )
        .await
        .expect("create migration");
    let stale = store
        .claim_next(scope, "worker-stale", Duration::from_secs(30))
        .await
        .expect("claim")
        .expect("claimed");
    sqlx::query("UPDATE kb_provider_migration_operation SET lease_expires_at = '2000-01-01T00:00:00Z' WHERE id = $1")
        .bind(i64::try_from(operation.id).expect("operation id"))
        .execute(&pool)
        .await
        .expect("expire lease");
    let recovered = store
        .claim_next(scope, "worker-recovery", Duration::from_secs(30))
        .await
        .expect("recover")
        .expect("recovered claim");
    assert_ne!(stale.claim_token, recovered.claim_token);

    let error = store
        .advance_claimed(
            scope,
            operation.id,
            &stale.claim_token,
            stale.operation.version,
            AdvanceClaimedKnowledgeEngineProviderMigration {
                expected_state: KnowledgeEngineProviderMigrationState::DryRun,
                next_state: KnowledgeEngineProviderMigrationState::Preparing,
                checkpoint: json!({}),
                observation_until: None,
                error_category: None,
            },
        )
        .await
        .expect_err("stale token must fail");
    assert_eq!(
        error,
        KnowledgeEngineProviderMigrationStoreError::ClaimLost(operation.id)
    );

    let failed = store
        .advance_claimed(
            scope,
            operation.id,
            &recovered.claim_token,
            recovered.operation.version,
            AdvanceClaimedKnowledgeEngineProviderMigration {
                expected_state: KnowledgeEngineProviderMigrationState::DryRun,
                next_state: KnowledgeEngineProviderMigrationState::Failed,
                checkpoint: recovered.checkpoint,
                observation_until: None,
                error_category: Some(
                    sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory::Validation,
                ),
            },
        )
        .await
        .expect("fail recovered migration");
    assert!(failed.completed_at.is_some());
    let rolling_back = store
        .request_rollback(scope, operation.id, "tenant-admin", failed.version)
        .await
        .expect("request rollback after failure");
    assert!(rolling_back.completed_at.is_none());
    let rollback_claim = store
        .claim_next(scope, "worker-recovery", Duration::from_secs(30))
        .await
        .expect("claim failed migration rollback")
        .expect("rollback must be claimable");
    let rolled_back = store
        .rollback_claimed(
            scope,
            operation.id,
            &rollback_claim.claim_token,
            rollback_claim.operation.version,
            "tenant-admin",
            rollback_claim.checkpoint,
        )
        .await
        .expect("complete pre-cutover rollback");
    assert_eq!(
        rolled_back.operation_state,
        KnowledgeEngineProviderMigrationState::RolledBack
    );
}

async fn migration_fixture(
    _name: &str,
) -> Option<(
    sqlx::AnyPool,
    KnowledgeEngineProviderScope,
    u64,
    KnowledgeEngineProviderBinding,
    KnowledgeEngineProviderBinding,
)> {
    let pool = migration_test_pool(120_001, 72).await?;
    let scope = KnowledgeEngineProviderScope {
        tenant_id: 120_001,
        organization_id: 72,
    };
    clean_migration_scope(&pool, 120_001, 72).await;
    let space_store =
        PostgresKnowledgeSpaceStore::new(pool.clone(), scope.tenant_id, scope.organization_id);
    let space = space_store
        .create_space(CreateKnowledgeSpaceRecord {
            name: "Migration fixture".to_string(),
            description: None,
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::External,
        })
        .await
        .expect("create space");
    let bindings = SqlxKnowledgeEngineProviderBindingStore::new(pool.clone());
    let source = create_tested_binding(
        &bindings,
        scope,
        space.id,
        "engine.knowledge.external.dify",
        "source",
    )
    .await;
    let source = bindings
        .activate_binding(scope, source.id, "tenant-admin", source.version)
        .await
        .expect("activate source");
    let target = create_tested_binding(
        &bindings,
        scope,
        space.id,
        "engine.knowledge.external.ragflow",
        "target",
    )
    .await;
    Some((pool, scope, space.id, source, target))
}

async fn create_tested_binding(
    store: &SqlxKnowledgeEngineProviderBindingStore,
    scope: KnowledgeEngineProviderScope,
    space_id: u64,
    implementation_id: &str,
    remote_resource_id: &str,
) -> KnowledgeEngineProviderBinding {
    let binding = store
        .create_binding(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderBindingRequest {
                space_id,
                implementation_id: implementation_id.to_string(),
                remote_resource_type: "dataset".to_string(),
                remote_resource_id: remote_resource_id.to_string(),
                credential_reference_id: None,
            },
        )
        .await
        .expect("create binding");
    let testing = store
        .begin_binding_test(scope, binding.id, "tenant-admin", binding.version)
        .await
        .expect("begin test");
    store
        .record_binding_test_result(
            scope,
            binding.id,
            RecordKnowledgeEngineProviderTestResult {
                expected_version: testing.version,
                capabilities: vec![
                    KnowledgeEngineCapability::Health,
                    KnowledgeEngineCapability::Search,
                    KnowledgeEngineCapability::ReadDocument,
                ],
                error_category: None,
                updated_by: "tenant-admin".to_string(),
            },
        )
        .await
        .expect("record successful test")
}

async fn migration_test_pool(tenant_id: u64, organization_id: u64) -> Option<sqlx::AnyPool> {
    support::scoped_pool(tenant_id, organization_id).await
}
