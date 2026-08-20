use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use sdkwork_intelligence_knowledgebase_repository_sqlx::is_postgres_database_url;
use sdkwork_routes_knowledgebase_app_api::{
    paths, KnowledgeAppRequestContext, KnowledgebaseRuntime,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, MutexGuard};
use tower::util::ServiceExt;

static TENANT_ISOLATION_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

async fn tenant_isolation_test_lock() -> MutexGuard<'static, ()> {
    TENANT_ISOLATION_TEST_LOCK.lock().await
}

struct TestEnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl TestEnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for TestEnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn app_context(
    tenant_id: u64,
    actor_id: u64,
    organization_id: Option<u64>,
) -> KnowledgeAppRequestContext {
    KnowledgeAppRequestContext {
        tenant_id,
        actor_id: Some(actor_id),
        organization_id,
        session_id: None,
        request_id: "test-request-tenant-isolation".to_string(),
        trace_id: None,
        idempotency_key: None,
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
async fn tenant_id_mismatch_rejects_space_retrieve() {
    let _guard = tenant_isolation_test_lock().await;
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let space_id = create_space(&runtime, app_context(1, 42, None)).await;
    let app = runtime.build_full_app_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/app/v3/api/knowledge/spaces/{space_id}"))
                .extension(app_context(2, 42, None))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response_body_json(response).await;
    assert_eq!(body["code"].as_i64(), Some(40303));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn tenant_id_mismatch_rejects_space_creation() {
    let _guard = tenant_isolation_test_lock().await;
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = runtime.build_full_app_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(paths::SPACES)
                .header("content-type", "application/json")
                .extension(app_context(2, 42, None))
                .body(Body::from(
                    json!({
                        "name": "cross-tenant-space",
                        "description": "must be rejected"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response_body_json(response).await;
    assert_eq!(body["code"].as_i64(), Some(40303));
}

#[ignore = "requires a PostgreSQL integration environment; the Knowledgebase server runtime requires PostgreSQL by architecture"]
#[tokio::test]
async fn organization_id_mismatch_rejects_when_runtime_org_configured() {
    let _guard = tenant_isolation_test_lock().await;
    let _org_env = TestEnvVarGuard::set("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID", "100");
    let Some(runtime) = test_runtime().await else {
        eprintln!(
            "skipping integration test: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let app = runtime.build_full_app_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(paths::SPACES)
                .header("content-type", "application/json")
                .extension(app_context(1, 42, Some(200)))
                .body(Body::from(
                    json!({
                        "name": "org-mismatch-space",
                        "description": "must be rejected"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response_body_json(response).await;
    assert_eq!(body["code"].as_i64(), Some(40304));
}

async fn create_space(runtime: &KnowledgebaseRuntime, context: KnowledgeAppRequestContext) -> u64 {
    let app = runtime.build_full_app_router();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(paths::SPACES)
                .header("content-type", "application/json")
                .extension(context)
                .body(Body::from(
                    json!({
                        "name": "tenant-isolation-space",
                        "description": "tenant isolation integration test"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read create space response");
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create space failed: {}",
        String::from_utf8_lossy(&bytes)
    );
    let body: Value = serde_json::from_slice(&bytes).expect("parse create space response");
    json_item_u64_field(&body, "id").expect("space id")
}

fn json_item_u64_field(body: &Value, field: &str) -> Option<u64> {
    body.pointer("/data/item")
        .and_then(|item| json_u64_field(item, field))
}

fn json_u64_field(body: &Value, field: &str) -> Option<u64> {
    body.get(field)
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
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
        .expect("system clock before unix epoch")
        .as_nanos();
    let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let work_dir = std::env::current_dir().expect("current directory");
    let test_root = work_dir
        .join("target")
        .join("integration-tenant-isolation-tests")
        .join(format!("{}-{}-{}", std::process::id(), nanos, sequence));
    std::fs::create_dir_all(&test_root).expect("create integration test directory");
    let drive_root = test_root.join("drive-objects");
    std::fs::create_dir_all(&drive_root).expect("create drive storage root");
    std::env::set_var(
        "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT",
        drive_root.to_string_lossy().as_ref(),
    );

    KnowledgebaseRuntime::connect(&database_url, 1)
        .await
        .expect("initialize integration runtime")
        .into()
}

async fn response_body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse response json");
    sdkwork_knowledgebase_test_support::api_envelope::unwrap_payload_or_envelope(&value)
}
