use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_knowledgebase_contract::browser::{
    KnowledgeBrowserListData, ListKnowledgeBrowserRequest,
};
use sdkwork_routes_knowledgebase_app_api::{
    app_route_manifest, build_router_with_browser, manifest, pagination::browser_list_page_data,
    wrap_router_with_web_framework, ApiResult, KnowledgeAppRequestContext, KnowledgeBrowserApi,
};
use sdkwork_web_core::RouteAuth;
use sdkwork_web_core::{
    access_token_jwt, auth_token_jwt_with_permissions, encode_unsigned_test_jwt,
};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tower::util::ServiceExt;

static ENV_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

#[test]
fn app_route_manifest_declares_dual_token_auth_for_all_operations() {
    let manifest = app_route_manifest();
    assert_eq!(manifest::ROUTES.len(), manifest.routes().len());
    for entry in manifest::ROUTES {
        let matched = manifest
            .match_route(entry.method, entry.path)
            .unwrap_or_else(|| {
                panic!(
                    "missing http route manifest for {} {}",
                    entry.method, entry.path
                )
            });
        assert_eq!(matched.auth, RouteAuth::DualToken);
        assert_eq!(matched.operation_id, entry.operation_id);
    }
}

#[tokio::test]
async fn app_router_web_framework_rejects_unauthenticated_requests() {
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_browser(EmptyBrowserApi),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/knowledge/spaces/7/browser")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn web_framework_accepts_dev_jwt_dual_tokens_before_handler() {
    let _env_guard = ENV_LOCK.lock().await;
    std::env::set_var("SDKWORK_ENV", "dev");
    std::env::set_var("SDKWORK_IAM_ALLOW_DEV_AUTH_FALLBACK", "true");
    let service = RecordingBrowserApi::default();
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_browser(service.clone()),
    );
    let auth = format!(
        "Bearer {}",
        auth_token_jwt_with_permissions(
            "100001",
            "30001",
            "session-1",
            "sdkwork-knowledgebase",
            "knowledge.spaces.read",
        )
    );
    let access = access_token_jwt("100001", "30001", "session-1", "sdkwork-knowledgebase");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/knowledge/spaces/7/browser")
                .header("Authorization", auth)
                .header("Access-Token", access)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(service.tenant_ids(), vec![100001]);
    std::env::remove_var("SDKWORK_ENV");
    std::env::remove_var("SDKWORK_IAM_ALLOW_DEV_AUTH_FALLBACK");
}

#[tokio::test]
async fn web_framework_prefers_auth_token_tenant_and_organization_context() {
    let _env_guard = ENV_LOCK.lock().await;
    std::env::set_var("SDKWORK_ENV", "dev");
    std::env::set_var("SDKWORK_IAM_ALLOW_DEV_AUTH_FALLBACK", "true");
    let service = RecordingBrowserApi::default();
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_browser(service.clone()),
    );
    let auth = format!(
        "Bearer {}",
        encode_unsigned_test_jwt(serde_json::json!({
            "token_type": "auth",
            "tenant_id": "100001",
            "organization_id": "200001",
            "login_scope": "ORGANIZATION",
            "user_id": "30001",
            "session_id": "session-1",
            "app_id": "sdkwork-knowledgebase",
            "auth_level": "password",
            // token-claims-gate: legacy-fixture — constructs a pre-slimming credential so this
            // test can assert the claim is no longer an authorization source.
            "permission_scope": "knowledge.spaces.read"
        }))
    );
    let access = access_token_jwt("100001", "30001", "session-1", "sdkwork-knowledgebase");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/knowledge/spaces/7/browser")
                .header("Authorization", auth)
                .header("Access-Token", access)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(service.tenant_ids(), vec![100001]);
    assert_eq!(service.organization_ids(), vec![Some(200001)]);
    std::env::remove_var("SDKWORK_ENV");
    std::env::remove_var("SDKWORK_IAM_ALLOW_DEV_AUTH_FALLBACK");
}

#[tokio::test]
async fn web_framework_allows_browser_origin_in_development() {
    let _env_guard = ENV_LOCK.lock().await;
    std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "development");
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_browser(EmptyBrowserApi),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/v3/api/knowledge/spaces")
                .header("origin", "http://127.0.0.1:5184")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Dev Space"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT");
}

#[tokio::test]
async fn web_framework_allows_im_host_browser_origin_in_development() {
    let _env_guard = ENV_LOCK.lock().await;
    std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "development");
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_browser(EmptyBrowserApi),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/v3/api/knowledge/spaces")
                .header("origin", "http://localhost:4176")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Dev Space"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT");
}

#[tokio::test]
async fn web_framework_rejects_unlisted_browser_origin_in_development() {
    let _env_guard = ENV_LOCK.lock().await;
    std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "development");
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_browser(EmptyBrowserApi),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/v3/api/knowledge/spaces")
                .header("origin", "http://evil.example")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Dev Space"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT");
}

struct EmptyBrowserApi;

#[async_trait]
impl KnowledgeBrowserApi for EmptyBrowserApi {
    async fn list_browser(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        unreachable!("unauthenticated requests must not reach handlers")
    }
}

#[derive(Clone, Default)]
struct RecordingBrowserApi {
    tenant_ids: Arc<Mutex<Vec<u64>>>,
    organization_ids: Arc<Mutex<Vec<Option<u64>>>>,
}

impl RecordingBrowserApi {
    fn tenant_ids(&self) -> Vec<u64> {
        self.tenant_ids.lock().unwrap().clone()
    }

    fn organization_ids(&self) -> Vec<Option<u64>> {
        self.organization_ids.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeBrowserApi for RecordingBrowserApi {
    async fn list_browser(
        &self,
        context: KnowledgeAppRequestContext,
        request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        self.tenant_ids.lock().unwrap().push(context.tenant_id);
        self.organization_ids
            .lock()
            .unwrap()
            .push(context.organization_id);
        Ok(browser_list_page_data(
            request.space_id,
            "drv-kb-001".to_string(),
            request.parent_id,
            request.view,
            vec![],
            None,
            request.page_size.unwrap_or(50),
        ))
    }
}
