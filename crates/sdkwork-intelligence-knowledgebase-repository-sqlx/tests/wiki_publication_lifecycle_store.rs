mod support;

static WIKI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wiki_test_guard() -> std::sync::MutexGuard<'static, ()> {
    WIKI_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    KnowledgeIdGenerator, KnowledgeIdGeneratorError, SqlxWikiPersistenceStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::{
    knowledge_wiki_persistence::{
        WikiPagePublicationState, WikiPersistenceError, WikiPersistenceScope,
        WikiPublicationStatus, WikiVisibility,
    },
    knowledge_wiki_publication_lifecycle::{
        ChangeWikiPageVisibilityRequest, ChangeWikiPublicationStatusRequest,
        PublishWikiPageRequest, UnpublishWikiPageRequest, WikiLifecycleAuditContext,
        WikiLifecycleDisposition, WikiPublicationLifecycleAction, WikiPublicationLifecycleStore,
    },
};

const SCOPE: WikiPersistenceScope = WikiPersistenceScope {
    tenant_id: 101,
    organization_id: 202,
};
const SPACE_ID: u64 = 501;
const PUBLICATION_ID: u64 = 601;
const PAGE_ID: u64 = 701;
const PAGE_UUID: &str = "11111111-1111-4111-8111-111111111701";

fn audit_context(request_id: &str) -> WikiLifecycleAuditContext {
    WikiLifecycleAuditContext {
        request_id: request_id.to_string(),
        trace_id: Some(format!("trace-{request_id}")),
    }
}

#[tokio::test]
async fn activation_and_pause_are_optimistic_and_emit_provider_events() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_wiki(&pool, "READY", "DRAFT", "PRIVATE", "READY").await;

    let activated = store
        .change_publication_status(ChangeWikiPublicationStatusRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            expected_version: 0,
            actor_id: 9001,
            action: WikiPublicationLifecycleAction::Activate,
            audit: audit_context("activate-1"),
        })
        .await
        .expect("activate Wiki publication");
    assert_eq!(activated.disposition, WikiLifecycleDisposition::Changed);
    assert_eq!(
        activated.publication.wiki_status,
        WikiPublicationStatus::Active
    );
    assert_eq!(activated.publication.provider_generation, 2);
    assert_eq!(activated.publication.version, 1);

    let stale = store
        .change_publication_status(ChangeWikiPublicationStatusRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            expected_version: 0,
            actor_id: 9001,
            action: WikiPublicationLifecycleAction::Pause,
            audit: audit_context("pause-stale"),
        })
        .await
        .expect_err("stale pause must fail");
    assert!(matches!(stale, WikiPersistenceError::StaleVersion { .. }));

    let paused = store
        .change_publication_status(ChangeWikiPublicationStatusRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            expected_version: 1,
            actor_id: 9002,
            action: WikiPublicationLifecycleAction::Pause,
            audit: audit_context("pause-1"),
        })
        .await
        .expect("pause Wiki publication");
    assert_eq!(
        paused.publication.wiki_status,
        WikiPublicationStatus::Paused
    );
    assert_eq!(paused.publication.provider_generation, 3);
    assert_eq!(paused.publication.version, 2);

    let events = outbox_events(&pool).await;
    assert_eq!(
        events
            .iter()
            .map(|event| event.0.as_str())
            .collect::<Vec<_>>(),
        [
            "knowledgebase.wiki.provider.changed.v1",
            "knowledgebase.wiki.provider.changed.v1"
        ]
    );
    assert_eq!(payload_field(&events[0].1, "data")["operation"], "ACTIVATE");
    assert_eq!(payload_field(&events[1].1, "data")["operation"], "PAUSE");
    assert!(!events[1].1.contains("actorId"));
    assert!(!events[1].1.contains("objectKey"));
    assert_provider_event_envelopes(&pool).await;

    let audits = audit_events(&pool).await;
    assert_eq!(audits.len(), 2);
    assert_eq!(audits[0].0, "knowledge.wiki.publication.activated");
    assert_eq!(audits[0].1, "9001");
    assert_eq!(audits[0].2, "activate-1");
    assert_eq!(payload_field(&audits[0].3, "disposition"), "CHANGED");
    assert_eq!(audits[1].0, "knowledge.wiki.publication.paused");
    assert_eq!(audits[1].1, "9002");
    assert_eq!(audits[1].2, "pause-1");
    assert!(!audits[1].3.contains("objectKey"));
}

#[tokio::test]
async fn publish_visibility_and_unpublish_advance_only_the_required_generations() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_wiki(&pool, "ACTIVE", "DRAFT", "PRIVATE", "READY").await;

    let published = store
        .publish_page(PublishWikiPageRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            visibility: WikiVisibility::Public,
            expected_publication_version: 0,
            expected_page_version: 0,
            actor_id: 9001,
            audit: audit_context("publish-1"),
        })
        .await
        .expect("publish Wiki page");
    assert_eq!(published.disposition, WikiLifecycleDisposition::Changed);
    assert_eq!(
        published.page.publication_state,
        WikiPagePublicationState::Published
    );
    assert_eq!(published.page.visibility, WikiVisibility::Public);
    assert_eq!(
        published.page.public_drive_version_uuid.as_deref(),
        Some("drive-version-701")
    );
    assert_eq!(published.page.page_public_version, 1);
    assert_eq!(published.page.version, 1);
    assert_eq!(published.publication.provider_generation, 1);
    assert_eq!(published.publication.navigation_generation, 2);
    assert_eq!(published.publication.search_generation, 2);
    assert_eq!(published.publication.version, 1);

    let replay = store
        .publish_page(PublishWikiPageRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            visibility: WikiVisibility::Public,
            expected_publication_version: 1,
            expected_page_version: 1,
            actor_id: 9001,
            audit: audit_context("publish-replay"),
        })
        .await
        .expect("idempotent publish replay");
    assert_eq!(replay.disposition, WikiLifecycleDisposition::Unchanged);

    let unlisted = store
        .change_page_visibility(ChangeWikiPageVisibilityRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            visibility: WikiVisibility::Unlisted,
            expected_publication_version: 1,
            expected_page_version: 1,
            actor_id: 9002,
            audit: audit_context("visibility-1"),
        })
        .await
        .expect("make Wiki page unlisted");
    assert_eq!(unlisted.page.visibility, WikiVisibility::Unlisted);
    assert_eq!(unlisted.page.page_public_version, 2);
    assert_eq!(unlisted.publication.provider_generation, 1);
    assert_eq!(unlisted.publication.navigation_generation, 3);
    assert_eq!(unlisted.publication.search_generation, 3);
    assert_eq!(unlisted.publication.version, 2);

    let unpublished = store
        .unpublish_page(UnpublishWikiPageRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            expected_publication_version: 2,
            expected_page_version: 2,
            actor_id: 9003,
            audit: audit_context("unpublish-1"),
        })
        .await
        .expect("unpublish Wiki page");
    assert_eq!(
        unpublished.page.publication_state,
        WikiPagePublicationState::Unpublished
    );
    assert_eq!(unpublished.page.visibility, WikiVisibility::Private);
    assert!(unpublished.page.public_drive_version_uuid.is_none());
    assert_eq!(unpublished.page.page_public_version, 3);
    assert_eq!(unpublished.publication.navigation_generation, 3);
    assert_eq!(unpublished.publication.search_generation, 3);
    assert_eq!(unpublished.publication.version, 2);

    let event_types = outbox_events(&pool)
        .await
        .into_iter()
        .map(|event| event.0)
        .collect::<Vec<_>>();
    assert_eq!(
        event_types,
        [
            "knowledgebase.wiki.route.changed.v1",
            "knowledgebase.wiki.navigation.changed.v1",
            "knowledgebase.wiki.search.changed.v1",
            "knowledgebase.wiki.route.changed.v1",
            "knowledgebase.wiki.navigation.changed.v1",
            "knowledgebase.wiki.search.changed.v1",
            "knowledgebase.wiki.route.revoked.v1",
        ]
    );
    assert_provider_event_envelopes(&pool).await;

    let audits = audit_events(&pool).await;
    assert_eq!(audits.len(), 4);
    assert_eq!(audits[0].0, "knowledge.wiki.source_file.published");
    assert_eq!(audits[1].0, "knowledge.wiki.source_file.published");
    assert_eq!(payload_field(&audits[1].3, "disposition"), "UNCHANGED");
    assert_eq!(audits[2].0, "knowledge.wiki.source_file.visibility_changed");
    assert_eq!(audits[3].0, "knowledge.wiki.source_file.unpublished");
}

#[tokio::test]
async fn invalid_private_publish_cross_scope_and_stale_commands_leave_state_unchanged() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_wiki(&pool, "ACTIVE", "DRAFT", "PRIVATE", "READY").await;

    let private = store
        .publish_page(PublishWikiPageRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            visibility: WikiVisibility::Private,
            expected_publication_version: 0,
            expected_page_version: 0,
            actor_id: 9001,
            audit: audit_context("publish-private"),
        })
        .await
        .expect_err("private publish must fail");
    assert!(matches!(private, WikiPersistenceError::InvalidRequest(_)));

    let wrong_scope = WikiPersistenceScope {
        tenant_id: SCOPE.tenant_id,
        organization_id: SCOPE.organization_id + 1,
    };
    let isolated = store
        .change_page_visibility(ChangeWikiPageVisibilityRequest {
            scope: wrong_scope,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            visibility: WikiVisibility::Public,
            expected_publication_version: 0,
            expected_page_version: 0,
            actor_id: 9001,
            audit: audit_context("visibility-cross-scope"),
        })
        .await
        .expect_err("cross-scope command must not resolve publication");
    assert!(matches!(isolated, WikiPersistenceError::NotFound { .. }));

    let stale = store
        .change_page_visibility(ChangeWikiPageVisibilityRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            source_file_uuid: PAGE_UUID.to_string(),
            visibility: WikiVisibility::Public,
            expected_publication_version: 1,
            expected_page_version: 0,
            actor_id: 9001,
            audit: audit_context("visibility-stale"),
        })
        .await
        .expect_err("stale publication fence must fail");
    assert!(matches!(stale, WikiPersistenceError::StaleVersion { .. }));

    let state: (String, String, i64, i64) = sqlx::query_as(
        "SELECT publication_state, visibility, page_public_version, version FROM kb_source_file_projection WHERE id = $1",
    )
    .bind(i64::try_from(PAGE_ID).expect("page id"))
    .fetch_one(&pool)
    .await
    .expect("read unchanged page");
    assert_eq!(state, ("DRAFT".to_string(), "PRIVATE".to_string(), 0, 0));
    assert!(outbox_events(&pool).await.is_empty());
    assert!(audit_events(&pool).await.is_empty());
}

/// Destructive: renames a shared table, so it requires exclusive access to the PostgreSQL
/// schema. The default suite shares one schema across parallel test binaries; run this test
/// with `--ignored` against an isolated database.
#[tokio::test]
#[ignore = "destructive: requires exclusive PostgreSQL schema access"]
async fn audit_persistence_failure_rolls_back_publication_and_outbox() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_wiki(&pool, "READY", "DRAFT", "PRIVATE", "READY").await;
    sqlx::query("ALTER TABLE kb_audit_event RENAME TO kb_audit_event_audit_failure")
        .execute(&pool)
        .await
        .expect("rename audit table to simulate persistence failure");

    let error = store
        .change_publication_status(ChangeWikiPublicationStatusRequest {
            scope: SCOPE,
            space_id: SPACE_ID,
            expected_version: 0,
            actor_id: 9001,
            action: WikiPublicationLifecycleAction::Activate,
            audit: audit_context("activate-audit-failure"),
        })
        .await
        .expect_err("audit failure must fail the lifecycle transaction");
    assert!(matches!(error, WikiPersistenceError::Internal(_)));

    let state: (String, i64) =
        sqlx::query_as("SELECT wiki_status, version FROM kb_site_publication WHERE id = $1")
            .bind(i64::try_from(PUBLICATION_ID).expect("publication id"))
            .fetch_one(&pool)
            .await
            .expect("read rolled back publication");
    assert_eq!(state, ("READY".to_string(), 0));
    assert!(outbox_events(&pool).await.is_empty());

    sqlx::query("ALTER TABLE kb_audit_event_audit_failure RENAME TO kb_audit_event")
        .execute(&pool)
        .await
        .expect("restore audit table");
}

async fn test_store() -> Option<(sqlx::AnyPool, SqlxWikiPersistenceStore)> {
    let pool = support::scoped_pool(101, 202).await?;
    support::clean_wiki_scope(&pool, 101, 202).await;
    let generator = Arc::new(TestIdGenerator::new(10_000));
    let store = SqlxWikiPersistenceStore::with_id_generator(pool.clone(), generator);
    Some((pool, store))
}

async fn seed_wiki(
    pool: &sqlx::AnyPool,
    wiki_status: &str,
    publication_state: &str,
    visibility: &str,
    source_state: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO kb_space (
            id, uuid, tenant_id, organization_id, name, drive_space_id,
            status, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, 'Lifecycle Wiki', $5, 1, $6::timestamptz, $6::timestamptz)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(i64::try_from(SPACE_ID).expect("space id"))
    .bind("11111111-1111-4111-8111-111111111501")
    .bind(i64::try_from(SCOPE.tenant_id).expect("tenant id"))
    .bind(i64::try_from(SCOPE.organization_id).expect("organization id"))
    .bind("drive-space-501")
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert knowledge space");
    sqlx::query(
        r#"
        INSERT INTO kb_site_publication (
            id, uuid, tenant_id, organization_id, space_id, drive_space_uuid,
            source_root_node_uuid, source_scope_uuid, wiki_status, title,
            created_by, updated_by, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Lifecycle Wiki', 9001, 9001, $10::timestamptz, $10::timestamptz)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(i64::try_from(PUBLICATION_ID).expect("publication id"))
    .bind("11111111-1111-4111-8111-111111111601")
    .bind(i64::try_from(SCOPE.tenant_id).expect("tenant id"))
    .bind(i64::try_from(SCOPE.organization_id).expect("organization id"))
    .bind(i64::try_from(SPACE_ID).expect("space id"))
    .bind("drive-space-501")
    .bind("raw-root-node")
    .bind("raw-root-scope")
    .bind(wiki_status)
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert Wiki publication");
    let published = publication_state == "PUBLISHED";
    sqlx::query(
        r#"
        INSERT INTO kb_source_file_projection (
            id, uuid, tenant_id, organization_id, site_publication_id, space_id,
            drive_space_uuid, drive_node_uuid, drive_version_uuid, source_path,
            canonical_route, file_kind, media_type, size_bytes, content_sha256,
            source_state, publication_state, visibility, index_state,
            public_drive_version_uuid, page_public_version,
            created_by, updated_by, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, 'drive-space-501', 'drive-node-701',
            'drive-version-701', 'guide.md', '/guide/', 'PAGE', 'text/markdown', 128,
            'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            $7, $8, $9, 'READY', $10, $11, 9001, 9001, $12::timestamptz, $12::timestamptz
        )
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(i64::try_from(PAGE_ID).expect("page id"))
    .bind(PAGE_UUID)
    .bind(i64::try_from(SCOPE.tenant_id).expect("tenant id"))
    .bind(i64::try_from(SCOPE.organization_id).expect("organization id"))
    .bind(i64::try_from(PUBLICATION_ID).expect("publication id"))
    .bind(i64::try_from(SPACE_ID).expect("space id"))
    .bind(source_state)
    .bind(publication_state)
    .bind(visibility)
    .bind(published.then_some("drive-version-701"))
    .bind(if published { 1_i64 } else { 0_i64 })
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert Wiki page");
}

async fn outbox_events(pool: &sqlx::AnyPool) -> Vec<(String, String)> {
    sqlx::query_as("SELECT event_type, CAST(payload AS TEXT) FROM kb_outbox_event ORDER BY id ASC")
        .fetch_all(pool)
        .await
        .expect("list lifecycle outbox events")
}

async fn assert_provider_event_envelopes(pool: &sqlx::AnyPool) {
    let events: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, uuid, event_type, CAST(payload AS TEXT) FROM kb_outbox_event ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .expect("list provider event envelopes");
    assert!(!events.is_empty());
    for (id, uuid, event_type, payload) in events {
        let payload: serde_json::Value =
            serde_json::from_str(&payload).expect("provider event payload JSON");
        assert_eq!(payload["id"], uuid);
        assert_eq!(payload["type"], event_type);
        assert_eq!(payload["source"], "sdkwork-knowledgebase");
        assert_eq!(payload["specversion"], "1.0");
        assert_eq!(payload["tenantId"], SCOPE.tenant_id.to_string());
        assert_eq!(payload["organizationId"], SCOPE.organization_id.to_string());
        assert_eq!(payload["sequenceNo"], id.to_string());
        assert!(payload["subject"]
            .as_str()
            .is_some_and(|value| value.starts_with("wiki-publication:")));
        assert!(payload["data"]["providerResourceUuid"].as_str().is_some());
        assert!(payload["data"]["providerGeneration"].as_str().is_some());
        assert!(payload["data"]["navigationGeneration"].as_str().is_some());
        assert!(payload["data"]["searchGeneration"].as_str().is_some());
        assert!(payload["data"]["operation"].as_str().is_some());
        assert!(payload.get("actorId").is_none());
        assert!(payload["data"].get("actorId").is_none());
    }
}

async fn audit_events(pool: &sqlx::AnyPool) -> Vec<(String, String, String, String)> {
    sqlx::query_as(
        "SELECT event_type, actor_id, request_id, CAST(payload AS TEXT) FROM kb_audit_event ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .expect("list lifecycle audit events")
}

/// Reads one field from a JSONB payload rendered as text (JSONB output includes spaces).
fn payload_field(payload: &str, field: &str) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(payload).expect("payload json")[field].clone()
}

struct TestIdGenerator {
    next: AtomicU64,
}

impl TestIdGenerator {
    fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }
}

impl fmt::Debug for TestIdGenerator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("TestIdGenerator").finish()
    }
}

impl KnowledgeIdGenerator for TestIdGenerator {
    fn next_id(&self) -> Result<u64, KnowledgeIdGeneratorError> {
        Ok(self.next.fetch_add(1, Ordering::Relaxed))
    }
}
