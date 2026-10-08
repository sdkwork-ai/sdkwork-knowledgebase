use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineHealthStatus, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_open_webui::{
    chunk_id_from_content, OpenWebuiConnectorConfig, OpenWebuiKnowledgeEngine,
};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn assert_open_webui_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/knowledge/health-knowledge"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = OpenWebuiKnowledgeEngine::with_config(OpenWebuiConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("health-key".to_string()),
        allow_private_network: true,
        default_knowledge_id: Some("health-knowledge".to_string()),
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn open_webui_health_maps_upstream_availability() {
    assert_open_webui_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_open_webui_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}

#[tokio::test]
async fn open_webui_search_uses_configured_remote_resource_id() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/retrieval/query/collection"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "distances": [[0.88]],
            "documents": [["space scoped answer"]],
            "metadatas": [[{
                "source": "Space Doc",
                "url": "file://space-doc.txt"
            }]]
        })))
        .mount(&mock_server)
        .await;

    let config = OpenWebuiConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_knowledge_id: Some("kb-space-42".to_string()),
    };
    let engine = OpenWebuiKnowledgeEngine::with_config(config);

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
async fn open_webui_read_document_resolves_chunk_from_query_collection() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/retrieval/query/collection"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "distances": [[0.91]],
            "documents": [["full chunk body"]],
            "metadatas": [[{
                "source": "Space Doc",
                "url": "file://space-doc.txt"
            }]]
        })))
        .mount(&mock_server)
        .await;

    let config = OpenWebuiConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_knowledge_id: Some("kb-space-42".to_string()),
    };
    let engine = OpenWebuiKnowledgeEngine::with_config(config);
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
