use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::{Method, StatusCode};
use sdkwork_knowledgebase_contract::provider_binding::{
    KnowledgeEngineDataScope, KnowledgeEngineExecutionContext,
};
use sdkwork_knowledgebase_provider_runtime::{
    ProviderErrorCategory, ProviderExecutionContext, ProviderHttpRequest, ProviderOperation,
    ProviderRuntime, ProviderRuntimeConfig, ProviderTargetPolicy, ProviderTelemetry,
    ProviderTelemetryEvent,
};
use sdkwork_utils_rust::SDKWORK_TRACE_ID_HEADER;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn test_config(server: &MockServer) -> ProviderRuntimeConfig {
    // Wiremock fixtures run on loopback; the provider SSRF protection fails closed
    // unless the runtime config explicitly opts into private-network targets. This is
    // the same flag a self-hosted engine deployment sets in production.
    let mut config = ProviderRuntimeConfig::for_base_url_with_policy(
        &server.uri(),
        ProviderTargetPolicy::Development,
    )
    .expect("test runtime config");
    config.allow_private_network_targets = true;
    config.connect_timeout = Duration::from_millis(100);
    config.request_timeout = Duration::from_secs(1);
    config.retry_base_delay = Duration::from_millis(1);
    config.retry_max_delay = Duration::from_millis(5);
    config.max_attempts = 2;
    config
}

fn context() -> ProviderExecutionContext {
    ProviderExecutionContext::from_knowledge_engine(
        &knowledge_engine_context(Some(73), vec![42]),
        "engine.knowledge.external.test",
        ProviderOperation::Search,
    )
    .expect("valid Provider execution context")
}

fn knowledge_engine_context(
    binding_id: Option<u64>,
    allowed_space_ids: Vec<u64>,
) -> KnowledgeEngineExecutionContext {
    let deadline_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_millis()
        + 60_000;
    KnowledgeEngineExecutionContext {
        tenant_id: 11,
        organization_id: 12,
        actor_id: "actor-provider-test".to_string(),
        permission_scope: vec!["knowledge.read".to_string()],
        data_scope: KnowledgeEngineDataScope {
            allowed_space_ids,
            allowed_source_ids: Vec::new(),
            allowed_document_ids: Vec::new(),
        },
        space_id: 42,
        binding_id,
        trace_id: "trace-provider-001".to_string(),
        deadline_unix_ms: u64::try_from(deadline_unix_ms).expect("deadline fits u64"),
    }
}

fn get_request(server: &MockServer, operation: ProviderOperation) -> ProviderHttpRequest {
    ProviderHttpRequest::new(operation, Method::GET, format!("{}/resource", server.uri()))
        .expect("request")
        .idempotent(true)
}

#[tokio::test]
async fn runtime_fails_closed_on_loopback_without_private_target_opt_in() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    // The loopback wiremock target must fail closed with the typed InvalidTarget
    // error unless the runtime config opts into private-network targets.
    let mut config = test_config(&server);
    config.allow_private_network_targets = false;
    let runtime = ProviderRuntime::new(config).expect("runtime");

    let error = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Health))
        .await
        .expect_err("loopback target must fail closed");

    assert_eq!(error.category, ProviderErrorCategory::InvalidTarget);
    assert_eq!(server.received_requests().await.expect("requests").len(), 0);
}

#[tokio::test]
async fn runtime_propagates_trace_and_decodes_bounded_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .and(header(SDKWORK_TRACE_ID_HEADER, "trace-provider-001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })))
        .expect(1)
        .mount(&server)
        .await;
    let runtime = ProviderRuntime::new(test_config(&server)).expect("runtime");

    let response = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Health))
        .await
        .expect("execute");
    let json: serde_json::Value = response.json().expect("json");

    assert_eq!(json["ok"], true);
}

#[tokio::test]
async fn runtime_rejects_out_of_scope_space_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let runtime = ProviderRuntime::new(test_config(&server)).expect("runtime");
    let mut context = context();
    context.data_scope.allowed_space_ids = vec![99];
    let error = runtime
        .execute(&context, get_request(&server, ProviderOperation::Search))
        .await
        .expect_err("out-of-scope space must fail");

    assert_eq!(error.category, ProviderErrorCategory::PermissionDenied);
}

#[test]
fn execution_context_requires_binding_and_live_deadline() {
    let missing_binding = ProviderExecutionContext::from_knowledge_engine(
        &knowledge_engine_context(None, vec![42]),
        "engine.knowledge.external.test",
        ProviderOperation::Read,
    )
    .expect_err("binding is required");
    assert_eq!(missing_binding.category, ProviderErrorCategory::Validation);

    let mut expired = knowledge_engine_context(Some(73), vec![42]);
    expired.deadline_unix_ms = 1;
    let expired = ProviderExecutionContext::from_knowledge_engine(
        &expired,
        "engine.knowledge.external.test",
        ProviderOperation::Search,
    )
    .expect_err("deadline must be live");
    assert_eq!(expired.category, ProviderErrorCategory::Timeout);
}

#[test]
fn request_scope_must_match_authenticated_execution_context() {
    for (tenant_id, space_id) in [(99, 42), (11, 99), (0, 42), (11, 0)] {
        let error = ProviderExecutionContext::from_knowledge_engine_request(
            &knowledge_engine_context(Some(73), vec![42]),
            "engine.knowledge.external.test",
            ProviderOperation::Search,
            tenant_id,
            space_id,
        )
        .expect_err("request scope mismatch must fail before Provider execution");

        assert_eq!(error.category, ProviderErrorCategory::PermissionDenied);
        assert_eq!(
            error.safe_message,
            "Provider request scope does not match the execution context"
        );
    }
}

#[test]
fn request_scope_conversion_preserves_authenticated_context() {
    let context = ProviderExecutionContext::from_knowledge_engine_request(
        &knowledge_engine_context(Some(73), vec![42]),
        "engine.knowledge.external.test",
        ProviderOperation::Read,
        11,
        42,
    )
    .expect("matching request scope");

    assert_eq!(context.tenant_id, 11);
    assert_eq!(context.organization_id, 12);
    assert_eq!(context.actor_id, "actor-provider-test");
    assert_eq!(context.space_id, 42);
    assert_eq!(context.binding_id.as_deref(), Some("73"));
    assert_eq!(context.trace_id, "trace-provider-001");
}

#[derive(Clone)]
struct FailThenSucceed {
    calls: Arc<AtomicUsize>,
}

impl Respond for FailThenSucceed {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            ResponseTemplate::new(503)
        } else {
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "retried": true }))
        }
    }
}

#[tokio::test]
async fn runtime_retries_only_idempotent_retryable_operations() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(FailThenSucceed {
            calls: calls.clone(),
        })
        .mount(&server)
        .await;
    let runtime = ProviderRuntime::new(test_config(&server)).expect("runtime");

    let response = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Search))
        .await
        .expect("retry succeeds");

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn runtime_preserves_retry_after_and_rate_limit_category() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "2"))
        .mount(&server)
        .await;
    let mut config = test_config(&server);
    config.max_attempts = 1;
    let runtime = ProviderRuntime::new(config).expect("runtime");

    let error = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Search))
        .await
        .expect_err("rate limit");

    assert_eq!(error.category, ProviderErrorCategory::RateLimited);
    assert_eq!(error.retry_after, Some(Duration::from_secs(2)));
    assert!(error.retryable);
}

#[tokio::test]
async fn runtime_rejects_oversized_streamed_responses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"12345"))
        .mount(&server)
        .await;
    let mut config = test_config(&server);
    config.max_response_bytes = 4;
    let runtime = ProviderRuntime::new(config).expect("runtime");

    let error = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Read))
        .await
        .expect_err("body limit");

    assert_eq!(error.category, ProviderErrorCategory::ResponseTooLarge);
    assert!(!error.retryable);
}

#[tokio::test]
async fn runtime_enforces_operation_deadline() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(100)))
        .mount(&server)
        .await;
    let mut config = test_config(&server);
    config.max_attempts = 1;
    let runtime = ProviderRuntime::new(config).expect("runtime");
    let mut execution_context = context();
    execution_context.deadline = Some(Instant::now() + Duration::from_millis(10));

    let error = runtime
        .execute(
            &execution_context,
            get_request(&server, ProviderOperation::Read),
        )
        .await
        .expect_err("deadline");

    assert_eq!(error.category, ProviderErrorCategory::Timeout);
}

#[tokio::test]
async fn runtime_opens_circuit_after_bounded_failures() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let mut config = test_config(&server);
    config.max_attempts = 1;
    config.circuit_failure_threshold = 2;
    let runtime = ProviderRuntime::new(config).expect("runtime");

    for _ in 0..2 {
        let error = runtime
            .execute(&context(), get_request(&server, ProviderOperation::Health))
            .await
            .expect_err("unavailable");
        assert_eq!(error.category, ProviderErrorCategory::Unavailable);
    }
    let error = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Health))
        .await
        .expect_err("circuit open");

    assert_eq!(error.category, ProviderErrorCategory::CircuitOpen);
    assert_eq!(server.received_requests().await.expect("requests").len(), 2);
}

#[tokio::test]
async fn runtime_bulkhead_fails_fast_when_capacity_is_saturated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(100)))
        .mount(&server)
        .await;
    let mut config = test_config(&server);
    config.max_concurrency = 1;
    let runtime = ProviderRuntime::new(config).expect("runtime");
    let first_runtime = runtime.clone();
    let first_context = context();
    let first_request = get_request(&server, ProviderOperation::Read);
    let first =
        tokio::spawn(async move { first_runtime.execute(&first_context, first_request).await });
    tokio::time::sleep(Duration::from_millis(10)).await;

    let error = runtime
        .execute(&context(), get_request(&server, ProviderOperation::Read))
        .await
        .expect_err("bulkhead");

    assert_eq!(error.category, ProviderErrorCategory::BulkheadSaturated);
    first.await.expect("join").expect("first request");
}

#[tokio::test]
async fn runtime_enforces_https_and_exact_configured_origin() {
    let production_error = ProviderRuntimeConfig::for_base_url_with_policy(
        "http://provider.example",
        ProviderTargetPolicy::Production,
    )
    .expect_err("production requires https");
    assert_eq!(
        production_error.category,
        ProviderErrorCategory::InvalidTarget
    );

    let allowed = MockServer::start().await;
    let other = MockServer::start().await;
    let runtime = ProviderRuntime::new(test_config(&allowed)).expect("runtime");
    let error = runtime
        .execute(&context(), get_request(&other, ProviderOperation::Health))
        .await
        .expect_err("origin mismatch");

    assert_eq!(error.category, ProviderErrorCategory::InvalidTarget);
}

#[tokio::test]
async fn runtime_maps_auth_permission_and_not_found_without_retry() {
    for (status, category) in [
        (401, ProviderErrorCategory::Authentication),
        (403, ProviderErrorCategory::PermissionDenied),
        (404, ProviderErrorCategory::NotFound),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/resource"))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
        let runtime = ProviderRuntime::new(test_config(&server)).expect("runtime");

        let error = runtime
            .execute(&context(), get_request(&server, ProviderOperation::Read))
            .await
            .expect_err("status error");

        assert_eq!(error.category, category);
        assert!(!error.retryable);
    }
}

#[derive(Default)]
struct RecordingTelemetry {
    events: Mutex<Vec<ProviderTelemetryEvent>>,
}

impl ProviderTelemetry for RecordingTelemetry {
    fn record(&self, event: ProviderTelemetryEvent) {
        self.events.lock().expect("events mutex").push(event);
    }
}

#[tokio::test]
async fn runtime_records_bounded_operation_telemetry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok"))
        .mount(&server)
        .await;
    let telemetry = Arc::new(RecordingTelemetry::default());
    let runtime = ProviderRuntime::new(test_config(&server))
        .expect("runtime")
        .with_telemetry(telemetry.clone());

    runtime
        .execute(&context(), get_request(&server, ProviderOperation::Health))
        .await
        .expect("execute");

    let events = telemetry.events.lock().expect("events mutex");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].operation, ProviderOperation::Health);
    assert_eq!(events[0].attempts, 1);
    assert_eq!(events[0].response_bytes, 2);
    assert!(events[0].error_category.is_none());
}

#[tokio::test]
async fn runtime_never_exposes_upstream_error_body_or_sensitive_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/resource"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream-secret-body"))
        .mount(&server)
        .await;
    let mut config = test_config(&server);
    config.max_attempts = 1;
    let runtime = ProviderRuntime::new(config).expect("runtime");
    let request = get_request(&server, ProviderOperation::Read)
        .bearer_auth("credential-secret")
        .expect("auth header");

    let error = runtime
        .execute(&context(), request)
        .await
        .expect_err("upstream error");
    let rendered = error.to_string();

    assert!(!rendered.contains("upstream-secret-body"));
    assert!(!rendered.contains("credential-secret"));
}
