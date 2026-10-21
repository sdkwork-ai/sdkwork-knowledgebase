use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use sdkwork_drive_contract::drive::events::{
    DriveEventEnvelope, DriveNodeDeletedV1Data, DriveRootScopeEffect, DriveRootScopeKind,
};
use sdkwork_intelligence_knowledgebase_repository_sqlx::is_postgres_database_url;
use sdkwork_knowledgebase_worker::{
    run_maintenance_tick, MaintenanceConfig, MaintenanceTickError, MaintenanceTickState,
    WikiDriveEventMaintenanceConfig,
};
use sdkwork_routes_knowledgebase_app_api::{dev_auth, paths, KnowledgebaseRuntime};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::util::ServiceExt;

fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn ingest_appends_outbox_event_and_worker_publishes_it() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = dev_auth::with_dev_app_auth_for_organization(
        runtime.build_full_app_router(),
        1,
        Some(42),
        Some(runtime.organization_id()),
    );
    let space_id = create_space(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(paths::INGESTS)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "spaceId": space_id,
                        "title": "integration-ingest",
                        "idempotencyKey": format!("integration-{}", unique_suffix()),
                        "payloadMarkdown": "# Hello\n\nIntegration ingest body."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body_text = response_body_string(response).await;
    assert_eq!(status, StatusCode::CREATED, "ingest failed: {body_text}");
    let job: Value = serde_json::from_str(&body_text).expect("parse ingest json");
    assert_eq!(job["data"]["item"]["state"], "succeeded");

    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_outbox_event WHERE tenant_id = 1 AND status = 0 AND event_type = 'knowledge.ingest.succeeded'",
    )
    .fetch_one(runtime.pool())
    .await
    .expect("count pending outbox");
    assert_eq!(pending, 1);

    let maintenance_config = maintenance_config();
    let tick = run_maintenance_tick(
        &runtime,
        &maintenance_config,
        MaintenanceTickState::default(),
    )
    .await
    .expect("maintenance tick");
    assert_eq!(tick.outbox_requeued, 0);
    assert_eq!(tick.outbox_published, 1);
    assert_eq!(tick.outbox_failed, 0);

    let still_pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kb_outbox_event WHERE tenant_id = 1 AND status = 0",
    )
    .fetch_one(runtime.pool())
    .await
    .expect("count pending after publish");
    assert_eq!(still_pending, 0);

    let checkpoint = sqlx::query(
        "SELECT checkpoint.id, checkpoint.source_scope_uuid, checkpoint.drive_space_uuid
         FROM kb_drive_source_checkpoint checkpoint
         INNER JOIN kb_site_publication publication
           ON publication.tenant_id = checkpoint.tenant_id
          AND publication.organization_id = checkpoint.organization_id
          AND publication.id = checkpoint.site_publication_id
         WHERE publication.tenant_id = 1
           AND publication.organization_id = 42
           AND publication.space_id = $1
           AND publication.status = 1
           AND checkpoint.status = 1",
    )
    .bind(i64::try_from(space_id).expect("space id fits signed bigint"))
    .fetch_one(runtime.pool())
    .await
    .expect("created space has an active Wiki Drive checkpoint");
    let checkpoint_id: i64 = checkpoint.get("id");
    let source_scope_uuid: String = checkpoint.get("source_scope_uuid");
    let drive_space_uuid: String = checkpoint.get("drive_space_uuid");
    let outbox_id = format!("outbox-{}", uuid::Uuid::new_v4());
    let event_id = format!("event-{}", uuid::Uuid::new_v4());
    let drive_node_uuid = uuid::Uuid::new_v4().to_string();
    let payload_json = serde_json::to_string(&DriveEventEnvelope::new(
        event_id.clone(),
        "drive.node.deleted.v1",
        "2026-07-21T00:00:00Z",
        "1",
        Some("42".to_string()),
        format!("drive://spaces/{drive_space_uuid}/nodes/{drive_node_uuid}"),
        "9001",
        1,
        DriveNodeDeletedV1Data {
            operation_id: uuid::Uuid::new_v4().to_string(),
            space_id: drive_space_uuid.clone(),
            node_id: drive_node_uuid.clone(),
            drive_uri: format!("drive://spaces/{drive_space_uuid}/nodes/{drive_node_uuid}"),
            drive_version_id: None,
            version_no: None,
            last_space_relative_path: "sources/raw/guide/removed.md".to_string(),
            deletion_reason: "PERMANENT_DELETE".to_string(),
            root_scopes: vec![DriveRootScopeEffect {
                scope_id: source_scope_uuid.clone(),
                scope_kind: DriveRootScopeKind::KnowledgebaseRaw,
                relative_path: "guide/removed.md".to_string(),
                root_generation: Some("1".to_string()),
            }],
        },
    ))
    .expect("serialize Drive event");
    sqlx::query(
        "INSERT INTO dr_drive_domain_outbox (
            id, tenant_id, space_id, node_id, event_type, actor_id, sequence_no, payload_json
         ) VALUES ($1, '1', $2, $3, 'drive.node.deleted.v1', '9001', 1, $4)",
    )
    .bind(&outbox_id)
    .bind(&drive_space_uuid)
    .bind(&drive_node_uuid)
    .bind(&payload_json)
    .execute(runtime.pool())
    .await
    .expect("seed Drive outbox event");

    let relay_tick = run_maintenance_tick(
        &runtime,
        &maintenance_config,
        MaintenanceTickState::default(),
    )
    .await
    .expect("relay and apply Drive event in one maintenance tick");
    assert!(relay_tick.wiki_drive_outbox_events_processed >= 1);
    assert_eq!(relay_tick.wiki_drive_outbox_events_delivered, 1);
    assert_eq!(relay_tick.wiki_drive_events_applied, 1);

    let drive_outbox_status: String =
        sqlx::query_scalar("SELECT delivery_status FROM dr_drive_domain_outbox WHERE id = $1")
            .bind(&outbox_id)
            .fetch_one(runtime.pool())
            .await
            .expect("read relayed Drive outbox status");
    assert_eq!(drive_outbox_status, "delivered");
    let inbox_state: String = sqlx::query_scalar(
        "SELECT processing_state FROM kb_drive_event_inbox
         WHERE tenant_id = 1 AND organization_id = 42 AND source_event_id = $1",
    )
    .bind(&event_id)
    .fetch_one(runtime.pool())
    .await
    .expect("read relayed Knowledgebase inbox event");
    assert_eq!(inbox_state, "APPLIED");
    let checkpoint_sequence: i64 =
        sqlx::query_scalar("SELECT last_sequence_no FROM kb_drive_source_checkpoint WHERE id = $1")
            .bind(checkpoint_id)
            .fetch_one(runtime.pool())
            .await
            .expect("read advanced Wiki Drive checkpoint");
    assert_eq!(checkpoint_sequence, 1);
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn maintenance_tick_propagates_outbox_store_failure() {
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    runtime.pool().close().await;

    let error = run_maintenance_tick(
        &runtime,
        &maintenance_config(),
        MaintenanceTickState::default(),
    )
    .await
    .expect_err("closed outbox store must fail the maintenance tick");

    assert!(matches!(error, MaintenanceTickError::Outbox(_)));
}

fn maintenance_config() -> MaintenanceConfig {
    MaintenanceConfig {
        worker_id: "integration-worker".to_string(),
        ingestion_job_lease: time::Duration::minutes(5),
        provider_migration_lease: std::time::Duration::from_secs(120),
        outbox_limit: 10,
        ingestion_job_limit: 10,
        provider_migration_limit: 10,
        group_archive_limit: 10,
        ingestion_max_attempts: 20,
        wiki_backfill: None,
        wiki_drive_events: wiki_drive_event_config(),
        audit_retention: None,
        phase_timeout: std::time::Duration::from_secs(60),
    }
}

fn wiki_drive_event_config() -> WikiDriveEventMaintenanceConfig {
    WikiDriveEventMaintenanceConfig {
        tenant_id: 1,
        organization_id: 42,
        actor_id: 9001,
        checkpoint_page_size: 50,
        event_batch_size: 25,
        lease_seconds: 120,
        retry_delay_seconds: 30,
        max_attempts: 20,
        source_batch_size: 10,
        source_lease_seconds: 120,
        source_retry_delay_seconds: 30,
        source_max_attempts: 10,
        delivery_renewal_page_size: 50,
    }
}

async fn create_space(app: &axum::Router) -> u64 {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(paths::SPACES)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "integration-outbox-space",
                        "description": "integration outbox test space"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body_text = response_body_string(response).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create space failed: {body_text}"
    );
    let body: Value = serde_json::from_str(&body_text).expect("parse create space response");
    json_item_u64_field(&body, "id").expect("created space id")
}

fn json_item_u64_field(body: &Value, field: &str) -> Option<u64> {
    body.pointer("/data/item")
        .and_then(|item| json_u64_field(item, field))
}

fn json_u64_field(body: &Value, field: &str) -> Option<u64> {
    body.get(field)
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
}

async fn response_body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    String::from_utf8_lossy(&bytes).into_owned()
}

async fn test_runtime() -> Option<KnowledgebaseRuntime> {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return None;
    };
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let work_dir = std::env::current_dir().expect("cwd");
    let test_root = work_dir
        .join("target")
        .join("integration-ingest-outbox-tests")
        .join(format!("{}-{}-{}", std::process::id(), nanos, sequence));
    std::fs::create_dir_all(&test_root).expect("mkdir test root");

    let drive_root = test_root.join("drive-objects");
    std::fs::create_dir_all(&drive_root).expect("mkdir drive root");
    std::env::set_var(
        "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT",
        drive_root.to_string_lossy().as_ref(),
    );

    std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "development");
    std::env::set_var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID", "42");
    std::env::remove_var("SDKWORK_KNOWLEDGEBASE_OUTBOX_WEBHOOK_URL");
    std::env::remove_var("SDKWORK_KNOWLEDGEBASE_OUTBOX_WEBHOOK_SECRET_FILE");

    KnowledgebaseRuntime::connect(&database_url, 1)
        .await
        .expect("connect runtime")
        .into()
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
}
