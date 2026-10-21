use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    is_postgres_database_url, PostgresGroupKnowledgeSpaceBindingStore, PostgresKnowledgeSpaceStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::{
    knowledge_group_space_binding_store::KnowledgeGroupSpaceBindingStore,
    knowledge_space_store::KnowledgeSpaceStore,
};
use sdkwork_routes_knowledgebase_app_api::{
    paths, KnowledgeAppRequestContext, KnowledgebaseRuntime,
};
use serde_json::json;
use sqlx::Row;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tower::util::ServiceExt;

const TENANT_ID: u64 = 1;
const ORGANIZATION_ID: u64 = 7001;
const DIRECT_DRIVE_OWNER_ID: u64 = 42;
const GROUP_OWNER_ID: u64 = 7;
const GROUP_ADMIN_ID: u64 = 8;
const GROUP_MEMBER_ID: u64 = 9;

struct EnvironmentGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvironmentGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn group_managed_spaces_do_not_fall_back_to_direct_drive_owner_permissions() {
    let _organization = EnvironmentGuard::set(
        "SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID",
        &ORGANIZATION_ID.to_string(),
    );
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    assert_eq!(runtime.organization_id(), ORGANIZATION_ID);
    let direct_owner = app_context(DIRECT_DRIVE_OWNER_ID);
    let space_id = create_space(&runtime, direct_owner.clone()).await;
    assert_persisted_space(&runtime, space_id).await;

    // The direct Drive owner has ordinary-space member access before the IM binding exists.
    assert_status(
        &runtime,
        direct_owner.clone(),
        Method::GET,
        format!("{}/{}", paths::SPACES, space_id),
        None,
        StatusCode::OK,
    )
    .await;
    assert_status(
        &runtime,
        direct_owner.clone(),
        Method::GET,
        paths::SPACE_MEMBERS.replace("{space_id}", &space_id.to_string()),
        None,
        StatusCode::OK,
    )
    .await;

    for (actor_id, role) in [
        (GROUP_OWNER_ID, "owner"),
        (GROUP_ADMIN_ID, "writer"),
        (GROUP_MEMBER_ID, "reader"),
    ] {
        assert_status(
            &runtime,
            direct_owner.clone(),
            Method::POST,
            paths::SPACE_MEMBERS.replace("{space_id}", &space_id.to_string()),
            Some(json!({
                "subjectType": "user",
                "subjectId": actor_id.to_string(),
                "role": role,
            })),
            StatusCode::OK,
        )
        .await;
    }

    seed_group_binding(&runtime, space_id).await;

    // A group owner can update KB-owned description metadata, but not IM-owned group name.
    assert_status(
        &runtime,
        app_context(GROUP_OWNER_ID),
        Method::PATCH,
        paths::SPACE.replace("{space_id}", &space_id.to_string()),
        Some(json!({ "description": "KB-owned group description" })),
        StatusCode::OK,
    )
    .await;
    assert_status(
        &runtime,
        app_context(GROUP_OWNER_ID),
        Method::PATCH,
        paths::SPACE.replace("{space_id}", &space_id.to_string()),
        Some(json!({ "name": "forbidden IM group rename" })),
        StatusCode::FORBIDDEN,
    )
    .await;

    // Group admins and ordinary members retain their content roles but cannot mutate space
    // metadata reserved for the IM group owner.
    for actor_id in [GROUP_ADMIN_ID, GROUP_MEMBER_ID] {
        assert_status(
            &runtime,
            app_context(actor_id),
            Method::PATCH,
            paths::SPACE.replace("{space_id}", &space_id.to_string()),
            Some(json!({ "description": "must be rejected" })),
            StatusCode::FORBIDDEN,
        )
        .await;
    }

    // The pre-existing direct Drive owner is not in the IM snapshot. Every generic route must
    // now fail before its permissive Drive grant can authorize a group-managed resource.
    for (method, uri, body) in [
        (
            Method::PATCH,
            paths::SPACE.replace("{space_id}", &space_id.to_string()),
            Some(json!({ "description": "direct drive owner must not update" })),
        ),
        (
            Method::DELETE,
            paths::SPACE.replace("{space_id}", &space_id.to_string()),
            None,
        ),
        (
            Method::GET,
            paths::SPACE_MEMBERS.replace("{space_id}", &space_id.to_string()),
            None,
        ),
        (
            Method::POST,
            paths::SPACE_MEMBERS.replace("{space_id}", &space_id.to_string()),
            Some(json!({
                "subjectType": "user",
                "subjectId": "99",
                "role": "reader",
            })),
        ),
        (
            Method::DELETE,
            format!(
                "{}?subjectType=user&subjectId=99",
                paths::SPACE_MEMBERS.replace("{space_id}", &space_id.to_string())
            ),
            None,
        ),
    ] {
        assert_status(
            &runtime,
            direct_owner.clone(),
            method,
            uri,
            body,
            StatusCode::FORBIDDEN,
        )
        .await;
    }

    let row = sqlx::query("SELECT name, description, status FROM kb_space WHERE id = $1")
        .bind(space_id as i64)
        .fetch_one(runtime.pool())
        .await
        .expect("group space row");
    assert_eq!(row.get::<String, _>("name"), "Direct Drive Owner Space");
    assert_eq!(
        row.get::<Option<String>, _>("description").as_deref(),
        Some("KB-owned group description")
    );
    assert_eq!(row.get::<i64, _>("status"), 1);
}

fn app_context(actor_id: u64) -> KnowledgeAppRequestContext {
    KnowledgeAppRequestContext {
        tenant_id: TENANT_ID,
        actor_id: Some(actor_id),
        organization_id: Some(ORGANIZATION_ID),
        session_id: None,
        request_id: format!("test-request-group-space-{actor_id}"),
        trace_id: None,
        idempotency_key: None,
    }
}

async fn create_space(runtime: &KnowledgebaseRuntime, context: KnowledgeAppRequestContext) -> u64 {
    let response = send(
        runtime,
        context,
        Method::POST,
        paths::SPACES.to_string(),
        Some(json!({
            "name": "Direct Drive Owner Space",
            "description": "ordinary before group binding",
            "ownerSubjectType": "user",
            "ownerSubjectId": DIRECT_DRIVE_OWNER_ID.to_string(),
        })),
    )
    .await;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("create response body");
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create space failed: {}",
        String::from_utf8_lossy(&body)
    );
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("create payload");
    payload["data"]["item"]["id"]
        .as_u64()
        .or_else(|| payload["data"]["item"]["id"].as_str()?.parse().ok())
        .expect("space id")
}

async fn seed_group_binding(runtime: &KnowledgebaseRuntime, space_id: u64) {
    let space_uuid = sqlx::query("SELECT uuid FROM kb_space WHERE id = $1")
        .bind(space_id as i64)
        .fetch_one(runtime.pool())
        .await
        .expect("space uuid")
        .get::<String, _>("uuid");
    let binding_id = 90_000_000_001_i64;
    let now = "2026-07-13T00:00:00Z";

    sqlx::query(
        r#"
        INSERT INTO kb_group_knowledge_space_binding (
            id, uuid, tenant_id, organization_id, conversation_id, space_id, space_uuid,
            group_name, lifecycle_state, acl_projection_state,
            provisioning_idempotency_key_sha256_hex, membership_epoch,
            created_by, updated_by, created_at, updated_at, version
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(binding_id)
    .bind("group-binding-security-test")
    .bind(TENANT_ID as i64)
    .bind(ORGANIZATION_ID as i64)
    .bind("conversation-security-test")
    .bind(space_id as i64)
    .bind(space_uuid)
    .bind("Security Test Group")
    .bind("active")
    .bind("active")
    .bind("a".repeat(64))
    .bind(1_i64)
    .bind("im")
    .bind("im")
    .bind(now)
    .bind(now)
    .bind(0_i64)
    .execute(runtime.pool())
    .await
    .expect("group binding");

    for (offset, actor_id, role, access_level) in [
        (1_i64, GROUP_OWNER_ID, "owner", "owner"),
        (2_i64, GROUP_ADMIN_ID, "admin", "writer"),
        (3_i64, GROUP_MEMBER_ID, "member", "reader"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO kb_group_knowledge_space_member (
                id, uuid, tenant_id, organization_id, binding_id, principal_kind, actor_id,
                member_role, access_level, membership_epoch, status, created_at, updated_at,
                version
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(binding_id + offset)
        .bind(format!("group-member-security-{offset}"))
        .bind(TENANT_ID as i64)
        .bind(ORGANIZATION_ID as i64)
        .bind(binding_id)
        .bind("user")
        .bind(actor_id.to_string())
        .bind(role)
        .bind(access_level)
        .bind(1_i64)
        .bind(1_i64)
        .bind(now)
        .bind(now)
        .bind(0_i64)
        .execute(runtime.pool())
        .await
        .expect("group member");
    }
}

async fn assert_persisted_space(runtime: &KnowledgebaseRuntime, space_id: u64) {
    let row =
        sqlx::query("SELECT id, tenant_id, organization_id, status FROM kb_space WHERE id = $1")
            .bind(space_id as i64)
            .fetch_optional(runtime.pool())
            .await
            .expect("query created knowledge space");
    let row = row.unwrap_or_else(|| panic!("created API space {space_id} was not persisted"));
    assert_eq!(row.get::<i64, _>("tenant_id"), TENANT_ID as i64);
    assert_eq!(row.get::<i64, _>("organization_id"), ORGANIZATION_ID as i64);
    assert_eq!(row.get::<i64, _>("status"), 1);

    let generic_visible = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM kb_space
        WHERE tenant_id = $1 AND organization_id = $2 AND id = $3 AND status = $4
          AND NOT EXISTS (
                SELECT 1
                FROM kb_group_knowledge_space_binding group_binding
                WHERE group_binding.tenant_id = kb_space.tenant_id
                  AND group_binding.organization_id = kb_space.organization_id
                  AND group_binding.space_id = kb_space.id
          )
        "#,
    )
    .bind(TENANT_ID as i64)
    .bind(ORGANIZATION_ID as i64)
    .bind(space_id as i64)
    .bind(1_i64)
    .fetch_one(runtime.pool())
    .await
    .expect("query generic visibility for created knowledge space");
    assert_eq!(generic_visible, 1);

    let store =
        PostgresKnowledgeSpaceStore::new(runtime.pool().clone(), TENANT_ID, ORGANIZATION_ID);
    store
        .get_space(space_id)
        .await
        .expect("newly created ordinary space must be visible through the repository store");

    let binding_store = PostgresGroupKnowledgeSpaceBindingStore::new(runtime.pool().clone());
    assert!(binding_store
        .find_group_space_for_space_in_tenant(TENANT_ID, ORGANIZATION_ID, space_id)
        .await
        .expect("query group binding for newly created ordinary space")
        .is_none());
}

async fn assert_status(
    runtime: &KnowledgebaseRuntime,
    context: KnowledgeAppRequestContext,
    method: Method,
    uri: String,
    body: Option<serde_json::Value>,
    expected: StatusCode,
) {
    let response = send(runtime, context, method, uri, body).await;
    let status = response.status();
    let payload = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    assert_eq!(
        status,
        expected,
        "unexpected status with body: {}",
        String::from_utf8_lossy(&payload)
    );
}

async fn send(
    runtime: &KnowledgebaseRuntime,
    context: KnowledgeAppRequestContext,
    method: Method,
    uri: String,
    body: Option<serde_json::Value>,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .extension(context);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    runtime
        .build_full_app_router()
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("route response")
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
        .expect("system clock")
        .as_nanos();
    let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let work_dir = std::env::current_dir().expect("work dir");
    let test_root = work_dir
        .join("target")
        .join("group-space-access-security-tests")
        .join(format!("{}-{}-{}", std::process::id(), nanos, sequence));
    std::fs::create_dir_all(&test_root).expect("test root");
    let drive_root = test_root.join("drive-objects");
    std::fs::create_dir_all(&drive_root).expect("drive root");
    std::env::set_var(
        "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT",
        drive_root.to_string_lossy().as_ref(),
    );
    KnowledgebaseRuntime::connect(&database_url, TENANT_ID)
        .await
        .expect("runtime")
        .into()
}
