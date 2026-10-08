use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeContextPack, KnowledgeContextPackRequest, KnowledgeRetrievalMethod,
    KnowledgeRetrievalRequest, KnowledgeRetrievalResult, KnowledgeRetrievalTrace,
};
use sdkwork_routes_knowledgebase_app_api::{
    build_router_with_retrieval_service, ApiError, ApiResult, KnowledgeAppRequestContext,
    KnowledgeRetrievalAppService,
};
use serde_json::Value;
use std::sync::Mutex;
use tower::util::ServiceExt;

fn app_request_context() -> KnowledgeAppRequestContext {
    KnowledgeAppRequestContext {
        tenant_id: 100001,
        actor_id: Some(30001),
        organization_id: None,
        session_id: None,
        request_id: "test-request-retrieval-routes".to_string(),
        trace_id: None,
        idempotency_key: None,
    }
}

#[tokio::test]
async fn retrieval_route_calls_injected_retrieval_service() {
    let service = RecordingRetrievalService::default();
    let app = build_router_with_retrieval_service(service.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/v3/api/knowledge/retrievals")
                .header("content-type", "application/json")
                .extension(app_request_context())
                .body(Body::from(
                    r#"{"actorId":"30001","query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"methods":["hybrid"],"includeCitations":true,"includeTrace":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["retrievalId"], "701");
    assert_eq!(body["hits"][0]["chunkId"], "11");
    let request = service.last_retrieval_request().unwrap();
    assert_eq!(request.tenant_id, 100001);
    assert_eq!(request.actor_id, Some(30001));
}

#[tokio::test]
async fn context_pack_route_calls_injected_retrieval_service() {
    let service = RecordingRetrievalService::default();
    let app = build_router_with_retrieval_service(service.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/v3/api/knowledge/context_packs")
                .header("content-type", "application/json")
                .extension(app_request_context())
                .body(Body::from(
                    r#"{"actorId":"30001","query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"contextBudgetTokens":80,"includeCitations":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["contextPackId"], "801");
    assert_eq!(body["fragments"][0]["chunkId"], "11");
    assert_eq!(body["estimatedTokens"], 8);
    let request = service.last_context_pack_request().unwrap();
    assert_eq!(request.tenant_id, 100001);
    assert_eq!(request.actor_id, Some(30001));
}

#[tokio::test]
async fn retrieval_route_maps_service_validation_errors_to_problem_details() {
    let app = build_router_with_retrieval_service(FailingRetrievalService);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/v3/api/knowledge/retrievals")
                .header("content-type", "application/json")
                .extension(app_request_context())
                .body(Body::from(
                    r#"{"query":"","bindings":[],"includeCitations":true,"includeTrace":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let body = response_json(response).await;
    assert_eq!(body["code"].as_i64(), Some(40001));
    assert_eq!(body["status"], 400);
}

#[tokio::test]
async fn retrieval_retrieve_route_uses_tenant_from_app_request_context() {
    let service = RecordingRetrievalService::default();
    let app = build_router_with_retrieval_service(service.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/knowledge/retrievals/701")
                .extension(KnowledgeAppRequestContext {
                    tenant_id: 100001,
                    actor_id: Some(30001),
                    organization_id: None,
                    session_id: None,
                    request_id: "test-request-retrieval-route".to_string(),
                    trace_id: None,
                    idempotency_key: None,
                })
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["retrievalId"], "701");
    assert_eq!(service.retrieve_requests(), vec![(100001, 701)]);
}

#[tokio::test]
async fn retrieval_retrieve_route_rejects_missing_app_request_context() {
    let app = build_router_with_retrieval_service(RecordingRetrievalService::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/app/v3/api/knowledge/retrievals/701")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/problem+json"
    );
    let body = response_json(response).await;
    assert_eq!(body["code"].as_i64(), Some(40101));
}

#[derive(Clone, Default)]
struct RecordingRetrievalService {
    retrieval_requests: std::sync::Arc<Mutex<Vec<KnowledgeRetrievalRequest>>>,
    retrieve_requests: std::sync::Arc<Mutex<Vec<(u64, u64)>>>,
    context_pack_requests: std::sync::Arc<Mutex<Vec<KnowledgeContextPackRequest>>>,
}

impl RecordingRetrievalService {
    fn last_retrieval_request(&self) -> Option<KnowledgeRetrievalRequest> {
        self.retrieval_requests.lock().unwrap().last().cloned()
    }

    fn retrieve_requests(&self) -> Vec<(u64, u64)> {
        self.retrieve_requests.lock().unwrap().clone()
    }

    fn last_context_pack_request(&self) -> Option<KnowledgeContextPackRequest> {
        self.context_pack_requests.lock().unwrap().last().cloned()
    }
}

#[async_trait]
impl KnowledgeRetrievalAppService for RecordingRetrievalService {
    async fn retrieve(
        &self,
        _context: KnowledgeAppRequestContext,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.retrieval_requests.lock().unwrap().push(request);
        Ok(KnowledgeRetrievalResult {
            retrieval_id: 701,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 701,
                status: "succeeded".to_string(),
                latency_ms: Some(9),
                result_count: 1,
            }),
            hits: vec![
                sdkwork_knowledgebase_contract::rag::KnowledgeContextFragment {
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
                },
            ],
        })
    }

    async fn retrieve_retrieval(
        &self,
        context: KnowledgeAppRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.retrieve_requests
            .lock()
            .unwrap()
            .push((context.tenant_id, retrieval_id));
        Ok(KnowledgeRetrievalResult {
            retrieval_id,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: retrieval_id,
                status: "succeeded".to_string(),
                latency_ms: Some(9),
                result_count: 1,
            }),
            hits: vec![
                sdkwork_knowledgebase_contract::rag::KnowledgeContextFragment {
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
                },
            ],
        })
    }

    async fn create_context_pack(
        &self,
        _context: KnowledgeAppRequestContext,
        request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack> {
        self.context_pack_requests.lock().unwrap().push(request);
        Ok(KnowledgeContextPack {
            context_pack_id: 801,
            retrieval_id: Some(701),
            query: "enterprise renewal support".to_string(),
            fragments: vec![
                sdkwork_knowledgebase_contract::rag::KnowledgeContextFragment {
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
                },
            ],
            memory_fragments: vec![],
            estimated_tokens: 8,
            citations: vec![],
            truncated: false,
        })
    }
}

struct FailingRetrievalService;

#[async_trait]
impl KnowledgeRetrievalAppService for FailingRetrievalService {
    async fn retrieve(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_knowledge_retrieval_request",
            "query is required",
        ))
    }

    async fn retrieve_retrieval(
        &self,
        _context: KnowledgeAppRequestContext,
        _retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "knowledge_retrieval_not_found",
            "retrieval trace was not found",
        ))
    }

    async fn create_context_pack(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeContextPackRequest,
    ) -> ApiResult<KnowledgeContextPack> {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_knowledge_context_pack_request",
            "context budget is required",
        ))
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    sdkwork_knowledgebase_test_support::api_envelope::unwrap_payload_or_envelope(&value)
}
