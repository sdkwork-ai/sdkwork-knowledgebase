use async_trait::async_trait;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{Request as HttpRequest, StatusCode};
use axum::middleware::{self, Next};
use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_knowledgebase_contract::{KnowledgeSource, KnowledgeSourceList};
use sdkwork_utils_rust::SdkWorkPageData;
use sdkwork_routes_knowledgebase_backend_api::{
    backend_route_manifest, build_router_with_backend_api, manifest,
    wrap_router_with_web_framework, BackendApiResult, KnowledgeBackendApi,
    KnowledgeBackendRequestContext,
};
use sdkwork_web_core::RouteAuth;
use tower::util::ServiceExt;

#[test]
fn backend_route_manifest_declares_dual_token_auth_for_all_operations() {
    let manifest = backend_route_manifest();
    assert_eq!(
        manifest::ROUTES.len(),
        manifest.routes().len(),
        "manifest::ROUTES must stay aligned with backend HTTP route manifest"
    );
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
async fn backend_router_web_framework_rejects_unauthenticated_requests() {
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_backend_api(EmptyBackendApi, 1),
    );

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/backend/v3/api/knowledge/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn dev_backend_auth_injects_backend_context_before_handler() {
    let app = with_dev_backend_auth(
        build_router_with_backend_api(OkBackendApi, 100_001),
        100_001,
        Some(99),
    );

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/backend/v3/api/knowledge/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn dev_backend_auth_rejects_tenant_id_mismatch() {
    let app = with_dev_backend_auth(
        build_router_with_backend_api(OkBackendApi, 100_001),
        200_002,
        Some(99),
    );

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/backend/v3/api/knowledge/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn dev_backend_auth_rejects_missing_knowledge_admin_permission() {
    let app = build_router_with_backend_api(OkBackendApi, 100_001).layer(middleware::from_fn(
        |mut request: Request, next: Next| async move {
            if request
                .extensions()
                .get::<KnowledgeBackendRequestContext>()
                .is_none()
            {
                request
                    .extensions_mut()
                    .insert(KnowledgeBackendRequestContext {
                        tenant_id: 100_001,
                        operator_id: Some(99),
                        organization_id: None,
                        permission_scope: vec!["knowledge.spaces.read".to_string()],
                        trace_id: "trace-insufficient-permission".to_string(),
                    });
            }
            next.run(request).await
        },
    ));

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/backend/v3/api/knowledge/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn backend_router_authorizes_personal_login_scope_sessions_with_permission() {
    // The SDKWork web-framework authority permits personal (TENANT login scope)
    // sessions on backend surfaces (sdkwork-web-core `EnforcePrincipalTenantIsolationPolicy`
    // and sdkwork-iam `IamAuthorizationPolicy`); backend authorization stays
    // permission-driven, so the `knowledge.platform.manage` scope alone must pass.
    std::env::set_var("SDKWORK_ENV", "dev");
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_backend_api(OkBackendApi, 100_001),
    );

    let tenant_id = "100001";
    let auth = format!(
        "Bearer {}",
        sdkwork_web_core::auth_token_jwt_with_permissions(
            tenant_id,
            "99",
            "session-1",
            "sdkwork-knowledgebase",
            "knowledge.platform.manage",
        )
    );
    let access =
        sdkwork_web_core::access_token_jwt(tenant_id, "99", "session-1", "sdkwork-knowledgebase");

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/backend/v3/api/knowledge/sources")
                .header("Authorization", auth)
                .header("Access-Token", access)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn backend_router_rejects_personal_login_scope_sessions_without_permission() {
    // Permission gate stays authoritative for personal-scope sessions: without the
    // `knowledge.platform.manage` scope the backend surface must still reject.
    std::env::set_var("SDKWORK_ENV", "dev");
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_backend_api(OkBackendApi, 100_001),
    );

    let tenant_id = "100001";
    let auth = format!(
        "Bearer {}",
        sdkwork_web_core::auth_token_jwt_with_permissions(
            tenant_id,
            "99",
            "session-1",
            "sdkwork-knowledgebase",
            "knowledge.spaces.read",
        )
    );
    let access =
        sdkwork_web_core::access_token_jwt(tenant_id, "99", "session-1", "sdkwork-knowledgebase");

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/backend/v3/api/knowledge/sources")
                .header("Authorization", auth)
                .header("Access-Token", access)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

fn with_dev_backend_auth(router: Router, tenant_id: u64, operator_id: Option<u64>) -> Router {
    router.layer(middleware::from_fn(
        move |mut request: Request, next: Next| {
            let operator_id = operator_id;
            async move {
                if request
                    .extensions()
                    .get::<KnowledgeBackendRequestContext>()
                    .is_none()
                {
                    request
                        .extensions_mut()
                        .insert(KnowledgeBackendRequestContext {
                            tenant_id,
                            operator_id,
                            organization_id: None,
                            permission_scope: vec![
                                sdkwork_routes_knowledgebase_backend_api::permission::KNOWLEDGE_PLATFORM_MANAGE_PERMISSION
                                    .to_string(),
                            ],
                            trace_id: "trace-backend-auth".to_string(),
                        });
                }
                next.run(request).await
            }
        },
    ))
}

struct EmptyBackendApi;

impl KnowledgeBackendApi for EmptyBackendApi {}

struct OkBackendApi;

#[async_trait]
impl KnowledgeBackendApi for OkBackendApi {
    async fn list_sources(&self) -> BackendApiResult<KnowledgeSourceList> {
        Ok(KnowledgeSourceList { items: vec![] })
    }

    async fn list_sources_page(
        &self,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> BackendApiResult<SdkWorkPageData<KnowledgeSource>> {
        // Auth tests exercise the router boundary; the page method must return an empty
        // page instead of relying on the (now forbidden) unbounded facade default.
        Ok(sdkwork_routes_knowledgebase_backend_api::pagination::cursor_page_data(
            vec![],
            None,
            false,
            20,
        ))
    }
}
