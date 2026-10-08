use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use sdkwork_knowledgebase_contract::agent_chat::{
    KnowledgeAgentChatRequest, KnowledgeAgentChatResponse, KnowledgeAgentKnowledgeMode,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentBindingList, KnowledgeAgentBindingRequest,
    KnowledgeAgentProfile, KnowledgeAgentProfileRequest, KnowledgeAgentStatus,
    KnowledgeContextFragment, KnowledgeRetrievalMethod, KnowledgeRetrievalRequest,
    KnowledgeRetrievalResult, KnowledgeRetrievalTrace,
};
use sdkwork_routes_knowledgebase_app_api::{
    build_router_with_agent_and_retrieval_services, build_router_with_agent_service, ApiResult,
    KnowledgeAgentAppService, KnowledgeAppRequestContext, KnowledgeRetrievalAppService,
};
use serde_json::Value;
use std::sync::Mutex;
use tower::util::ServiceExt;

#[tokio::test]
async fn agent_profile_routes_call_injected_service() {
    let service = RecordingAgentService::default();
    let app = build_router_with_agent_service(service);

    let create_response = app
        .clone()
        .oneshot(request(
            "POST",
            "/app/v3/api/knowledge/agent_profiles",
            profile_body("Support Agent"),
        ))
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);
    assert_eq!(
        response_json(create_response).await["modelProviderId"],
        "provider.model.openai"
    );

    let binding_response = app
        .clone()
        .oneshot(request(
            "POST",
            "/app/v3/api/knowledge/agent_profiles/501/bindings",
            binding_body(501, 7, true),
        ))
        .await
        .unwrap();
    assert_eq!(binding_response.status(), StatusCode::OK);
    assert_eq!(response_json(binding_response).await["spaceId"], "7");

    let list_response = app
        .clone()
        .oneshot(request(
            "GET",
            "/app/v3/api/knowledge/agent_profiles/501/bindings",
            "",
        ))
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    assert_eq!(
        response_json(list_response).await["items"][0]["bindingId"],
        "601"
    );

    let delete_response = app
        .clone()
        .oneshot(request(
            "DELETE",
            "/app/v3/api/knowledge/agent_profiles/501/bindings/601",
            "",
        ))
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn agent_retrieval_preview_route_calls_injected_service() {
    let service = RecordingAgentService::default();
    let app = build_router_with_agent_service(service);

    let response = app
        .oneshot(request(
            "POST",
            "/app/v3/api/knowledge/agent_profiles/501/retrieval_preview",
            r#"{"tenantId":"100001","actorId":"30001","query":"enterprise renewal support","bindings":[],"includeCitations":true,"includeTrace":true}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["retrievalId"], "701");
    assert_eq!(body["hits"][0]["chunkId"], "11");
}

#[tokio::test]
async fn agent_chat_route_calls_injected_service() {
    let service = RecordingAgentService::default();
    let app = build_router_with_agent_service(service);

    let response = app
        .oneshot(request(
            "POST",
            "/app/v3/api/knowledge/agent_profiles/501/chat",
            r#"{"tenantId":"100001","message":"What is enterprise renewal support?","mode":"okf_bundle"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["mode"], "okf_bundle");
    assert_eq!(body["citations"][0]["title"], "Support Playbook");
}

#[tokio::test]
async fn combined_router_serves_retrieval_and_agent_routes_together() {
    let app = build_router_with_agent_and_retrieval_services(
        RecordingAgentService::default(),
        RecordingRetrievalService,
    );

    let retrieval_response = app
        .clone()
        .oneshot(request(
            "POST",
            "/app/v3/api/knowledge/retrievals",
            r#"{"tenantId":"100001","actorId":"30001","query":"enterprise renewal support","bindings":[{"spaceId":"7","priority":10}],"methods":["hybrid"],"includeCitations":true,"includeTrace":true}"#,
        ))
        .await
        .unwrap();
    assert_eq!(retrieval_response.status(), StatusCode::CREATED);
    assert_eq!(
        response_json(retrieval_response).await["retrievalId"],
        "702"
    );

    let preview_response = app
        .oneshot(request(
            "POST",
            "/app/v3/api/knowledge/agent_profiles/501/retrieval_preview",
            r#"{"tenantId":"100001","actorId":"30001","query":"enterprise renewal support","bindings":[],"includeCitations":true,"includeTrace":true}"#,
        ))
        .await
        .unwrap();
    assert_eq!(preview_response.status(), StatusCode::OK);
    assert_eq!(response_json(preview_response).await["retrievalId"], "701");
}

#[derive(Default)]
struct RecordingAgentService {
    profile_requests: Mutex<Vec<KnowledgeAgentProfileRequest>>,
    binding_requests: Mutex<Vec<KnowledgeAgentBindingRequest>>,
    preview_requests: Mutex<Vec<KnowledgeRetrievalRequest>>,
    chat_requests: Mutex<Vec<KnowledgeAgentChatRequest>>,
}

#[async_trait]
impl KnowledgeAgentAppService for RecordingAgentService {
    async fn create_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        self.profile_requests.lock().unwrap().push(request);
        Ok(profile())
    }

    async fn retrieve_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
    ) -> ApiResult<KnowledgeAgentProfile> {
        Ok(profile())
    }

    async fn update_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> ApiResult<KnowledgeAgentProfile> {
        self.profile_requests.lock().unwrap().push(request);
        Ok(profile())
    }

    async fn delete_profile(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn list_bindings(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
    ) -> ApiResult<KnowledgeAgentBindingList> {
        Ok(KnowledgeAgentBindingList {
            items: vec![binding(601, 501, 7, true)],
        })
    }

    async fn create_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        self.binding_requests.lock().unwrap().push(request);
        Ok(binding(601, 501, 7, true))
    }

    async fn update_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> ApiResult<KnowledgeAgentBinding> {
        self.binding_requests.lock().unwrap().push(request);
        Ok(binding(binding_id, 501, 7, true))
    }

    async fn delete_binding(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        _binding_id: u64,
    ) -> ApiResult<()> {
        Ok(())
    }

    async fn preview_retrieval(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        self.preview_requests.lock().unwrap().push(request);
        Ok(KnowledgeRetrievalResult {
            retrieval_id: 701,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 701,
                status: "succeeded".to_string(),
                latency_ms: Some(9),
                result_count: 1,
            }),
            hits: vec![KnowledgeContextFragment {
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
            }],
        })
    }

    async fn create_agent_chat(
        &self,
        _context: KnowledgeAppRequestContext,
        _profile_id: u64,
        request: KnowledgeAgentChatRequest,
    ) -> ApiResult<KnowledgeAgentChatResponse> {
        self.chat_requests.lock().unwrap().push(request);
        Ok(KnowledgeAgentChatResponse {
            chat_id: "chat.1".to_string(),
            answer: "enterprise renewal support answer".to_string(),
            mode: KnowledgeAgentKnowledgeMode::OkfBundle,
            agent_implementation_id: sdkwork_knowledgebase_contract::RIG_AGENT_IMPLEMENTATION_ID
                .to_string(),
            model_provider_id: "provider.model.rig-rust".to_string(),
            model_id: "rig.default-chat".to_string(),
            citations: vec![
                sdkwork_knowledgebase_contract::agent_chat::KnowledgeAgentChatCitation {
                    document_id: None,
                    concept_id: Some("tables/users".to_string()),
                    title: "Support Playbook".to_string(),
                    source_uri: Some("okf/playbooks/support-playbook.md".to_string()),
                    logical_path: Some("okf/playbooks/support-playbook.md".to_string()),
                    locator: None,
                    score: Some(0.91),
                    snippet: Some("enterprise renewal support answer".to_string()),
                },
            ],
            retrieval_id: None,
            session_id: None,
        })
    }
}

struct RecordingRetrievalService;

#[async_trait]
impl KnowledgeRetrievalAppService for RecordingRetrievalService {
    async fn retrieve(
        &self,
        _context: KnowledgeAppRequestContext,
        _request: KnowledgeRetrievalRequest,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Ok(KnowledgeRetrievalResult {
            retrieval_id: 702,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 702,
                status: "succeeded".to_string(),
                latency_ms: Some(8),
                result_count: 1,
            }),
            hits: vec![KnowledgeContextFragment {
                chunk_id: 12,
                document_id: 102,
                document_version_id: Some(202),
                space_id: 7,
                title: "Renewal Notes".to_string(),
                content: "enterprise renewal support notes".to_string(),
                score: Some(0.89),
                rank: 1,
                token_count: Some(7),
                retrieval_method: KnowledgeRetrievalMethod::Hybrid,
                citation: None,
            }],
        })
    }

    async fn retrieve_retrieval(
        &self,
        _context: KnowledgeAppRequestContext,
        retrieval_id: u64,
    ) -> ApiResult<KnowledgeRetrievalResult> {
        Ok(KnowledgeRetrievalResult {
            retrieval_id,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: retrieval_id,
                status: "succeeded".to_string(),
                latency_ms: Some(8),
                result_count: 0,
            }),
            hits: vec![],
        })
    }

    async fn create_context_pack(
        &self,
        _context: KnowledgeAppRequestContext,
        request: sdkwork_knowledgebase_contract::rag::KnowledgeContextPackRequest,
    ) -> ApiResult<sdkwork_knowledgebase_contract::rag::KnowledgeContextPack> {
        Ok(sdkwork_knowledgebase_contract::rag::KnowledgeContextPack {
            context_pack_id: 802,
            retrieval_id: Some(702),
            query: request.query,
            fragments: vec![],
            memory_fragments: vec![],
            estimated_tokens: 0,
            citations: vec![],
            truncated: false,
        })
    }
}

fn request(method: &str, uri: &str, body: impl Into<String>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .extension(KnowledgeAppRequestContext {
            tenant_id: 100001,
            actor_id: Some(30001),
            organization_id: None,
            session_id: None,
            request_id: "test-request-agent-routes".to_string(),
            trace_id: None,
            idempotency_key: None,
        })
        .body(Body::from(body.into()))
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    sdkwork_knowledgebase_test_support::api_envelope::unwrap_payload_or_envelope(&value)
}

fn profile_body(name: &str) -> String {
    format!(
        r#"{{"tenantId":"100001","name":"{name}","description":"Support KB","systemInstruction":"Answer with citations.","modelProviderId":"provider.model.openai","modelId":"gpt-4.1","retrievalProfileId":"31","status":"active"}}"#
    )
}

fn binding_body(profile_id: u64, space_id: u64, enabled: bool) -> String {
    format!(
        r#"{{"tenantId":"100001","profileId":"{profile_id}","spaceId":"{space_id}","priority":20,"topK":3,"minScore":0.75,"enabled":{enabled}}}"#
    )
}

fn profile() -> KnowledgeAgentProfile {
    KnowledgeAgentProfile {
        profile_id: 501,
        tenant_id: 100001,
        name: "Support Agent".to_string(),
        description: Some("Support KB".to_string()),
        system_instruction: "Answer with citations.".to_string(),
        model_provider_id: "provider.model.openai".to_string(),
        model_id: "gpt-4.1".to_string(),
        model_parameters: None,
        retrieval_profile_id: Some(31),
        citation_policy: None,
        memory_policy_ref: None,
        tool_policy_ref: None,
        answer_policy: None,
        status: KnowledgeAgentStatus::Active,
        knowledge_mode: Default::default(),
        agent_implementation_id: sdkwork_knowledgebase_contract::default_agent_implementation_id(),
        bindings: vec![binding(601, 501, 7, true)],
    }
}

fn binding(
    binding_id: u64,
    profile_id: u64,
    space_id: u64,
    enabled: bool,
) -> KnowledgeAgentBinding {
    KnowledgeAgentBinding {
        binding_id,
        profile_id,
        tenant_id: 100001,
        space_id,
        source_filter: None,
        document_filter: None,
        priority: 20,
        top_k: Some(3),
        min_score: Some(0.75),
        enabled,
    }
}
