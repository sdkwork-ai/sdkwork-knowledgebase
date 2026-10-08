use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineHealthStatus, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_flowise::{
    chunk_id_from_content, FlowiseConnectorConfig, FlowiseKnowledgeEngine,
};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn assert_flowise_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/document-store/store/health-store"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = FlowiseKnowledgeEngine::with_config(FlowiseConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("health-key".to_string()),
        allow_private_network: true,
        default_store_id: Some("health-store".to_string()),
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn flowise_health_maps_upstream_availability() {
    assert_flowise_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_flowise_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}

#[tokio::test]
async fn flowise_search_uses_configured_remote_resource_id() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/document-store/vectorstore/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "timeTaken": 12,
            "docs": [{
                "pageContent": "space scoped answer",
                "metadata": {
                    "source": "Space Doc",
                    "url": "file://space-doc.txt"
                }
            }]
        })))
        .mount(&mock_server)
        .await;

    let config = FlowiseConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_store_id: Some("603a7b51-ae7c-4b0a-8865-e454ed2f6766".to_string()),
    };
    let engine = FlowiseKnowledgeEngine::with_config(config);

    let result = engine
        .search(
            &provider_execution_context(1, 2, 42, 7, "trace-adapter-search"),
            KnowledgeEngineSearchRequest {
                tenant_id: 1,
                space_id: 42,
                query: "hello".to_string(),
                top_k: 3,
            },
        )
        .await
        .expect("search");

    assert_eq!(result.hits.len(), 1);
    let chunk_id = chunk_id_from_content("space scoped answer");
    assert_eq!(
        result.hits[0].document.document_id,
        format!("42/Space Doc#{chunk_id}")
    );
    assert_eq!(result.hits[0].snippet, "space scoped answer");
}

#[tokio::test]
async fn flowise_read_document_resolves_chunk_from_vector_query() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/document-store/vectorstore/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "timeTaken": 8,
            "docs": [{
                "pageContent": "full chunk body",
                "metadata": {
                    "source": "Space Doc",
                    "url": "file://space-doc.txt"
                }
            }]
        })))
        .mount(&mock_server)
        .await;

    let config = FlowiseConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_store_id: Some("603a7b51-ae7c-4b0a-8865-e454ed2f6766".to_string()),
    };
    let engine = FlowiseKnowledgeEngine::with_config(config);
    let chunk_id = chunk_id_from_content("full chunk body");

    let document = engine
        .read_document(
            &provider_execution_context(1, 2, 42, 7, "trace-adapter-read"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 42,
                document_id: format!("Space Doc#{chunk_id}"),
            },
        )
        .await
        .expect("read");

    assert_eq!(document.title, "Space Doc");
    assert_eq!(document.content, "full chunk body");
    assert_eq!(document.document_id, format!("Space Doc#{chunk_id}"));
}
