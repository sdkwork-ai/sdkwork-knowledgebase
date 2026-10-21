mod support;

static WIKI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wiki_test_guard() -> std::sync::MutexGuard<'static, ()> {
    WIKI_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

use sdkwork_intelligence_knowledgebase_repository_sqlx::SqlxWikiPersistenceStore;
use sdkwork_intelligence_knowledgebase_service::ports::{
    knowledge_wiki_persistence::WikiPersistenceScope,
    knowledge_wiki_public_provider::{
        ListWikiPublicNavigationRequest, SearchWikiPublicPagesRequest, WikiPublicProviderStore,
    },
};

const SCOPE: WikiPersistenceScope = WikiPersistenceScope {
    tenant_id: 101,
    organization_id: 202,
};
const PUBLICATION_UUID: &str = "11111111-1111-4111-8111-111111111501";

#[tokio::test]
async fn active_publication_and_route_resolution_are_scope_isolated_and_non_disclosing() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_public_wiki(&pool).await;

    let publication = store
        .get_active_publication_by_uuid(SCOPE, PUBLICATION_UUID)
        .await
        .expect("retrieve active publication")
        .expect("active publication");
    assert_eq!(publication.id, 501);
    assert_eq!(publication.supported_locales, ["zh-CN"]);
    assert!(publication.search_enabled);
    assert!(!publication.sitemap_enabled);

    let wrong_scope = WikiPersistenceScope {
        tenant_id: SCOPE.tenant_id,
        organization_id: SCOPE.organization_id + 1,
    };
    assert!(store
        .get_active_publication_by_uuid(wrong_scope, PUBLICATION_UUID)
        .await
        .expect("wrong scope lookup")
        .is_none());

    let exact = store
        .resolve_public_route(SCOPE, 501, "/a/")
        .await
        .expect("resolve exact route")
        .expect("exact public page");
    assert!(!exact.matched_previous_route);
    assert_eq!(exact.page.uuid, projection_uuid(601));

    let redirect = store
        .resolve_public_route(SCOPE, 501, "/old-b/")
        .await
        .expect("resolve reviewed redirect")
        .expect("redirect page");
    assert!(redirect.matched_previous_route);
    assert_eq!(redirect.redirect_status, Some(308));
    assert_eq!(redirect.page.canonical_route, "/b/");

    assert!(store
        .resolve_public_route(SCOPE, 501, "/private/")
        .await
        .expect("private lookup")
        .is_none());

    sqlx::query("UPDATE kb_site_publication SET wiki_status = 'PAUSED' WHERE id = 501")
        .execute(&pool)
        .await
        .expect("pause publication");
    assert!(store
        .resolve_public_route(SCOPE, 501, "/a/")
        .await
        .expect("paused lookup")
        .is_none());
}

#[tokio::test]
async fn content_lookup_revalidates_exact_public_version() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_public_wiki(&pool).await;
    let projection = store
        .get_public_content_projection(SCOPE, 501, &projection_uuid(601), 1)
        .await
        .expect("retrieve pinned projection")
        .expect("current public projection");
    assert_eq!(
        projection.public_drive_version_uuid,
        drive_version_uuid(601)
    );
    assert_eq!(projection.page_public_version, 1);
    assert_eq!(projection.public_updated_at, "2026-07-21T00:00:00.000000Z");

    assert!(store
        .get_public_content_projection(SCOPE, 501, &projection_uuid(601), 2)
        .await
        .expect("stale version lookup")
        .is_none());
    sqlx::query(
        "UPDATE kb_source_file_projection SET publication_state = 'UNPUBLISHED', visibility = 'PRIVATE' WHERE id = 601",
    )
    .execute(&pool)
    .await
    .expect("revoke page");
    assert!(store
        .get_public_content_projection(SCOPE, 501, &projection_uuid(601), 1)
        .await
        .expect("revoked content lookup")
        .is_none());
}

#[tokio::test]
async fn navigation_and_search_use_bounded_keyset_windows_and_public_filters() {
    let _guard = wiki_test_guard();
    let Some((pool, store)) = test_store().await else {
        eprintln!("skipping wiki postgres test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL");
        return;
    };
    seed_public_wiki(&pool).await;

    let first = store
        .list_public_navigation(ListWikiPublicNavigationRequest {
            scope: SCOPE,
            publication_id: 501,
            locale: Some("zh-CN".to_string()),
            after: None,
            limit: 1,
        })
        .await
        .expect("first navigation page");
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].canonical_route, "/a/");
    let second = store
        .list_public_navigation(ListWikiPublicNavigationRequest {
            scope: SCOPE,
            publication_id: 501,
            locale: Some("zh-CN".to_string()),
            after: first.next,
            limit: 1,
        })
        .await
        .expect("second navigation page");
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].canonical_route, "/b/");
    assert!(second.next.is_none());

    let search = store
        .search_public_pages(SearchWikiPublicPagesRequest {
            scope: SCOPE,
            publication_id: 501,
            query: "guide".to_string(),
            locale: Some("zh-CN".to_string()),
            after: None,
            limit: 20,
        })
        .await
        .expect("search public projections");
    assert_eq!(
        search
            .items
            .iter()
            .map(|item| item.canonical_route.as_str())
            .collect::<Vec<_>>(),
        ["/a/", "/b/", "/hidden/"]
    );
    assert!(store
        .search_public_pages(SearchWikiPublicPagesRequest {
            scope: SCOPE,
            publication_id: 501,
            query: "%".to_string(),
            locale: None,
            after: None,
            limit: 20,
        })
        .await
        .expect("literal wildcard search")
        .items
        .is_empty());
}

async fn test_store() -> Option<(sqlx::AnyPool, SqlxWikiPersistenceStore)> {
    let pool = support::scoped_pool(101, 202).await?;
    support::clean_wiki_scope(&pool, 101, 202).await;
    Some((pool.clone(), SqlxWikiPersistenceStore::new(pool)))
}

async fn seed_public_wiki(pool: &sqlx::AnyPool) {
    sqlx::query(
        r#"
        INSERT INTO kb_space (
            id, uuid, tenant_id, organization_id, name, drive_space_id,
            status, created_at, updated_at
        ) VALUES (501, $1, 101, 202, 'Wiki Space', $2, 1, $3::timestamptz, $3::timestamptz)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind("11111111-1111-4111-8111-111111111401")
    .bind("11111111-1111-4111-8111-111111111402")
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert space");
    sqlx::query(
        r#"
        INSERT INTO kb_site_publication (
            id, uuid, tenant_id, organization_id, space_id, drive_space_uuid,
            source_root_node_uuid, source_scope_uuid, wiki_status, title,
            supported_locales_json, created_by, updated_by, created_at, updated_at
        ) VALUES (
            501, $1, 101, 202, 501, $2, $3, $4, 'ACTIVE', 'SDKWork Wiki',
            '["zh-CN"]', 9001, 9001, $5::timestamptz, $5::timestamptz
        )
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(PUBLICATION_UUID)
    .bind("11111111-1111-4111-8111-111111111402")
    .bind("11111111-1111-4111-8111-111111111403")
    .bind("11111111-1111-4111-8111-111111111404")
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert active publication");
    insert_projection(pool, 601, "/a/", "Alpha Guide", "PUBLIC", false, "READY").await;
    insert_projection(pool, 602, "/b/", "Beta Guide", "PUBLIC", false, "READY").await;
    insert_projection(
        pool,
        603,
        "/hidden/",
        "Hidden Guide",
        "PUBLIC",
        true,
        "READY",
    )
    .await;
    insert_projection(
        pool,
        604,
        "/unlisted/",
        "Unlisted Guide",
        "UNLISTED",
        false,
        "READY",
    )
    .await;
    sqlx::query(
        r#"
        UPDATE kb_source_file_projection
        SET previous_canonical_route = '/old-b/', redirect_status = 308
        WHERE id = 602
        "#,
    )
    .execute(pool)
    .await
    .expect("add reviewed redirect");
}

async fn insert_projection(
    pool: &sqlx::AnyPool,
    id: u64,
    route: &str,
    title: &str,
    visibility: &str,
    nav_hidden: bool,
    index_state: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO kb_source_file_projection (
            id, uuid, tenant_id, organization_id, site_publication_id, space_id,
            drive_space_uuid, drive_node_uuid, drive_version_uuid, source_path,
            canonical_route, file_kind, media_type, size_bytes, content_sha256,
            source_state, publication_state, visibility, index_state, title, locale,
            nav_hidden, public_drive_version_uuid, page_public_version,
            created_by, updated_by, created_at, updated_at
        ) VALUES (
            $1, $2, 101, 202, 501, 501,
            $3, $4, $5, $6,
            $7, 'PAGE', 'text/markdown', 6,
            'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            'READY', 'PUBLISHED', $8, $9, $10, 'zh-CN',
            $11, $5, 1, 9001, 9001, $12::timestamptz, $12::timestamptz
        )
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(i64::try_from(id).expect("projection id"))
    .bind(projection_uuid(id))
    .bind("11111111-1111-4111-8111-111111111402")
    .bind(format!("drive-node-{id}"))
    .bind(drive_version_uuid(id))
    .bind(format!("page-{id}.md"))
    .bind(route)
    .bind(visibility)
    .bind(index_state)
    .bind(title)
    .bind(nav_hidden)
    .bind("2026-07-21T00:00:00Z")
    .execute(pool)
    .await
    .expect("insert public projection");
}

fn projection_uuid(id: u64) -> String {
    format!("11111111-1111-4111-8111-{id:012}")
}

fn drive_version_uuid(id: u64) -> String {
    format!("22222222-2222-4222-8222-{id:012}")
}
