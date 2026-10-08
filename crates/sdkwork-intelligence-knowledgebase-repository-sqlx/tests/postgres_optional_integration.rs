use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    connect_postgres_and_install_schema, is_postgres_database_url, knowledgebase_health_check,
    KnowledgeAuditEventRecord, PostgresIngestionJobStore, PostgresKnowledgeAuditEventStore,
    PostgresKnowledgeBrowserProjectionStore, PostgresKnowledgeDocumentStore,
    PostgresKnowledgeDocumentVersionStore, PostgresKnowledgeDriveObjectRefStore,
    PostgresKnowledgeSpaceStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_browser_projection_store::KnowledgeBrowserProjectionStore;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope, KnowledgeDocumentStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_version_store::{
    CreateKnowledgeDocumentVersionRecord, KnowledgeDocumentVersionStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_object_ref_store::{
    CreateKnowledgeDriveObjectRefRecord, KnowledgeDriveObjectRefStore, MANAGED_DRIVE_ACCESS_MODE,
    SDKWORK_DRIVE_PROVIDER_KIND,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::{
    CreateIngestionJobRecord, IngestionJobStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use sdkwork_knowledgebase_observability::KnowledgebaseTenantQuotaLimits;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Barrier;

fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

#[tokio::test]
async fn postgres_repository_health_check_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    knowledgebase_health_check(&pool)
        .await
        .expect("postgres health check");
}

#[tokio::test]
async fn postgres_ingest_quota_advisory_lock_is_atomic_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres quota integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as u64;
    let tenant_id = 700_000_000_u64.saturating_add(nonce % 100_000_000);
    let limits = KnowledgebaseTenantQuotaLimits {
        max_concurrent_ingest_jobs: 2,
        ..KnowledgebaseTenantQuotaLimits::default()
    };
    let store = Arc::new(
        PostgresIngestionJobStore::new(pool.clone(), tenant_id, 0)
            .with_database_engine(DatabaseEngine::Postgres)
            .with_quota_limits(limits),
    );
    let barrier = Arc::new(Barrier::new(11));
    let mut tasks = Vec::new();
    for index in 0..10_u64 {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            store
                .create_or_get_job(CreateIngestionJobRecord {
                    space_id: 7,
                    source_type: "api".to_string(),
                    idempotency_key: format!("postgres-atomic-{nonce}-{index}"),
                    idempotency_fingerprint_sha256_hex: None,
                })
                .await
        }));
    }
    barrier.wait().await;

    let mut succeeded = 0;
    let mut rejected = 0;
    for task in tasks {
        match task.await.expect("join") {
            Ok(_) => succeeded += 1,
            Err(error) if error.to_string().contains("quota exceeded") => rejected += 1,
            Err(error) => panic!("unexpected postgres quota error: {error}"),
        }
    }
    assert_eq!(succeeded, 2);
    assert_eq!(rejected, 8);

    sqlx::query("DELETE FROM kb_ingestion_job WHERE tenant_id = $1")
        .bind(tenant_id as i64)
        .execute(&pool)
        .await
        .expect("cleanup postgres quota probe");
}

#[tokio::test]
async fn postgres_audit_event_table_accepts_append_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres audit integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    let store = PostgresKnowledgeAuditEventStore::new(pool.clone(), 100_001, 7);
    let request_id = format!(
        "req-postgres-audit-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    store
        .append_event(KnowledgeAuditEventRecord {
            id: None,
            uuid: None,
            event_type: "knowledge.backend.admin_operation".to_string(),
            actor_type: "user".to_string(),
            actor_id: "99".to_string(),
            resource_type: "backend_operation".to_string(),
            resource_id: None,
            result: "success".to_string(),
            request_id: Some(request_id.clone()),
            trace_id: None,
            payload: Some(serde_json::json!({"probe": "postgres-audit"})),
            created_at: None,
        })
        .await
        .expect("append audit event");

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM kb_audit_event WHERE tenant_id = 100001 AND request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("count audit rows");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn postgres_agent_profile_create_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping postgres agent profile integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };

    use sdkwork_intelligence_knowledgebase_repository_sqlx::PostgresKnowledgeAgentProfileStore;
    use sdkwork_intelligence_knowledgebase_service::ports::knowledge_agent_profile_store::KnowledgeAgentProfileStore;
    use sdkwork_knowledgebase_contract::rag::{KnowledgeAgentProfileRequest, KnowledgeAgentStatus};

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    let store = PostgresKnowledgeAgentProfileStore::new(pool, 100_001, 0);
    let created = store
        .create_profile(KnowledgeAgentProfileRequest {
            tenant_id: 100_001,
            name: format!(
                "postgres-agent-profile-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ),
            description: Some("postgres agent profile integration".to_string()),
            system_instruction: "Answer with citations.".to_string(),
            model_provider_id: "provider.model.knowledgebase-contract".to_string(),
            model_id: "contract".to_string(),
            model_parameters: Some(r#"{"temperature":0.7}"#.to_string()),
            retrieval_profile_id: None,
            citation_policy: None,
            memory_policy_ref: None,
            tool_policy_ref: None,
            answer_policy: None,
            status: KnowledgeAgentStatus::Active,
            knowledge_mode: Default::default(),
            agent_implementation_id:
                sdkwork_knowledgebase_contract::default_agent_implementation_id(),
        })
        .await
        .expect("create agent profile on postgres");
    assert!(created.profile_id > 0);
}

#[tokio::test]
async fn postgres_create_space_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres create space integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    let store = PostgresKnowledgeSpaceStore::new(pool, 100_001, 0);
    let created = store
        .create_space(CreateKnowledgeSpaceRecord {
            name: "postgres integration space".to_string(),
            description: Some("created by postgres_optional_integration".to_string()),
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::OkfBundle,
        })
        .await
        .expect("create knowledge space on postgres");
    assert!(created.id > 0);
}

#[tokio::test]
async fn postgres_browser_projection_batches_document_status_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping postgres browser projection integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    let tenant_id = 100_001;
    let documents = PostgresKnowledgeDocumentStore::new(pool.clone(), tenant_id, 0);
    let object_refs = PostgresKnowledgeDriveObjectRefStore::new(pool.clone(), tenant_id, 0);
    let versions = PostgresKnowledgeDocumentVersionStore::new(pool.clone(), tenant_id, 0);
    let projections = PostgresKnowledgeBrowserProjectionStore::new(pool, tenant_id, 0);

    let object_ref = object_refs
        .create_object_ref(CreateKnowledgeDriveObjectRefRecord {
            space_id: 7,
            drive_space_id: Some("drv-kb-001".to_string()),
            drive_node_id: Some("node-pdf".to_string()),
            logical_path: Some("raw/documents/doc-1/original/report.pdf".to_string()),
            drive_provider_kind: SDKWORK_DRIVE_PROVIDER_KIND.to_string(),
            drive_storage_provider_id: "provider-kb".to_string(),
            drive_bucket: "knowledgebase-test".to_string(),
            drive_object_key: "raw/documents/doc-1/original/report.pdf".to_string(),
            drive_object_version: None,
            drive_etag: None,
            content_type: Some("application/pdf".to_string()),
            size_bytes: 42,
            checksum_sha256_hex: Some("checksum".to_string()),
            object_role: "original_document".to_string(),
            access_mode: MANAGED_DRIVE_ACCESS_MODE.to_string(),
        })
        .await
        .expect("create drive object ref on postgres");
    let document = documents
        .create_document(CreateKnowledgeDocumentRecord {
            space_id: 7,
            source_id: None,
            identity_scope: KnowledgeDocumentIdentityScope::SourceAndOriginalDriveNode,
            original_file_drive_node_id: Some("node-pdf".to_string()),
            title: "Report".to_string(),
            mime_type: Some("application/pdf".to_string()),
            language: Some("en".to_string()),
        })
        .await
        .expect("create document on postgres");
    let version = versions
        .create_document_version(CreateKnowledgeDocumentVersionRecord {
            document_id: document.id,
            version_no: 1,
            original_object_ref_id: object_ref.id,
            checksum_sha256_hex: object_ref.checksum_sha256_hex.clone(),
            size_bytes: object_ref.size_bytes,
            mime_type: object_ref.content_type.clone(),
        })
        .await
        .expect("create document version on postgres");

    let batch = projections
        .batch_document_projections(7, vec!["node-folder".to_string(), "node-pdf".to_string()])
        .await
        .expect("batch browser document projections on postgres");

    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].drive_node_id, "node-pdf");
    assert_eq!(batch[0].document_id, document.id);
    assert_eq!(batch[0].current_version_id, Some(version.id));
}

#[tokio::test]
async fn postgres_rls_hides_rows_without_session_scope_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres RLS test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");

    // Write one row through a session bound to tenant 1 / organization 1.
    let store = PostgresKnowledgeSpaceStore::new(pool.clone(), 1, 1);
    let created = store
        .create_space(CreateKnowledgeSpaceRecord {
            name: "RLS fail-closed fixture".to_string(),
            description: None,
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .expect("create space under tenant 1");

    // A session without `app.current_tenant_id`/`app.current_organization_id` must see
    // zero rows (FORCE ROW LEVEL SECURITY policies fail closed on missing variables).
    let unscoped = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("connect unscoped pool");
    let visible: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_space WHERE id = $1")
        .bind(i64::try_from(created.id).expect("space id fits i64"))
        .fetch_one(&unscoped)
        .await
        .expect("count rows without scope");
    assert_eq!(
        visible, 0,
        "RLS must hide rows when session scope variables are absent"
    );

    // A session bound to a different organization must also see zero rows.
    let other_org = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("connect other-org pool");
    sqlx::query("SELECT set_config('app.current_tenant_id', '1', false)")
        .execute(&other_org)
        .await
        .expect("set tenant scope");
    sqlx::query("SELECT set_config('app.current_organization_id', '999', false)")
        .execute(&other_org)
        .await
        .expect("set organization scope");
    let visible_other: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_space WHERE id = $1")
        .bind(i64::try_from(created.id).expect("space id fits i64"))
        .fetch_one(&other_org)
        .await
        .expect("count rows in other organization");
    assert_eq!(
        visible_other, 0,
        "RLS must hide rows for a mismatched organization scope"
    );
}
