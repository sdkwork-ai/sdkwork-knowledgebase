use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_knowledgebase_contract::browser::{
    KnowledgeBrowserListData, KnowledgeBrowserNode, KnowledgeBrowserNodePermissions,
    KnowledgeBrowserNodeType, KnowledgeBrowserView, ListKnowledgeBrowserRequest,
};
use sdkwork_knowledgebase_contract::document::{
    KnowledgeDocument, KnowledgeDocumentState, KnowledgeDocumentVersionState,
    KnowledgeDocumentVisibility,
};
use sdkwork_knowledgebase_contract::ingest::{
    IngestionJob, IngestionJobState, KnowledgeIngestRequest,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeContextFragment, KnowledgeContextPack, KnowledgeContextPackRequest,
    KnowledgeRetrievalMethod, KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
    KnowledgeRetrievalTrace,
};
use sdkwork_routes_knowledgebase_open_api::{
    build_router_with_open_api, manifest, open_route_manifest, wrap_router_with_web_framework,
    wrap_router_with_web_framework_from_env, ApiResult, KnowledgeOpenApi,
    KnowledgeOpenApiRequestContext, ProblemDetails,
};
use sdkwork_web_core::RouteAuth;
use serde_json::Value;
use std::sync::Mutex;
use tower::util::ServiceExt;

#[test]
fn open_api_manifest_uses_public_knowledge_prefix_and_api_key_auth() {
    assert_eq!(
        manifest::PACKAGE_NAME,
        "sdkwork-routes-knowledgebase-open-api"
    );
    assert_eq!(manifest::SURFACE, "open-api");
    assert_eq!(manifest::OWNER, "sdkwork-knowledgebase");
    assert_eq!(manifest::DOMAIN, "intelligence");
    assert_eq!(manifest::CAPABILITY, "knowledgebase");
    assert_eq!(manifest::API_AUTHORITY, "sdkwork-knowledgebase-open-api");
    assert_eq!(manifest::SDK_FAMILY, "sdkwork-knowledgebase-sdk");
    assert_eq!(manifest::PREFIX, "/knowledge/v3/api");

    let routes = manifest::ROUTES;
    assert_eq!(routes.len(), 8);
    assert!(routes.iter().all(|route| route.auth_mode == "api-key"));
    assert!(routes
        .iter()
        .all(|route| route.path.starts_with("/knowledge/v3/api")));
    assert!(routes
        .iter()
        .all(|route| !route.path.starts_with("/app/v3/api")));
    assert!(routes
        .iter()
        .all(|route| !route.path.starts_with("/backend/v3/api")));

    assert_route("POST", "/knowledge/v3/api/retrievals", "retrievals.create");
    assert_route(
        "GET",
        "/knowledge/v3/api/retrievals/{retrievalId}",
        "retrievals.retrieve",
    );
    assert_route(
        "POST",
        "/knowledge/v3/api/context_packs",
        "contextPacks.create",
    );
    assert_route("POST", "/knowledge/v3/api/ingests", "ingests.create");
    assert_route(
        "GET",
        "/knowledge/v3/api/ingests/{ingestId}",
        "ingests.retrieve",
    );
    assert_route("GET", "/knowledge/v3/api/documents", "documents.list");
    assert_route(
        "GET",
        "/knowledge/v3/api/documents/{documentId}",
        "documents.retrieve",
    );
    assert_route(
        "GET",
        "/knowledge/v3/api/spaces/{spaceId}/browser",
        "spaces.browser.list",
    );
}

#[test]
fn open_openapi_exposes_browser_list_data_context_contract() {
    let spec: Value = serde_json::from_str(include_str!(
        "../../../sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json"
    ))
    .unwrap();

    let operation = &spec["paths"]["/knowledge/v3/api/spaces/{spaceId}/browser"]["get"];
    assert!(
        operation["description"]
            .as_str()
            .unwrap_or_default()
            .contains("sources/raw"),
        "browser operation must document OKF raw source root"
    );
    let response_schema = &operation["responses"]["200"]["content"]["application/json"]["schema"];
    assert_eq!(
        response_schema["allOf"][1]["properties"]["data"]["$ref"],
        "#/components/schemas/KnowledgeBrowserListData"
    );
    assert!(spec["components"]["schemas"]["KnowledgeBrowserPage"].is_null());

    let data_schema = &spec["components"]["schemas"]["KnowledgeBrowserListData"];
    for property in [
        "spaceId",
        "driveSpaceId",
        "parentId",
        "view",
        "pageSize",
        "items",
        "pageInfo",
    ] {
        assert!(
            data_schema["properties"][property].is_object(),
            "KnowledgeBrowserListData must expose {property}"
        );
    }
    assert_eq!(
        data_schema["properties"]["items"]["items"]["$ref"],
        "#/components/schemas/KnowledgeBrowserNode"
    );
    assert_eq!(
        data_schema["properties"]["pageInfo"]["$ref"],
        "#/components/schemas/PageInfo"
    );
}

#[tokio::test]
async fn open_retrieval_route_calls_injected_service_with_api_key_context() {
    let service = RecordingOpenApi::default();
    let app = build_router_with_open_api(service.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/knowledge/v3/api/retrievals")
                .header("content-type", "application/json")
                .extension(open_context())
                .body(Body::from(
                    r#"{"actorId":"30001","query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"methods":["hybrid"],"includeCitations":true,"includeTrace":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["data"]["item"]["retrievalId"], "701");
    assert_eq!(body["data"]["item"]["hits"][0]["chunkId"], "11");
    assert_eq!(
        service.contexts(),
        vec![("api-key-001".to_string(), 100001)]
    );
    assert_eq!(service.last_retrieval_tenant_id(), Some(100001));
}

#[tokio::test]
async fn open_retrieval_route_rejects_missing_api_key_context() {
    let app = build_router_with_open_api(RecordingOpenApi::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/knowledge/v3/api/retrievals")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"methods":["hybrid"],"includeCitations":true,"includeTrace":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let problem: ProblemDetails = response_model(response).await;
    assert_eq!(problem.code, 40101);
}

#[tokio::test]
async fn open_context_pack_route_calls_injected_service() {
    let app = build_router_with_open_api(RecordingOpenApi::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/knowledge/v3/api/context_packs")
                .header("content-type", "application/json")
                .extension(open_context())
                .body(Body::from(
                    r#"{"actorId":"30001","query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"contextBudgetTokens":80,"includeCitations":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["data"]["item"]["contextPackId"], "801");
    assert_eq!(body["data"]["item"]["fragments"][0]["chunkId"], "11");
}

#[tokio::test]
async fn open_browser_route_preserves_query_parameters() {
    let service = RecordingOpenApi::default();
    let app = build_router_with_open_api(service.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/knowledge/v3/api/spaces/7/browser?view=okf_bundle&page_size=25&parent_id=node-okf&cursor=c1")
                .extension(open_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"]["pageInfo"]["mode"], "cursor");
    assert_eq!(body["data"]["parentId"], "node-okf");
    assert_eq!(body["data"]["driveSpaceId"], "drv-kb-001");
    assert_eq!(body["data"]["view"], "okf_bundle");
    assert_eq!(
        service.last_browser_request().unwrap(),
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
async fn open_browser_route_rejects_forbidden_page_size_alias() {
    let app = build_router_with_open_api(RecordingOpenApi::default());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/knowledge/v3/api/spaces/7/browser?view=okf_bundle&pageSize=25")
                .extension(open_context())
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
async fn open_router_exposes_document_and_ingest_read_routes() {
    let app = build_router_with_open_api(RecordingOpenApi::default());

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/knowledge/v3/api/documents?space_id=7")
                .extension(open_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(list_response).await["data"]["items"][0]["id"],
        "901"
    );

    let retrieve_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/knowledge/v3/api/documents/901")
                .extension(open_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(retrieve_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(retrieve_response).await["data"]["item"]["id"],
        "901"
    );

    let ingest_response = app
        .oneshot(
            Request::builder()
                .uri("/knowledge/v3/api/ingests/51")
                .extension(open_context())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ingest_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(ingest_response).await["data"]["item"]["id"],
        "51"
    );
}

#[test]
fn open_route_manifest_declares_api_key_auth_for_all_operations() {
    let manifest = open_route_manifest();
    assert_eq!(manifest::ROUTES.len(), 8);
    for entry in manifest::ROUTES {
        let matched = manifest
            .match_route(entry.method, entry.path)
            .unwrap_or_else(|| {
                panic!(
                    "missing http route manifest for {} {}",
                    entry.method, entry.path
                )
            });
        assert_eq!(matched.auth, RouteAuth::ApiKey);
        assert_eq!(matched.operation_id, entry.operation_id);
    }
}

#[tokio::test]
async fn open_router_web_framework_rejects_unauthenticated_requests() {
    let app = wrap_router_with_web_framework_from_env(build_router_with_open_api(
        RecordingOpenApi::default(),
    ))
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/knowledge/v3/api/retrievals")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"methods":["hybrid"],"includeCitations":true,"includeTrace":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Without an IAM database the resolver cannot look up credentials and the
    // framework fails closed with 503 dependency-unavailable; with a database
    // the same request is rejected with 401. Either way the unauthenticated
    // request must never reach the handler.
    assert!(
        matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
        ),
        "unauthenticated open-api request must be rejected, got {}",
        response.status()
    );
}

#[tokio::test]
async fn open_router_web_framework_accepts_dev_inline_api_key_before_handler() {
    std::env::set_var("SDKWORK_ENV", "dev");
    let service = RecordingOpenApi::default();
    let app = wrap_router_with_web_framework(
        IamWebRequestContextResolver::new(None),
        build_router_with_open_api(service.clone()),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/knowledge/v3/api/retrievals")
                .header("content-type", "application/json")
                .header(
                    "x-api-key",
                    "api_key_id=api-key-001;tenant_id=100001;user_id=30001;app_id=knowledgebase",
                )
                .body(Body::from(
                    r#"{"actorId":"30001","query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"methods":["hybrid"],"includeCitations":true,"includeTrace":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        service.contexts(),
        vec![("api-key-001".to_string(), 100001)]
    );
}

fn assert_route(method: &str, path: &str, operation_id: &str) {
    assert!(
        manifest::ROUTES.iter().any(|route| {
            route.method == method && route.path == path && route.operation_id == operation_id
        }),
        "missing route {method} {path} {operation_id}"
    );
}

fn open_context() -> KnowledgeOpenApiRequestContext {
    KnowledgeOpenApiRequestContext {
        api_key_id: "api-key-001".to_string(),
        tenant_id: 100001,
        actor_id: Some(30001),
        organization_id: Some(100),
        request_id: "test-request-open-api".to_string(),
        trace_id: None,
        idempotency_key: None,
    }
}

#[derive(Clone, Default)]
struct RecordingOpenApi {
    contexts: std::sync::Arc<Mutex<Vec<(String, u64)>>>,
    retrieval_tenant_ids: std::sync::Arc<Mutex<Vec<u64>>>,
    browser_request: std::sync::Arc<Mutex<Option<ListKnowledgeBrowserRequest>>>,
}

impl RecordingOpenApi {
    fn contexts(&self) -> Vec<(String, u64)> {
        self.contexts.lock().unwrap().clone()
    }

    fn last_browser_request(&self) -> Option<ListKnowledgeBrowserRequest> {
        self.browser_request.lock().unwrap().clone()
    }

    fn last_retrieval_tenant_id(&self) -> Option<u64> {
        self.retrieval_tenant_ids.lock().unwrap().last().copied()
    }
}

#[async_trait]
impl KnowledgeOpenApi for RecordingOpenApi {
    async fn create_retrieval(
        &self,
        context: KnowledgeOpenApiRequestContext,
        _request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.contexts
            .lock()
            .unwrap()
            .push((context.api_key_id, context.tenant_id));
        self.retrieval_tenant_ids
            .lock()
            .unwrap()
            .push(_request.tenant_id);
        Ok(retrieval_result(701))
    }

    async fn retrieve_retrieval(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Ok(retrieval_result(retrieval_id))
    }

    async fn create_context_pack(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        _request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack> {
        Ok(KnowledgeContextPack {
            context_pack_id: 801,
            retrieval_id: Some(701),
            query: "enterprise renewal support".to_string(),
            fragments: vec![context_fragment()],
            memory_fragments: vec![],
            estimated_tokens: 8,
            citations: vec![],
            truncated: false,
        })
    }

    async fn create_ingest(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        request: KnowledgeIngestRequest,
    ) -> ApiResult<IngestionJob> {
        Ok(IngestionJob {
            id: 51,
            space_id: request.space_id,
            source_type: "api_payload".to_string(),
            idempotency_key: request.idempotency_key,
            state: IngestionJobState::Queued,
            error_message: None,
        })
    }

    async fn retrieve_ingest(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        ingest_id: u64,
    ) -> ApiResult<IngestionJob> {
        Ok(IngestionJob {
            id: ingest_id,
            space_id: 7,
            source_type: "api_payload".to_string(),
            idempotency_key: "api-note".to_string(),
            state: IngestionJobState::Succeeded,
            error_message: None,
        })
    }

    async fn list_documents(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        _space_id: u64,
        _cursor: Option<String>,
        _page_size: Option<u32>,
    ) -> ApiResult<sdkwork_utils_rust::SdkWorkPageData<KnowledgeDocument>> {
        Ok(sdkwork_utils_rust::SdkWorkPageData {
            items: vec![document(901)],
            page_info: sdkwork_utils_rust::PageInfo {
                mode: sdkwork_utils_rust::PageMode::Cursor,
                page: None,
                page_size: Some(20),
                total_items: None,
                total_pages: None,
                next_cursor: None,
                has_more: Some(false),
            },
        })
    }

    async fn retrieve_document(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        document_id: u64,
    ) -> ApiResult<KnowledgeDocument> {
        Ok(document(document_id))
    }

    async fn list_browser(
        &self,
        _context: KnowledgeOpenApiRequestContext,
        request: ListKnowledgeBrowserRequest,
    ) -> ApiResult<KnowledgeBrowserListData> {
        *self.browser_request.lock().unwrap() = Some(request.clone());
        Ok(KnowledgeBrowserListData {
            space_id: request.space_id,
            drive_space_id: "drv-kb-001".to_string(),
            parent_id: request.parent_id.clone(),
            view: request.view,
            page_size: request.page_size.unwrap_or(50),
            items: vec![KnowledgeBrowserNode {
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
            page_info: sdkwork_utils_rust::PageInfo {
                mode: sdkwork_utils_rust::PageMode::Cursor,
                page: None,
                page_size: Some(request.page_size.unwrap_or(50) as i32),
                total_items: None,
                total_pages: None,
                next_cursor: None,
                has_more: Some(false),
            },
        })
    }
}

fn retrieval_result(retrieval_id: u64) -> KnowledgeRetrievalResult {
    KnowledgeRetrievalResult {
        retrieval_id,
        trace: Some(KnowledgeRetrievalTrace {
            retrieval_trace_id: retrieval_id,
            status: "succeeded".to_string(),
            latency_ms: Some(9),
            result_count: 1,
        }),
        hits: vec![context_fragment()],
    }
}

fn context_fragment() -> KnowledgeContextFragment {
    KnowledgeContextFragment {
        chunk_id: 11,
        document_id: 101,
        document_version_id: Some(201),
        space_id: 7,
        title: "Support Playbook".to_string(),
        content: "enterprise renewal support answer".to_string(),
        score: Some(0.91),
        rank: 1,
        token_count: Some(8),
        retrieval_method: KnowledgeRetrievalMethod::Hybrid,
        citation: None,
    }
}

fn document(document_id: u64) -> KnowledgeDocument {
    KnowledgeDocument {
        id: document_id,
        space_id: 7,
        source_id: Some(31),
        original_file_drive_node_id: Some("node-index".to_string()),
        title: "Support Playbook".to_string(),
        mime_type: Some("text/markdown".to_string()),
        language: Some("en-US".to_string()),
        current_version_id: Some(101),
        visibility: KnowledgeDocumentVisibility::Organization,
        content_state: KnowledgeDocumentState::Ready,
        index_state: KnowledgeDocumentVersionState::Succeeded,
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    response_model(response).await
}

async fn response_model<T>(response: axum::response::Response) -> T
where
    T: serde::de::DeserializeOwned,
{
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
