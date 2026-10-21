use sdkwork_database_config::{
    workspace_database::normalize_workspace_postgres_url, DatabaseEngine,
};
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    connect_postgres_and_install_schema, is_postgres_database_url, knowledgebase_health_check,
    CreateKnowledgeAuditLegalHold, KnowledgeAuditEventRecord, PostgresIngestionJobStore,
    PostgresKnowledgeAuditEventStore, PostgresKnowledgeBrowserProjectionStore,
    PostgresKnowledgeDocumentStore, PostgresKnowledgeDocumentVersionStore,
    PostgresKnowledgeDriveObjectRefStore, PostgresKnowledgeSpaceStore,
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Barrier;
use url::Url;

/// Connection capacity for the per-test pools. The PostgreSQL integration probes are small,
/// but the quota probe drives ten concurrent ingestion-job writes and must not starve.
const SCOPED_POOL_MAX_CONNECTIONS: u32 = 16;

fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

/// Builds the deployment-schema-normalized PostgreSQL URL and injects the tenant/organization
/// RLS session scope the same way the production pool bootstrap does.
fn postgres_url_with_rls_scope(base_url: &str, tenant_id: u64, organization_id: u64) -> String {
    let normalized = normalize_workspace_postgres_url(base_url.trim())
        .expect("normalize postgres workspace url");
    sdkwork_intelligence_knowledgebase_repository_sqlx::db::bootstrap::postgres_url_with_deployment_scope(
        &normalized,
        tenant_id,
        organization_id,
    )
    .expect("inject postgres RLS deployment scope")
}

/// Removes every `options` parameter from the PostgreSQL URL so a probe pool can prove the
/// RLS policies fail closed when no session scope is present at all.
fn postgres_url_without_session_scope(base_url: &str) -> String {
    let mut url = Url::parse(base_url.trim()).expect("parse postgres database url");
    let retained = url
        .query_pairs()
        .filter(|(key, _)| !key.eq_ignore_ascii_case("options"))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    url.set_query(None);
    if !retained.is_empty() {
        url.query_pairs_mut().extend_pairs(retained);
    }
    let unscoped: String = url.into();
    let parsed = Url::parse(&unscoped).expect("reparse unscoped postgres url");
    assert!(
        !parsed
            .query_pairs()
            .any(|(key, _)| key.eq_ignore_ascii_case("options")),
        "the unscoped RLS probe URL must not carry any connection options"
    );
    assert!(
        !unscoped
            .to_ascii_lowercase()
            .contains("app.current_tenant_id"),
        "the unscoped RLS probe URL must not carry a tenant session scope"
    );
    unscoped
}

/// Installs/upgrades the Knowledgebase schema through the production lifecycle.
///
/// `connect_postgres_and_install_schema` always re-derives the deployment scope from
/// `SDKWORK_KNOWLEDGEBASE_TENANT_ID` / `SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID` and rejects a URL
/// that already carries a tenant/organization scope (single source of truth), so the schema
/// bootstrap must run against the raw deployment URL.
async fn install_postgres_schema(base_url: &str) {
    connect_postgres_and_install_schema(base_url)
        .await
        .expect("install postgres knowledgebase schema")
        .close()
        .await;
}

/// Connects an `AnyPool` whose PostgreSQL sessions carry the RLS scope of the store under test.
///
/// The schema is installed through the production lifecycle first, then the pool is opened from
/// the deployment-scoped URL so every connection (not only the first one) sees the same
/// `app.current_tenant_id` / `app.current_organization_id` values.
async fn connect_scoped_pool(
    base_url: &str,
    tenant_id: u64,
    organization_id: u64,
) -> sqlx::AnyPool {
    let scoped_url = postgres_url_with_rls_scope(base_url, tenant_id, organization_id);
    install_postgres_schema(base_url).await;
    sqlx::any::install_default_drivers();
    sqlx::any::AnyPoolOptions::new()
        .max_connections(SCOPED_POOL_MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(60))
        .connect(&scoped_url)
        .await
        .expect("connect postgres pool with RLS deployment scope")
}

/// Typed `PgPool` variant of [`connect_scoped_pool`] for repositories whose constructors take a
/// `sqlx::PgPool` (retrieval and embedding backends). The schema must already be installed by a
/// preceding [`connect_scoped_pool`] call.
async fn connect_scoped_pg_pool(
    base_url: &str,
    tenant_id: u64,
    organization_id: u64,
) -> sqlx::PgPool {
    let scoped_url = postgres_url_with_rls_scope(base_url, tenant_id, organization_id);
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(SCOPED_POOL_MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(60))
        .connect(&scoped_url)
        .await
        .expect("connect postgres typed pool with RLS deployment scope")
}

#[tokio::test]
async fn postgres_repository_health_check_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    // The health check is scope-independent; it still needs a scoped pool so the schema
    // bootstrap and the probe use the same kind of connection as every other Postgres probe.
    let pool = connect_scoped_pool(&database_url, 1, 0).await;
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

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as u64;
    let tenant_id = 700_000_000_u64.saturating_add(nonce % 100_000_000);
    let pool = connect_scoped_pool(&database_url, tenant_id, 0).await;
    // `kb_ingestion_job` is FK-bound to `kb_space`, so the probe owns a fixture space.
    let space = PostgresKnowledgeSpaceStore::new(pool.clone(), tenant_id, 0)
        .create_space(CreateKnowledgeSpaceRecord {
            name: format!("postgres quota fixture space {nonce}"),
            description: Some("quota advisory lock probe".to_string()),
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .expect("create postgres quota fixture space");
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
                    space_id: space.id,
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
    sqlx::query("DELETE FROM kb_space WHERE id = $1")
        .bind(space.id as i64)
        .execute(&pool)
        .await
        .expect("cleanup postgres quota fixture space");
}

#[tokio::test]
async fn postgres_audit_event_table_accepts_append_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres audit integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_scoped_pool(&database_url, 100_001, 7).await;
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

    // The verification query must run on the same tenant/organization scope as the write;
    // RLS hides the row from any other session.
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM kb_audit_event WHERE tenant_id = 100001 AND request_id = $1",
    )
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("count audit rows");
    assert_eq!(count.0, 1);
}

/// Proves the retention purge and legal-hold predicate against a live PostgreSQL schema:
/// an expired event is deleted, while an equally old event whose actor is under an active
/// legal hold survives. Uses direct inserts because `append_event` always stamps `now()`.
#[tokio::test]
async fn postgres_audit_retention_respects_legal_holds_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping postgres audit retention integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };

    let pool = connect_scoped_pool(&database_url, 100_001, 7).await;
    let store = PostgresKnowledgeAuditEventStore::new(pool.clone(), 100_001, 7);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let unheld_event_id = 7_900_001_i64 + (nonce % 100_000) as i64;
    let held_event_id = unheld_event_id + 1;
    let held_actor = format!("retention-held-actor-{nonce}");
    let unheld_actor = format!("retention-unheld-actor-{nonce}");

    sqlx::query(
        r#"
        INSERT INTO kb_audit_event (
            id, uuid, tenant_id, organization_id, event_type, actor_type, actor_id,
            resource_type, result, payload, created_at, version
        )
        VALUES
            ($1, $3, 100001, 7, 'probe', 'user', $5, 'space', 'success', NULL,
             now() - interval '400 days', 0),
            ($2, $4, 100001, 7, 'probe', 'user', $6, 'space', 'success', NULL,
             now() - interval '400 days', 0)
        "#,
    )
    .bind(unheld_event_id)
    .bind(held_event_id)
    .bind(format!("retention-unheld-{nonce}"))
    .bind(format!("retention-held-{nonce}"))
    .bind(&unheld_actor)
    .bind(&held_actor)
    .execute(&pool)
    .await
    .expect("insert expired audit probes");

    let hold = store
        .create_legal_hold(CreateKnowledgeAuditLegalHold {
            subject_type: "actor".to_string(),
            subject_id: held_actor.clone(),
            reason: "postgres retention drill".to_string(),
            case_reference: Some("CASE-RETENTION".to_string()),
            created_by: "integration-test".to_string(),
        })
        .await
        .expect("create legal hold");

    let cutoff = (SystemTime::now() - Duration::from_secs(365 * 86_400))
        .duration_since(UNIX_EPOCH)
        .expect("cutoff")
        .as_secs();
    let cutoff = time::OffsetDateTime::from_unix_timestamp(cutoff as i64)
        .expect("cutoff timestamp")
        .format(&time::format_description::well_known::Rfc3339)
        .expect("format cutoff");
    let purged = store
        .purge_expired_events(&cutoff, 500)
        .await
        .expect("purge expired events");
    assert!(purged >= 1, "the unheld expired event must be purged");

    let remaining: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM kb_audit_event WHERE id IN ($1, $2)")
            .bind(unheld_event_id)
            .bind(held_event_id)
            .fetch_one(&pool)
            .await
            .expect("count remaining probes");
    assert_eq!(remaining.0, 1, "only the legally held event may remain");

    let released = store
        .release_legal_hold(hold.hold_id, "integration-test")
        .await
        .expect("release legal hold");
    assert_eq!(released.status, 0);
    assert_eq!(released.released_by.as_deref(), Some("integration-test"));

    sqlx::query("DELETE FROM kb_audit_event WHERE id IN ($1, $2)")
        .bind(unheld_event_id)
        .bind(held_event_id)
        .execute(&pool)
        .await
        .expect("cleanup probe events");
    sqlx::query("DELETE FROM kb_audit_legal_hold WHERE id = $1")
        .bind(hold.hold_id as i64)
        .execute(&pool)
        .await
        .expect("cleanup legal hold");
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

    let pool = connect_scoped_pool(&database_url, 100_001, 0).await;
    let store = PostgresKnowledgeAgentProfileStore::new(pool, 100_001, 0);
    let created = store
        .create_profile(KnowledgeAgentProfileRequest {
            tenant_id: 100_001,
            created_by: Some(42_424),
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
    assert_eq!(
        created.created_by, 42_424,
        "created_by must round-trip through the INSERT ... RETURNING mapping"
    );
    let retrieved = store
        .retrieve_profile(created.profile_id)
        .await
        .expect("retrieve agent profile on postgres");
    assert_eq!(
        retrieved.created_by, 42_424,
        "created_by must round-trip through the SELECT mapping"
    );
}

#[tokio::test]
async fn postgres_create_space_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!("skipping postgres create space integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };

    let pool = connect_scoped_pool(&database_url, 100_001, 0).await;
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

    let tenant_id = 100_001;
    let pool = connect_scoped_pool(&database_url, tenant_id, 0).await;
    let spaces = PostgresKnowledgeSpaceStore::new(pool.clone(), tenant_id, 0);
    // The drive object ref, document and document version are FK-bound to a knowledge space;
    // a fresh space per run also keeps the fixed drive fixture rerunnable.
    let space = spaces
        .create_space(CreateKnowledgeSpaceRecord {
            name: "postgres browser projection space".to_string(),
            description: Some("created by postgres_optional_integration".to_string()),
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        })
        .await
        .expect("create browser projection space on postgres");
    let documents = PostgresKnowledgeDocumentStore::new(pool.clone(), tenant_id, 0);
    let object_refs = PostgresKnowledgeDriveObjectRefStore::new(pool.clone(), tenant_id, 0);
    let versions = PostgresKnowledgeDocumentVersionStore::new(pool.clone(), tenant_id, 0);
    let projections = PostgresKnowledgeBrowserProjectionStore::new(pool, tenant_id, 0);

    let object_ref = object_refs
        .create_object_ref(CreateKnowledgeDriveObjectRefRecord {
            space_id: space.id,
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
            space_id: space.id,
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
        .batch_document_projections(
            space.id,
            vec!["node-folder".to_string(), "node-pdf".to_string()],
        )
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

    // Write one row through a pool whose sessions are bound to tenant 1 / organization 1.
    let pool = connect_scoped_pool(&database_url, 1, 1).await;
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
    let space_id = i64::try_from(created.id).expect("space id fits i64");

    // A session without `app.current_tenant_id`/`app.current_organization_id` must see
    // zero rows (FORCE ROW LEVEL SECURITY policies fail closed on missing variables). The
    // probe URL is stripped of every `options` parameter so it truly carries no session scope.
    let unscoped_url = postgres_url_without_session_scope(&database_url);
    let unscoped = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(&unscoped_url)
        .await
        .expect("connect unscoped pool");
    let visible: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kb_space WHERE id = $1")
        .bind(space_id)
        .fetch_one(&unscoped)
        .await
        .expect("count rows without scope");
    assert_eq!(
        visible, 0,
        "RLS must hide rows when session scope variables are absent"
    );

    // A session bound to a different organization must also see zero rows.
    let other_org_url = postgres_url_without_session_scope(&database_url);
    let other_org = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(&other_org_url)
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
        .bind(space_id)
        .fetch_one(&other_org)
        .await
        .expect("count rows in other organization");
    assert_eq!(
        visible_other, 0,
        "RLS must hide rows for a mismatched organization scope"
    );

    // Cleanup runs on the tenant 1 / organization 1 pool so the RLS policy permits the delete.
    let deleted = sqlx::query("DELETE FROM kb_space WHERE id = $1")
        .bind(space_id)
        .execute(&pool)
        .await
        .expect("cleanup RLS fixture under tenant 1");
    assert_eq!(
        deleted.rows_affected(),
        1,
        "the scoped cleanup delete must remove exactly the fixture row"
    );
}

/// End-to-end proof of the product's core claim on a real PostgreSQL database:
/// a Drive document is ingested through the production pipeline into `kb_chunk`,
/// then hybrid/keyword retrieval returns it.
#[tokio::test]
async fn postgres_ingest_to_retrieval_round_trip_when_database_url_configured() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping postgres ingest-to-retrieval test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };

    use sdkwork_intelligence_knowledgebase_repository_sqlx::PostgresKnowledgeChunkRetrievalStore;
    use sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineService;
    use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::DriveImportJobLinkage;
    use sdkwork_intelligence_knowledgebase_service::retrieval::KnowledgeRetrievalService;
    use sdkwork_knowledgebase_contract::ingest::KnowledgeDriveImportResult;
    use sdkwork_knowledgebase_contract::rag::{
        KnowledgeRetrievalBinding, KnowledgeRetrievalMethod, KnowledgeRetrievalRequest,
    };
    use sdkwork_knowledgebase_contract::source::{KnowledgeSource, KnowledgeSourceType};
    use sdkwork_knowledgebase_test_support::fake_drive::FakeKnowledgeDriveStorage;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos() as u64;
    let tenant_id = 100_001_u64;
    // The pool sessions carry the deployment-bound RLS tenant, so the E2E uses the same tenant
    // and organization as the other optional Postgres probes in this file.
    let pool = connect_scoped_pool(&database_url, tenant_id, 0).await;
    let spaces = PostgresKnowledgeSpaceStore::new(pool.clone(), tenant_id, 0);
    let space = spaces
        .create_space(CreateKnowledgeSpaceRecord {
            name: format!("postgres e2e space {nonce}"),
            description: Some("ingest-to-retrieval e2e".to_string()),
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::Rag,
        })
        .await
        .expect("create e2e space");

    let drive = FakeKnowledgeDriveStorage::default();
    let stored = drive
        .put_text(
            "raw/documents/policy/original/policy.md",
            "original_document",
            "# Refund policy\n\nRefunds are accepted within 30 days of purchase.\n",
        )
        .await
        .expect("store document payload");

    let object_refs = PostgresKnowledgeDriveObjectRefStore::new(pool.clone(), tenant_id, 0);
    let object_ref = object_refs
        .create_object_ref(CreateKnowledgeDriveObjectRefRecord {
            space_id: space.id,
            drive_space_id: Some("drive-space-e2e".to_string()),
            drive_node_id: Some(format!("node-e2e-{nonce}")),
            logical_path: Some(stored.logical_path.clone()),
            drive_provider_kind: SDKWORK_DRIVE_PROVIDER_KIND.to_string(),
            drive_storage_provider_id: stored.storage_provider_id.clone(),
            drive_bucket: stored.bucket.clone(),
            drive_object_key: stored.object_key.clone(),
            drive_object_version: None,
            drive_etag: None,
            content_type: Some("text/markdown".to_string()),
            size_bytes: stored.size_bytes,
            checksum_sha256_hex: stored.checksum_sha256_hex.clone(),
            object_role: "original_document".to_string(),
            access_mode: MANAGED_DRIVE_ACCESS_MODE.to_string(),
        })
        .await
        .expect("create drive object ref");

    let documents = PostgresKnowledgeDocumentStore::new(pool.clone(), tenant_id, 0);
    let document = documents
        .create_document(CreateKnowledgeDocumentRecord {
            space_id: space.id,
            source_id: None,
            identity_scope: KnowledgeDocumentIdentityScope::SourceAndOriginalDriveNode,
            original_file_drive_node_id: object_ref.drive_node_id.clone(),
            title: "Refund policy".to_string(),
            mime_type: Some("text/markdown".to_string()),
            language: Some("en".to_string()),
        })
        .await
        .expect("create e2e document");

    let versions = PostgresKnowledgeDocumentVersionStore::new(pool.clone(), tenant_id, 0);
    let version = versions
        .create_document_version(CreateKnowledgeDocumentVersionRecord {
            document_id: document.id,
            version_no: 1,
            original_object_ref_id: object_ref.id,
            checksum_sha256_hex: object_ref.checksum_sha256_hex.clone(),
            size_bytes: object_ref.size_bytes,
            mime_type: Some("text/markdown".to_string()),
        })
        .await
        .expect("create e2e document version");

    let jobs = PostgresIngestionJobStore::new(pool.clone(), tenant_id, 0)
        .with_database_engine(DatabaseEngine::Postgres);
    let created = jobs
        .create_or_get_job(CreateIngestionJobRecord {
            space_id: space.id,
            source_type: KnowledgeSourceType::DriveObject.as_str().to_string(),
            idempotency_key: format!("postgres-e2e-{nonce}"),
            idempotency_fingerprint_sha256_hex: None,
        })
        .await
        .expect("create ingestion job");
    jobs.attach_drive_import_linkage(
        created.job.id,
        DriveImportJobLinkage {
            source_id: 1,
            document_id: document.id,
            document_version_id: version.id,
            original_object_ref: object_ref.clone(),
        },
    )
    .await
    .expect("attach drive import linkage");

    let import = KnowledgeDriveImportResult {
        source: KnowledgeSource {
            id: 1,
            space_id: space.id,
            source_type: KnowledgeSourceType::DriveObject,
            provider: Some("sdkwork-drive".to_string()),
            drive_bucket: Some(object_ref.drive_bucket.clone()),
            drive_prefix: Some(object_ref.drive_object_key.clone()),
            connector_metadata_json: None,
        },
        document,
        version,
        original_object_ref: object_ref,
        job: created.job.clone(),
    };
    let pipeline = KnowledgeDriveImportPipelineService::new(&drive, &jobs);
    let pipeline_result = pipeline
        .process_import_result(&import)
        .await
        .expect("ingest drive document");
    assert!(
        pipeline_result.index_result.is_some(),
        "ingestion must produce an index result"
    );

    let chunk_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM kb_chunk WHERE tenant_id = $1 AND space_id = $2")
            .bind(tenant_id as i64)
            .bind(space.id as i64)
            .fetch_one(&pool)
            .await
            .expect("count ingested chunks");
    assert!(chunk_count > 0, "ingestion must persist chunks");

    let retrieval_pg_pool = connect_scoped_pg_pool(&database_url, tenant_id, 0).await;
    let retrieval_store =
        PostgresKnowledgeChunkRetrievalStore::new(retrieval_pg_pool, tenant_id, 0)
            .with_database_engine(DatabaseEngine::Postgres);
    {
        use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_backend::{
            KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend,
        };
        let direct = retrieval_store
            .search_chunks(KnowledgeChunkSearchRequest {
                tenant_id,
                query: "refund policy".to_string(),
                binding: KnowledgeRetrievalBinding {
                    space_id: space.id,
                    source_filter: None,
                    document_filter: None,
                    priority: 0,
                    top_k: Some(5),
                    min_score: None,
                },
                method: KnowledgeRetrievalMethod::FullText,
                top_k: 5,
                query_embedding: None,
            })
            .await
            .unwrap_or_else(|error| panic!("direct keyword search failed: {error:?}"));
        assert!(!direct.is_empty());
    }
    let retrieval = KnowledgeRetrievalService::new(&retrieval_store, &retrieval_store);
    let result = retrieval
        .retrieve(KnowledgeRetrievalRequest {
            tenant_id,
            actor_id: None,
            query: "refund policy".to_string(),
            retrieval_profile_id: None,
            bindings: vec![KnowledgeRetrievalBinding {
                space_id: space.id,
                source_filter: None,
                document_filter: None,
                priority: 0,
                top_k: Some(5),
                min_score: None,
            }],
            methods: vec![KnowledgeRetrievalMethod::FullText],
            top_k: Some(5),
            include_citations: true,
            include_trace: false,
            context_budget_tokens: None,
            metadata: Vec::new(),
        })
        .await
        .expect("retrieve ingested knowledge");

    assert!(
        !result.hits.is_empty(),
        "retrieval must return the ingested document"
    );
    assert!(
        result
            .hits
            .iter()
            .any(|hit| hit.content.contains("30 days")),
        "retrieved context must contain the ingested text"
    );

    // Vector path: write embeddings for the ingested chunks and prove the
    // pgvector ANN backend returns them. This guards the embedding write and
    // the vector/hybrid retrieval contract, which the keyword assertions above
    // cannot cover.
    {
        use sdkwork_intelligence_knowledgebase_repository_sqlx::{
            PgVectorKnowledgeRetrievalBackend, PostgresKnowledgeEmbeddingStore,
        };
        use sdkwork_intelligence_knowledgebase_service::ports::knowledge_embedding_store::ChunkEmbeddingUpsertRequest;
        use sdkwork_intelligence_knowledgebase_service::ports::knowledge_retrieval_backend::{
            KnowledgeChunkSearchRequest, KnowledgeRetrievalBackend,
        };

        use sdkwork_knowledgebase_contract::rag::KnowledgeRetrievalMethod;

        let chunk_ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM kb_chunk WHERE tenant_id = $1 AND space_id = $2 ORDER BY id LIMIT 4",
        )
        .bind(tenant_id as i64)
        .bind(space.id as i64)
        .fetch_all(&pool)
        .await
        .expect("load ingested chunk ids");
        assert!(!chunk_ids.is_empty(), "vector fixtures require chunks");

        let vector_pg_pool = connect_scoped_pg_pool(&database_url, tenant_id, 0).await;
        // The embedding row is FK-bound to a knowledge index; create the index
        // the ingestion pipeline would normally provision.
        let index_id = 930_000_000_i64 + (nonce % 10_000_000) as i64;
        sqlx::query(
            r#"
            INSERT INTO kb_index (
                id, uuid, tenant_id, organization_id, space_id, index_kind,
                schema_version, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, 0, $4, 'rag', '1', 1, now(), now())
            "#,
        )
        .bind(index_id)
        .bind(format!("index-{nonce}"))
        .bind(tenant_id as i64)
        .bind(space.id as i64)
        .execute(&pool)
        .await
        .expect("create knowledge index for embeddings");

        let embeddings = PostgresKnowledgeEmbeddingStore::new(
            vector_pg_pool.clone(),
            tenant_id,
            0,
            DatabaseEngine::Postgres,
        );
        let mut query_embedding = vec![0.0_f32; 1536];
        query_embedding[0] = 1.0;
        for chunk_id in &chunk_ids {
            embeddings
                .upsert_chunk_embedding(ChunkEmbeddingUpsertRequest {
                    tenant_id,
                    index_id: u64::try_from(index_id).expect("index id fits u64"),
                    chunk_id: u64::try_from(*chunk_id).expect("chunk id fits u64"),
                    vector: query_embedding.clone(),
                    provider_id: Some("provider.test.embedding".to_string()),
                    model: Some("test-embedding".to_string()),
                })
                .await
                .expect("upsert chunk embedding");
        }

        let vector_backend = PgVectorKnowledgeRetrievalBackend::new(vector_pg_pool, tenant_id, 0);
        let vector_hits = vector_backend
            .search_chunks(KnowledgeChunkSearchRequest {
                tenant_id,
                query: "refund policy".to_string(),
                binding: KnowledgeRetrievalBinding {
                    space_id: space.id,
                    source_filter: None,
                    document_filter: None,
                    priority: 0,
                    top_k: Some(5),
                    min_score: None,
                },
                method: KnowledgeRetrievalMethod::Vector,
                top_k: 5,
                query_embedding: Some(query_embedding),
            })
            .await
            .expect("pgvector search");
        assert!(
            !vector_hits.is_empty(),
            "vector retrieval must return embedded chunks"
        );
        assert!(
            vector_hits
                .iter()
                .any(|hit| hit.content.contains("30 days")),
            "vector retrieval must return the ingested text"
        );
    }
}
