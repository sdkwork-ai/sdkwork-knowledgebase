mod support;

use sdkwork_intelligence_knowledgebase_repository_sqlx::SqlxKnowledgeEngineProviderBindingReadinessStore;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_readiness_store::{
    KnowledgeEngineProviderBindingReadinessStore,
    KnowledgeEngineProviderBindingReadinessStoreError,
    ListKnowledgeEngineProviderBindingReadinessGapsRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_binding_store::KnowledgeEngineProviderScope;
use sqlx::AnyPool;

const TENANT_ID: i64 = 100_001;
const ORGANIZATION_ID: i64 = 7_001;
const NOW: &str = "2026-07-20T00:00:00Z";

#[tokio::test]
async fn readiness_report_is_scoped_bounded_and_never_infers_from_sources() {
    let Some(pool) = readiness_test_pool("provider-binding-readiness-scope").await else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    for statement in [
        "DELETE FROM kb_provider_binding WHERE tenant_id = $1 AND organization_id = $2",
        "DELETE FROM kb_source WHERE tenant_id = $1 AND organization_id = $2",
        "DELETE FROM kb_space WHERE tenant_id = $1 AND organization_id = $2",
    ] {
        sqlx::query(statement)
            .bind(TENANT_ID)
            .bind(ORGANIZATION_ID)
            .execute(&pool)
            .await
            .expect("clean readiness fixtures");
    }

    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 900, "external", 1).await;
    insert_source_with_provider(
        &pool,
        TENANT_ID,
        ORGANIZATION_ID,
        900,
        "engine.knowledge.external.dify",
    )
    .await;

    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 800, "external", 1).await;
    for (index, lifecycle_state) in ["draft", "testing", "failed", "disabled"]
        .into_iter()
        .enumerate()
    {
        insert_binding(
            &pool,
            TENANT_ID,
            ORGANIZATION_ID,
            8_000 + index as i64,
            800,
            lifecycle_state,
            1,
        )
        .await;
    }

    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 700, "external", 1).await;
    insert_binding(&pool, TENANT_ID, ORGANIZATION_ID, 7_000, 700, "degraded", 1).await;

    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 600, "external", 1).await;
    insert_binding(&pool, TENANT_ID, ORGANIZATION_ID, 6_000, 600, "active", 1).await;

    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 500, "rag", 1).await;
    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 400, "external", 0).await;

    insert_space(&pool, TENANT_ID, ORGANIZATION_ID, 300, "external", 1).await;
    insert_binding(&pool, TENANT_ID, ORGANIZATION_ID, 3_000, 300, "active", 0).await;

    // Cross-scope fixtures must be written through a pool that carries that scope: a
    // production-scoped session cannot create rows for another tenant/organization under
    // FORCE ROW LEVEL SECURITY, which is exactly the isolation being asserted.
    let cross_tenant_pool = support::scoped_pool((TENANT_ID + 1) as u64, ORGANIZATION_ID as u64)
        .await
        .expect("connect cross-tenant readiness pool");
    let cross_organization_pool =
        support::scoped_pool(TENANT_ID as u64, (ORGANIZATION_ID + 1) as u64)
            .await
            .expect("connect cross-organization readiness pool");
    insert_space(
        &cross_tenant_pool,
        TENANT_ID + 1,
        ORGANIZATION_ID,
        1_100,
        "external",
        1,
    )
    .await;
    insert_space(
        &cross_organization_pool,
        TENANT_ID,
        ORGANIZATION_ID + 1,
        1_000,
        "external",
        1,
    )
    .await;

    let scope = KnowledgeEngineProviderScope {
        tenant_id: TENANT_ID as u64,
        organization_id: ORGANIZATION_ID as u64,
    };
    let store = SqlxKnowledgeEngineProviderBindingReadinessStore::new(pool);
    let first = store
        .list_spaces_missing_active_binding(
            scope,
            ListKnowledgeEngineProviderBindingReadinessGapsRequest {
                cursor: None,
                page_size: Some(2),
            },
        )
        .await
        .expect("first readiness page");

    assert_eq!(
        first
            .items
            .iter()
            .map(|item| item.space_id)
            .collect::<Vec<_>>(),
        vec![900, 800]
    );
    assert_eq!(first.items[0].non_active_binding_count, 0);
    assert_eq!(first.items[1].non_active_binding_count, 4);
    let first_json = serde_json::to_string(&first).expect("serialize readiness page");
    assert!(!first_json.contains("engine.knowledge.external.dify"));
    assert!(!first_json.contains("External space"));
    assert!(!first_json.contains("remoteResource"));
    assert!(!first_json.contains("credential"));

    let cursor = first.next_cursor.expect("bounded first page cursor");
    assert!(cursor.parse::<u64>().is_err(), "cursor must be opaque");
    let second = store
        .list_spaces_missing_active_binding(
            scope,
            ListKnowledgeEngineProviderBindingReadinessGapsRequest {
                cursor: Some(cursor.clone()),
                page_size: Some(2),
            },
        )
        .await
        .expect("second readiness page");
    assert_eq!(
        second
            .items
            .iter()
            .map(|item| (item.space_id, item.non_active_binding_count))
            .collect::<Vec<_>>(),
        vec![(700, 1), (300, 0)]
    );
    assert!(second.next_cursor.is_none());

    let wrong_scope = KnowledgeEngineProviderScope {
        tenant_id: scope.tenant_id,
        organization_id: scope.organization_id + 1,
    };
    assert!(matches!(
        store
            .list_spaces_missing_active_binding(
                wrong_scope,
                ListKnowledgeEngineProviderBindingReadinessGapsRequest {
                    cursor: Some(cursor),
                    page_size: Some(2),
                },
            )
            .await,
        Err(KnowledgeEngineProviderBindingReadinessStoreError::InvalidRequest(_))
    ));
}

#[tokio::test]
async fn readiness_report_rejects_invalid_page_and_cursor_inputs() {
    let Some(pool) = readiness_test_pool("provider-binding-readiness-input").await else {
        eprintln!("skipping provider postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    let store = SqlxKnowledgeEngineProviderBindingReadinessStore::new(pool);
    let scope = KnowledgeEngineProviderScope {
        tenant_id: TENANT_ID as u64,
        organization_id: ORGANIZATION_ID as u64,
    };

    for request in [
        ListKnowledgeEngineProviderBindingReadinessGapsRequest {
            cursor: None,
            page_size: Some(0),
        },
        ListKnowledgeEngineProviderBindingReadinessGapsRequest {
            cursor: None,
            page_size: Some(201),
        },
        ListKnowledgeEngineProviderBindingReadinessGapsRequest {
            cursor: Some("800".to_string()),
            page_size: Some(20),
        },
        ListKnowledgeEngineProviderBindingReadinessGapsRequest {
            cursor: Some(sdkwork_utils_rust::base64url_encode(
                b"knowledgebase-provider-binding-readiness:v1:0100001:7001:800",
            )),
            page_size: Some(20),
        },
    ] {
        assert!(matches!(
            store
                .list_spaces_missing_active_binding(scope, request)
                .await,
            Err(KnowledgeEngineProviderBindingReadinessStoreError::InvalidRequest(_))
        ));
    }
}

async fn insert_space(
    pool: &AnyPool,
    tenant_id: i64,
    organization_id: i64,
    id: i64,
    knowledge_mode: &str,
    status: i32,
) {
    sqlx::query(
        r#"
        INSERT INTO kb_space (
            id, uuid, tenant_id, organization_id, name, status,
            okf_bundle_initialized, okf_log_sequence_counter, knowledge_mode,
            created_at, updated_at, version
        ) VALUES ($1, $2, $3, $4, $5, $6, 0, 0, $7, $8::timestamptz, $8::timestamptz, 0)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(format!("space-{tenant_id}-{organization_id}-{id}"))
    .bind(tenant_id)
    .bind(organization_id)
    .bind(format!("External space {id}"))
    .bind(status)
    .bind(knowledge_mode)
    .bind(NOW)
    .execute(pool)
    .await
    .expect("insert space fixture");
}

async fn insert_binding(
    pool: &AnyPool,
    tenant_id: i64,
    organization_id: i64,
    id: i64,
    space_id: i64,
    lifecycle_state: &str,
    status: i32,
) {
    sqlx::query(
        r#"
        INSERT INTO kb_provider_binding (
            id, uuid, tenant_id, organization_id, space_id, implementation_id,
            remote_resource_type, remote_resource_id, lifecycle_state,
            capability_snapshot, capability_snapshot_version, created_by, updated_by,
            status, created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, $4, $5, 'engine.knowledge.external.dify',
            'dataset', $6, $7, '[]', 0, 'test', 'test', $8, $9::timestamptz, $9::timestamptz, 0
        )
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(format!("binding-{tenant_id}-{organization_id}-{id}"))
    .bind(tenant_id)
    .bind(organization_id)
    .bind(space_id)
    .bind(format!("remote-{id}"))
    .bind(lifecycle_state)
    .bind(status)
    .bind(NOW)
    .execute(pool)
    .await
    .expect("insert Provider Binding fixture");
}

async fn insert_source_with_provider(
    pool: &AnyPool,
    tenant_id: i64,
    organization_id: i64,
    space_id: i64,
    provider: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO kb_source (
            id, uuid, tenant_id, organization_id, space_id, source_type, provider, status,
            created_at, updated_at, version
        ) VALUES ($1, $2, $3, $4, $5, 'connector', $6, 1, $7::timestamptz, $7::timestamptz, 0)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(90_000 + space_id)
    .bind(format!("source-{tenant_id}-{space_id}"))
    .bind(tenant_id)
    .bind(organization_id)
    .bind(space_id)
    .bind(provider)
    .bind(NOW)
    .execute(pool)
    .await
    .expect("insert historic source fixture");
}

async fn readiness_test_pool(_test_name: &str) -> Option<AnyPool> {
    support::scoped_pool(TENANT_ID as u64, ORGANIZATION_ID as u64).await
}
