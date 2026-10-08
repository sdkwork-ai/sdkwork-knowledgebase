use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use sdkwork_knowledgebase_contract::browser::{
    KnowledgeBrowserListData, KnowledgeBrowserNode, KnowledgeBrowserNodePermissions,
    KnowledgeBrowserNodeType, KnowledgeBrowserView, ListKnowledgeBrowserRequest,
};
use sdkwork_routes_knowledgebase_app_api::{
    build_router_with_browser, pagination::browser_list_page_data, ApiResult,
    KnowledgeAppRequestContext, KnowledgeBrowserApi, ProblemDetails,
};
use std::sync::Mutex;
use tower::util::ServiceExt;

fn app_request_context() -> KnowledgeAppRequestContext {
    KnowledgeAppRequestContext {
        tenant_id: 100001,
        actor_id: Some(30001),
        organization_id: None,
        session_id: None,
        request_id: "test-request-browser-routes".to_string(),
        trace_id: None,
        idempotency_key: None,
    }
}

#[tokio::test]
async fn app_router_exposes_browser_route_with_query_parameters() {
    let browser = RecordingBrowserApi::default();
    let app = build_router_with_browser(browser.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/app/v3/api/knowledge/spaces/7/browser?view=okf_bundle&page_size=25&parent_id=node-okf&cursor=c1")
                .extension(app_request_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(0, payload["code"].as_i64().unwrap());
    assert_eq!(1, payload["data"]["items"].as_array().unwrap().len());
    assert_eq!(
        "okf_concept",
        payload["data"]["items"][0]["nodeType"].as_str().unwrap()
    );
    assert_eq!(
        "cursor",
        payload["data"]["pageInfo"]["mode"].as_str().unwrap()
    );
    assert_eq!("node-okf", payload["data"]["parentId"].as_str().unwrap());
    assert_eq!(
        "drv-kb-001",
        payload["data"]["driveSpaceId"].as_str().unwrap()
    );
    assert_eq!("okf_bundle", payload["data"]["view"].as_str().unwrap());
    assert_eq!(
        browser.last_request().unwrap(),
        ListKnowledgeBrowserRequest {
            space_id: 7,
            parent_id: Some("node-okf".to_string()),
            view: KnowledgeBrowserView::OkfBundle,
            cursor: Some("c1".to_string()),
            page_size: Some(25),
        }
    );
}

#[tokio::test]
async fn app_router_rejects_forbidden_page_size_alias() {
    let app = sdkwork_knowledgebase_observability::wrap_router_with_metrics(
        build_router_with_browser(RecordingBrowserApi::default()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/app/v3/api/knowledge/spaces/7/browser?view=okf_bundle&pageSize=25")
                .extension(app_request_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let problem: ProblemDetails = serde_json::from_slice(&body).unwrap();
    assert_eq!(problem.status, 400);
    assert_eq!(problem.code, 40003);
    assert_eq!(
        problem.detail.as_deref(),
        Some("HTTP query parameter pageSize is forbidden; use page_size")
    );
}

#[tokio::test]
async fn app_router_rejects_invalid_browser_view() {
    let app = sdkwork_knowledgebase_observability::wrap_router_with_metrics(
        build_router_with_browser(RecordingBrowserApi::default()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/app/v3/api/knowledge/spaces/7/browser?view=invalid")
                .header("x-request-id", "corr-browser-001")
                .extension(app_request_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("corr-browser-001")
    );
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let problem: ProblemDetails = serde_json::from_slice(&body).unwrap();
    assert_eq!(problem.r#type, "https://docs.sdkwork.com/problems/40001");
    assert_eq!(problem.title, "Bad Request");
    assert_eq!(problem.status, 400);
    assert_eq!(problem.code, 40001);
    assert_eq!(problem.trace_id, "corr-browser-001");
    assert_eq!(
        problem.detail.as_deref(),
        Some("unsupported browser view: invalid")
    );
}

#[derive(Clone, Default)]
struct RecordingBrowserApi {
    last_request: std::sync::Arc<Mutex<Option<ListKnowledgeBrowserRequest>>>,
}

impl RecordingBrowserApi {
    fn last_request(&self) -> Option<ListKnowledgeBrowserRequest> {
        self.last_request.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeBrowserApi for RecordingBrowserApi {
    async fn list_browser(
        &self,
        _context: KnowledgeAppRequestContext,
        request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(browser_list_page_data(
            request.space_id,
            "drv-kb-001".to_string(),
            request.parent_id.clone(),
            request.view,
            vec![KnowledgeBrowserNode {
                id: "node-index".to_string(),
                node_type: KnowledgeBrowserNodeType::OkfConcept,
                name: "index.md".to_string(),
                parent_id: Some("node-okf".to_string()),
                path: "okf/index.md".to_string(),
                drive_space_id: Some("drv-kb-001".to_string()),
                drive_node_id: Some("node-index".to_string()),
                document_id: None,
                document_version_id: None,
                concept_id: Some(1),
                concept_revision_id: Some(2),
                mime_type: Some("text/markdown; charset=utf-8".to_string()),
                size_bytes: Some(64),
                ingest_state: None,
                parse_state: None,
                index_state: None,
                okf_state: Some("published".to_string()),
                children_count: None,
                updated_at: "2026-06-04T12:00:00Z".to_string(),
                permissions: KnowledgeBrowserNodePermissions::read_only(),
                drive_storage_provider_id: None,
                drive_bucket: None,
                drive_object_key: None,
            }],
            None,
            request.page_size.unwrap_or(50),
        ))
    }
}
