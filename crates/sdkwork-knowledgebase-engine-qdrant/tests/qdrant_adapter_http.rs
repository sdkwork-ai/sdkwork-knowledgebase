use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineError, KnowledgeEngineHealthStatus, KnowledgeEngineListRequest,
    KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_qdrant::{QdrantConnectorConfig, QdrantKnowledgeEngine};
use sdkwork_knowledgebase_test_support::provider_execution::{
    knowledge_execution_context, provider_execution_context,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn qdrant_search_uses_configured_remote_resource_id() {
    let mock_server = MockServer::start().await;
    let collection_name = "policies";
    Mock::given(method("POST"))
        .and(path(format!("/collections/{collection_name}/points/query")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "points": [{
                    "id": "pt-9",
                    "score": 0.88,
                    "payload": {
                        "title": "Space Doc",
                        "text": "space scoped answer",
                        "source": "file://space-doc.txt"
                    }
                }]
            },
            "status": "ok"
        })))
        .mount(&mock_server)
        .await;

    let config = QdrantConnectorConfig {
        base_url: mock_server.uri(),
        api_key: Some(zeroize::Zeroizing::new("test-api-key".to_string())),
        allow_private_network: true,
        default_collection_name: Some(collection_name.to_string()),
        query_model: Some("sentence-transformers/all-minilm-l6-v2".to_string()),
        using_vector: None,
    };
    let engine = QdrantKnowledgeEngine::with_config(config);

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
    assert_eq!(result.hits[0].document.document_id, "42/Space Doc#pt-9");
    assert_eq!(result.hits[0].snippet, "space scoped answer");
}

#[tokio::test]
async fn qdrant_read_document_fetches_point_by_id() {
    let mock_server = MockServer::start().await;
    let collection_name = "policies";
    Mock::given(method("POST"))
        .and(path(format!("/collections/{collection_name}/points")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{
                "id": "pt-9",
                "payload": {
                    "title": "Space Doc",
                    "text": "full point body",
                    "source": "file://space-doc.txt"
                }
            }],
            "status": "ok"
        })))
        .mount(&mock_server)
        .await;

    let config = QdrantConnectorConfig {
        base_url: mock_server.uri(),
        api_key: Some(zeroize::Zeroizing::new("test-api-key".to_string())),
        allow_private_network: true,
        default_collection_name: Some(collection_name.to_string()),
        query_model: Some("sentence-transformers/all-minilm-l6-v2".to_string()),
        using_vector: None,
    };
    let engine = QdrantKnowledgeEngine::with_config(config);

    let document = engine
        .read_document(
            &provider_execution_context(1, 2, 42, 7, "trace-adapter-read"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 42,
                document_id: "Space Doc#pt-9".to_string(),
            },
        )
        .await
        .expect("read");

    assert_eq!(document.title, "Space Doc");
    assert_eq!(document.content, "full point body");
    assert_eq!(document.document_id, "Space Doc#pt-9");
}

#[tokio::test]
async fn qdrant_list_documents_is_explicitly_unsupported() {
    let collection_name = "policies";
    let config = QdrantConnectorConfig {
        base_url: "http://localhost:6333".to_string(),
        api_key: None,
        allow_private_network: true,
        default_collection_name: Some(collection_name.to_string()),
        query_model: Some("sentence-transformers/all-minilm-l6-v2".to_string()),
        using_vector: None,
    };
    let engine = QdrantKnowledgeEngine::with_config(config);

    let error = engine
        .list_documents(
            &knowledge_execution_context(1, 1, 42, None, "trace-qdrant-list"),
            KnowledgeEngineListRequest {
                tenant_id: 1,
                space_id: 42,
                limit: 10,
            },
        )
        .await
        .expect_err("list_documents must not synthesize collection descriptors");

    assert!(matches!(error, KnowledgeEngineError::Unsupported(_)));
}

async fn assert_qdrant_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/collections/health-collection"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = QdrantKnowledgeEngine::with_config(QdrantConnectorConfig {
        base_url: mock_server.uri(),
        api_key: None,
        allow_private_network: true,
        default_collection_name: Some("health-collection".to_string()),
        query_model: Some("health-model".to_string()),
        using_vector: None,
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn qdrant_health_maps_upstream_availability() {
    assert_qdrant_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_qdrant_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}
