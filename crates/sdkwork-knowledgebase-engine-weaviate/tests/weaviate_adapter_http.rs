use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineError, KnowledgeEngineHealthStatus, KnowledgeEngineListRequest,
    KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_weaviate::{
    WeaviateConnectorConfig, WeaviateKnowledgeEngine, DEFAULT_WEAVIATE_CONTENT_PROPERTY,
    DEFAULT_WEAVIATE_TITLE_PROPERTY,
};
use sdkwork_knowledgebase_test_support::provider_execution::{
    knowledge_execution_context, provider_execution_context,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn weaviate_search_uses_configured_remote_resource_id() {
    let mock_server = MockServer::start().await;
    let class_name = "KnowledgeChunk";
    Mock::given(method("POST"))
        .and(path("/v1/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "Get": {
                    class_name: [{
                        "title": "Space Doc",
                        "content": "space scoped answer",
                        "_additional": {
                            "id": "rec-9",
                            "certainty": 0.88
                        }
                    }]
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let config = WeaviateConnectorConfig {
        base_url: mock_server.uri(),
        api_key: Some(zeroize::Zeroizing::new("test-api-key".to_string())),
        allow_private_network: true,
        default_class_name: Some(class_name.to_string()),
        title_property: DEFAULT_WEAVIATE_TITLE_PROPERTY.to_string(),
        content_property: DEFAULT_WEAVIATE_CONTENT_PROPERTY.to_string(),
    };
    let engine = WeaviateKnowledgeEngine::with_config(config);

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
    assert_eq!(result.hits[0].document.document_id, "42/Space Doc#rec-9");
    assert_eq!(result.hits[0].snippet, "space scoped answer");
}

#[tokio::test]
async fn weaviate_read_document_fetches_object_by_id() {
    let mock_server = MockServer::start().await;
    let class_name = "KnowledgeChunk";
    Mock::given(method("GET"))
        .and(path(format!("/v1/objects/{class_name}/rec-9")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "properties": {
                "title": "Space Doc",
                "content": "full object body",
                "source": "file://space-doc.txt"
            }
        })))
        .mount(&mock_server)
        .await;

    let config = WeaviateConnectorConfig {
        base_url: mock_server.uri(),
        api_key: Some(zeroize::Zeroizing::new("test-api-key".to_string())),
        allow_private_network: true,
        default_class_name: Some(class_name.to_string()),
        title_property: DEFAULT_WEAVIATE_TITLE_PROPERTY.to_string(),
        content_property: DEFAULT_WEAVIATE_CONTENT_PROPERTY.to_string(),
    };
    let engine = WeaviateKnowledgeEngine::with_config(config);

    let document = engine
        .read_document(
            &provider_execution_context(1, 2, 42, 7, "trace-adapter-read"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 42,
                document_id: "Space Doc#rec-9".to_string(),
            },
        )
        .await
        .expect("read");

    assert_eq!(document.title, "Space Doc");
    assert_eq!(document.content, "full object body");
    assert_eq!(document.document_id, "Space Doc#rec-9");
}

#[tokio::test]
async fn weaviate_list_documents_is_explicitly_unsupported() {
    let config = WeaviateConnectorConfig {
        base_url: "http://localhost:8080".to_string(),
        api_key: None,
        allow_private_network: true,
        default_class_name: Some("KnowledgeChunk".to_string()),
        title_property: DEFAULT_WEAVIATE_TITLE_PROPERTY.to_string(),
        content_property: DEFAULT_WEAVIATE_CONTENT_PROPERTY.to_string(),
    };
    let engine = WeaviateKnowledgeEngine::with_config(config);

    let error = engine
        .list_documents(
            &knowledge_execution_context(1, 1, 42, None, "trace-weaviate-list"),
            KnowledgeEngineListRequest {
                tenant_id: 1,
                space_id: 42,
                limit: 10,
            },
        )
        .await
        .expect_err("list_documents must not synthesize class descriptors");

    assert!(matches!(error, KnowledgeEngineError::Unsupported(_)));
}

async fn assert_weaviate_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/.well-known/ready"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = WeaviateKnowledgeEngine::with_config(WeaviateConnectorConfig {
        base_url: mock_server.uri(),
        api_key: None,
        allow_private_network: true,
        default_class_name: Some("HealthClass".to_string()),
        title_property: DEFAULT_WEAVIATE_TITLE_PROPERTY.to_string(),
        content_property: DEFAULT_WEAVIATE_CONTENT_PROPERTY.to_string(),
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn weaviate_health_maps_upstream_availability() {
    assert_weaviate_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_weaviate_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}
