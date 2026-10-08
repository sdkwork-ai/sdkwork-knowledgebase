use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineHealthStatus, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_onyx::{OnyxConnectorConfig, OnyxKnowledgeEngine};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn assert_onyx_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = OnyxKnowledgeEngine::with_config(OnyxConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("health-key".to_string()),
        allow_private_network: true,

    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn onyx_health_maps_upstream_availability() {
    assert_onyx_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_onyx_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}

#[tokio::test]
async fn onyx_search_maps_unified_search_results() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "title": "Policy Guide",
                "url": "https://example.com/policy",
                "content": "policy content",
                "source_type": "web"
            }]
        })))
        .mount(&mock_server)
        .await;

    let engine = OnyxKnowledgeEngine::with_config(OnyxConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,

    });

    let result = engine
        .search(
            &provider_execution_context(1, 2, 3, 7, "trace-adapter-search"),
            KnowledgeEngineSearchRequest {
                tenant_id: 1,
                space_id: 3,
                query: "policy".to_string(),
                top_k: 4,
            },
        )
        .await
        .expect("search");

    assert_eq!(result.hits.len(), 1);
    assert_eq!(
        result.hits[0].document.document_id,
        "3/url:https://example.com/policy"
    );
    assert_eq!(result.hits[0].snippet, "policy content");
}

#[tokio::test]
async fn onyx_read_document_uses_open_urls() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "title": "Policy Guide",
                "url": "https://example.com/policy",
                "content": "policy content",
                "source_type": "web"
            }]
        })))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/open_urls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "title": "Policy Guide",
                "content": "full policy body"
            }]
        })))
        .mount(&mock_server)
        .await;

    let engine = OnyxKnowledgeEngine::with_config(OnyxConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,

    });

    // The read allowlist is populated only by this space's search results.
    engine
        .search(
            &provider_execution_context(1, 2, 3, 7, "trace-adapter-search"),
            KnowledgeEngineSearchRequest {
                tenant_id: 1,
                space_id: 3,
                query: "policy".to_string(),
                top_k: 4,
            },
        )
        .await
        .expect("search");

    let document = engine
        .read_document(
            &provider_execution_context(1, 2, 3, 7, "trace-adapter-read"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 3,
                document_id: "url:https://example.com/policy".to_string(),
            },
        )
        .await
        .expect("read");

    assert_eq!(document.title, "Policy Guide");
    assert_eq!(document.content, "full policy body");
}

#[tokio::test]
async fn onyx_read_document_rejects_urls_outside_the_space_search_allowlist() {
    use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;

    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/open_urls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": []
        })))
        .expect(0)
        .mount(&mock_server)
        .await;

    let engine = OnyxKnowledgeEngine::with_config(OnyxConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,

    });

    let error = engine
        .read_document(
            &provider_execution_context(1, 2, 3, 7, "trace-adapter-read-forged"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 3,
                document_id: "url:http://169.254.169.254/latest/meta-data".to_string(),
            },
        )
        .await
        .expect_err("non-https url ids outside search hits must be rejected");

    assert!(matches!(error, KnowledgeEngineError::Validation(_)));
}
