mod support;

use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    PostgresKnowledgeSpaceStore, SqlxKnowledgeEngineProviderBindingStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderBindingStoreError,
    KnowledgeEngineProviderScope, RecordKnowledgeEngineProviderTestResult,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
};
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineCapability;
use sdkwork_knowledgebase_contract::provider_binding::{
    CreateKnowledgeEngineProviderBindingRequest,
    CreateKnowledgeEngineProviderCredentialReferenceRequest, KnowledgeEngineProviderBindingState,
    KnowledgeEngineProviderCredentialRotationState, ListKnowledgeEngineProviderBindingsRequest,
    ListKnowledgeEngineProviderCredentialReferencesRequest,
    RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    RotateKnowledgeEngineProviderCredentialReferenceRequest,
    UpdateKnowledgeEngineProviderBindingRequest,
};

#[tokio::test]
async fn provider_binding_lifecycle_is_scoped_versioned_and_atomically_switchable() {
    let Some(pool) = provider_test_pool(100_001, 7_001).await else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    let scope = KnowledgeEngineProviderScope {
        tenant_id: 100_001,
        organization_id: 7_001,
    };
    clean_provider_scope(&pool, scope.tenant_id, scope.organization_id).await;
    let space_store =
        PostgresKnowledgeSpaceStore::new(pool.clone(), scope.tenant_id, scope.organization_id);
    let space = space_store
        .create_space(CreateKnowledgeSpaceRecord {
            name: "Provider Binding Space".to_string(),
            description: None,
            okf_bundle_initialized: false,
            knowledge_mode:
                sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode::External,
        })
        .await
        .expect("create external space");
    let store = SqlxKnowledgeEngineProviderBindingStore::new(pool);

    let credential = store
        .create_credential_reference(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderCredentialReferenceRequest {
                implementation_id: "engine.knowledge.external.dify".to_string(),
                display_name: "Dify production credential".to_string(),
                reference_locator: "secret://knowledgebase/dify/primary".to_string(),
            },
        )
        .await
        .expect("create credential reference");
    let credential_json = serde_json::to_string(&credential).expect("serialize credential view");
    assert!(!credential_json.contains("secret://"));
    assert!(!credential_json.contains("referenceLocator"));

    let first = create_tested_binding(
        &store,
        scope,
        space.id,
        "engine.knowledge.external.dify",
        "dataset",
        "dataset-a",
        Some(credential.id),
    )
    .await;
    let first = store
        .activate_binding(scope, first.id, "tenant-admin", first.version)
        .await
        .expect("activate first binding");
    assert_eq!(
        first.lifecycle_state,
        KnowledgeEngineProviderBindingState::Active
    );
    assert_eq!(
        store
            .get_active_binding_for_space(scope, space.id)
            .await
            .expect("resolve active")
            .expect("active binding")
            .id,
        first.id
    );

    let second = create_tested_binding(
        &store,
        scope,
        space.id,
        "engine.knowledge.external.ragflow",
        "dataset",
        "dataset-b",
        None,
    )
    .await;
    let second = store
        .activate_binding(scope, second.id, "tenant-admin", second.version)
        .await
        .expect("atomically switch binding");
    assert_eq!(
        second.lifecycle_state,
        KnowledgeEngineProviderBindingState::Active
    );
    assert_eq!(
        store
            .get_binding(scope, first.id)
            .await
            .expect("retained predecessor")
            .lifecycle_state,
        KnowledgeEngineProviderBindingState::Disabled
    );

    let wrong_organization = KnowledgeEngineProviderScope {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id + 1,
    };
    assert!(store
        .get_active_binding_for_space(wrong_organization, space.id)
        .await
        .expect("isolated lookup")
        .is_none());

    let page = store
        .list_bindings(
            scope,
            ListKnowledgeEngineProviderBindingsRequest {
                space_id: Some(space.id),
                lifecycle_state: None,
                cursor: None,
                page_size: Some(1),
            },
        )
        .await
        .expect("list first page");
    assert_eq!(page.items.len(), 1);
    assert!(page.next_cursor.is_some());
}

#[tokio::test]
async fn provider_binding_rejects_stale_versions_and_cross_implementation_credentials() {
    let Some(pool) = provider_test_pool(100_002, 7_002).await else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    let scope = KnowledgeEngineProviderScope {
        tenant_id: 100_002,
        organization_id: 7_002,
    };
    clean_provider_scope(&pool, scope.tenant_id, scope.organization_id).await;
    let space_store =
        PostgresKnowledgeSpaceStore::new(pool.clone(), scope.tenant_id, scope.organization_id);
    let space = space_store
        .create_space(CreateKnowledgeSpaceRecord {
            name: "Provider Concurrency Space".to_string(),
            description: None,
            okf_bundle_initialized: false,
            knowledge_mode:
                sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode::External,
        })
        .await
        .expect("create external space");
    let store = SqlxKnowledgeEngineProviderBindingStore::new(pool);
    let credential = store
        .create_credential_reference(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderCredentialReferenceRequest {
                implementation_id: "engine.knowledge.external.dify".to_string(),
                display_name: "Dify credential".to_string(),
                reference_locator: "secret://knowledgebase/dify/concurrency".to_string(),
            },
        )
        .await
        .expect("create credential");

    let mismatch = store
        .create_binding(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderBindingRequest {
                space_id: space.id,
                implementation_id: "engine.knowledge.external.ragflow".to_string(),
                remote_resource_type: "dataset".to_string(),
                remote_resource_id: "dataset-ragflow".to_string(),
                credential_reference_id: Some(credential.id),
            },
        )
        .await;
    assert!(matches!(
        mismatch,
        Err(KnowledgeEngineProviderBindingStoreError::CredentialUnavailable(id)) if id == credential.id
    ));

    let binding = store
        .create_binding(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderBindingRequest {
                space_id: space.id,
                implementation_id: "engine.knowledge.external.dify".to_string(),
                remote_resource_type: "dataset".to_string(),
                remote_resource_id: "dataset-dify".to_string(),
                credential_reference_id: Some(credential.id),
            },
        )
        .await
        .expect("create binding");
    let updated = store
        .update_draft_binding(
            scope,
            binding.id,
            "tenant-admin",
            UpdateKnowledgeEngineProviderBindingRequest {
                remote_resource_type: None,
                remote_resource_id: Some("dataset-dify-v2".to_string()),
                credential_reference_id: None,
                clear_credential_reference: false,
                expected_version: binding.version,
            },
        )
        .await
        .expect("update binding");
    assert_eq!(updated.remote_resource_id, "dataset-dify-v2");

    let stale = store
        .update_draft_binding(
            scope,
            binding.id,
            "tenant-admin",
            UpdateKnowledgeEngineProviderBindingRequest {
                remote_resource_type: None,
                remote_resource_id: Some("stale-write".to_string()),
                credential_reference_id: None,
                clear_credential_reference: false,
                expected_version: binding.version,
            },
        )
        .await;
    assert!(matches!(
        stale,
        Err(KnowledgeEngineProviderBindingStoreError::Conflict(_))
    ));
}

#[tokio::test]
async fn provider_credential_lifecycle_is_paginated_versioned_and_revocation_fails_closed() {
    let Some(pool) = provider_test_pool(100_003, 7_003).await else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    let scope = KnowledgeEngineProviderScope {
        tenant_id: 100_003,
        organization_id: 7_003,
    };
    clean_provider_scope(&pool, scope.tenant_id, scope.organization_id).await;
    let store = SqlxKnowledgeEngineProviderBindingStore::new(pool);
    let first = store
        .create_credential_reference(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderCredentialReferenceRequest {
                implementation_id: "engine.knowledge.external.dify".to_string(),
                display_name: "Dify primary".to_string(),
                reference_locator: "env://SDKWORK_DIFY_PRIMARY_KEY".to_string(),
            },
        )
        .await
        .expect("create first credential");
    let second = store
        .create_credential_reference(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderCredentialReferenceRequest {
                implementation_id: "engine.knowledge.external.ragflow".to_string(),
                display_name: "RAGFlow primary".to_string(),
                reference_locator: "env://SDKWORK_RAGFLOW_PRIMARY_KEY".to_string(),
            },
        )
        .await
        .expect("create second credential");

    let first_page = store
        .list_credential_references(
            scope,
            ListKnowledgeEngineProviderCredentialReferencesRequest {
                implementation_id: None,
                rotation_state: None,
                cursor: None,
                page_size: Some(1),
            },
        )
        .await
        .expect("list first credential page");
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.items[0].id, second.id);
    let second_page = store
        .list_credential_references(
            scope,
            ListKnowledgeEngineProviderCredentialReferencesRequest {
                implementation_id: None,
                rotation_state: None,
                cursor: first_page.next_cursor,
                page_size: Some(1),
            },
        )
        .await
        .expect("list second credential page");
    assert_eq!(second_page.items, vec![first.clone()]);
    assert!(second_page.next_cursor.is_none());

    let rotated = store
        .rotate_credential_reference(
            scope,
            first.id,
            "security-operator",
            RotateKnowledgeEngineProviderCredentialReferenceRequest {
                reference_locator: "env://SDKWORK_DIFY_ROTATED_KEY".to_string(),
                expected_version: first.version,
            },
        )
        .await
        .expect("rotate credential");
    assert_eq!(rotated.version, first.version + 1);
    assert_eq!(
        rotated.rotation_state,
        KnowledgeEngineProviderCredentialRotationState::Current
    );
    assert!(rotated.last_rotated_at.is_some());

    let stale_rotation = store
        .rotate_credential_reference(
            scope,
            first.id,
            "security-operator",
            RotateKnowledgeEngineProviderCredentialReferenceRequest {
                reference_locator: "env://SDKWORK_DIFY_STALE_KEY".to_string(),
                expected_version: first.version,
            },
        )
        .await;
    assert!(matches!(
        stale_rotation,
        Err(KnowledgeEngineProviderBindingStoreError::Conflict(_))
    ));

    let revoked = store
        .revoke_credential_reference(
            scope,
            first.id,
            "security-operator",
            RevokeKnowledgeEngineProviderCredentialReferenceRequest {
                expected_version: rotated.version,
            },
        )
        .await
        .expect("revoke credential");
    assert_eq!(
        revoked.rotation_state,
        KnowledgeEngineProviderCredentialRotationState::Revoked
    );
    assert!(matches!(
        store
            .resolve_credential_reference(scope, first.id, &first.implementation_id)
            .await,
        Err(KnowledgeEngineProviderBindingStoreError::CredentialUnavailable(id)) if id == first.id
    ));
    assert!(matches!(
        store
            .rotate_credential_reference(
                scope,
                first.id,
                "security-operator",
                RotateKnowledgeEngineProviderCredentialReferenceRequest {
                    reference_locator: "env://SDKWORK_DIFY_AFTER_REVOKE".to_string(),
                    expected_version: revoked.version,
                },
            )
            .await,
        Err(KnowledgeEngineProviderBindingStoreError::InvalidLifecycle(
            _
        ))
    ));

    let wrong_scope = KnowledgeEngineProviderScope {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id + 1,
    };
    assert!(matches!(
        store.get_credential_reference(wrong_scope, second.id).await,
        Err(KnowledgeEngineProviderBindingStoreError::NotFound(id)) if id == second.id
    ));
}

async fn create_tested_binding(
    store: &SqlxKnowledgeEngineProviderBindingStore,
    scope: KnowledgeEngineProviderScope,
    space_id: u64,
    implementation_id: &str,
    remote_resource_type: &str,
    remote_resource_id: &str,
    credential_reference_id: Option<u64>,
) -> sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineProviderBinding {
    let binding = store
        .create_binding(
            scope,
            "tenant-admin",
            CreateKnowledgeEngineProviderBindingRequest {
                space_id,
                implementation_id: implementation_id.to_string(),
                remote_resource_type: remote_resource_type.to_string(),
                remote_resource_id: remote_resource_id.to_string(),
                credential_reference_id,
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

async fn clean_provider_scope(pool: &sqlx::AnyPool, tenant_id: u64, organization_id: u64) {
    for statement in [
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
            .expect("clean provider binding fixtures");
    }
}

async fn provider_test_pool(tenant_id: u64, organization_id: u64) -> Option<sqlx::AnyPool> {
    support::scoped_pool(tenant_id, organization_id).await
}
