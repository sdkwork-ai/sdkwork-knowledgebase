use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineHealthStatus, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_ragflow::{
    RagflowConnectorConfig, RagflowKnowledgeEngine, RAGFLOW_IMPLEMENTATION_ID,
};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn assert_ragflow_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/datasets"))
        .respond_with(
            ResponseTemplate::new(upstream_status)
                .set_body_json(serde_json::json!({ "code": 0, "data": {} })),
        )
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = RagflowKnowledgeEngine::with_config(RagflowConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("health-key".to_string()),
        allow_private_network: true,
        default_dataset_id: Some("health-dataset".to_string()),
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn ragflow_health_maps_upstream_availability() {
    assert_ragflow_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_ragflow_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}

#[tokio::test]
async fn ragflow_search_uses_configured_remote_resource_id() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/retrieval"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0,
            "data": {
                "chunks": [{
                    "id": "chunk-9",
                    "content": "space scoped ragflow answer",
                    "document_id": "doc-42",
                    "document_keyword": "Space Doc",
                    "similarity": 0.91
                }]
            }
        })))
        .mount(&mock_server)
        .await;

    let config = RagflowConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_dataset_id: Some("ds-space-42".to_string()),
    };
    let engine = RagflowKnowledgeEngine::with_config(config);

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

    assert_eq!(result.implementation_id, RAGFLOW_IMPLEMENTATION_ID);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].document.document_id, "42/doc-42#chunk-9");
    assert_eq!(result.hits[0].snippet, "space scoped ragflow answer");
}
