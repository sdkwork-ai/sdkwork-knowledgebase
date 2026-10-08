use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineError, KnowledgeEngineHealthStatus, KnowledgeEngineListRequest,
    KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_engine_chroma::{
    ChromaConnectorConfig, ChromaKnowledgeEngine, DEFAULT_CHROMA_DATABASE, DEFAULT_CHROMA_TENANT,
};
use sdkwork_knowledgebase_test_support::provider_execution::{
    knowledge_execution_context, provider_execution_context,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn collection_base_path(collection_id: &str) -> String {
    format!(
        "/api/v2/tenants/{DEFAULT_CHROMA_TENANT}/databases/{DEFAULT_CHROMA_DATABASE}/collections/{collection_id}"
    )
}

#[tokio::test]
async fn chroma_search_uses_configured_remote_resource_id() {
    let mock_server = MockServer::start().await;
    let collection_id = "603a7b51-ae7c-4b0a-8865-e454ed2f6766";
    Mock::given(method("POST"))
        .and(path(format!(
            "{}/query",
            collection_base_path(collection_id)
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ids": [["rec-9"]],
            "documents": [["space scoped answer"]],
            "metadatas": [[{
                "title": "Space Doc",
                "source": "file://space-doc.txt"
            }]],
            "distances": [[0.1]]
        })))
        .mount(&mock_server)
        .await;

    let config = ChromaConnectorConfig {
        base_url: mock_server.uri(),
        api_key: Some(zeroize::Zeroizing::new("test-api-key".to_string())),
        allow_private_network: true,
        default_collection_id: Some(collection_id.to_string()),
        tenant: DEFAULT_CHROMA_TENANT.to_string(),
        database: DEFAULT_CHROMA_DATABASE.to_string(),
    };
    let engine = ChromaKnowledgeEngine::with_config(config);

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
async fn chroma_read_document_fetches_record_by_id() {
    let mock_server = MockServer::start().await;
    let collection_id = "603a7b51-ae7c-4b0a-8865-e454ed2f6766";
    Mock::given(method("POST"))
        .and(path(format!("{}/get", collection_base_path(collection_id))))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ids": ["rec-9"],
            "documents": ["full record body"],
            "metadatas": [{
                "title": "Space Doc",
                "source": "file://space-doc.txt"
            }]
        })))
        .mount(&mock_server)
        .await;

    let config = ChromaConnectorConfig {
        base_url: mock_server.uri(),
        api_key: Some(zeroize::Zeroizing::new("test-api-key".to_string())),
        allow_private_network: true,
        default_collection_id: Some(collection_id.to_string()),
        tenant: DEFAULT_CHROMA_TENANT.to_string(),
        database: DEFAULT_CHROMA_DATABASE.to_string(),
    };
    let engine = ChromaKnowledgeEngine::with_config(config);

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
    assert_eq!(document.content, "full record body");
    assert_eq!(document.document_id, "Space Doc#rec-9");
}

#[tokio::test]
async fn chroma_list_documents_is_explicitly_unsupported() {
    let collection_id = "603a7b51-ae7c-4b0a-8865-e454ed2f6766";
    let config = ChromaConnectorConfig {
        base_url: "http://localhost:8000".to_string(),
        api_key: None,
        allow_private_network: true,
        default_collection_id: Some(collection_id.to_string()),
        tenant: DEFAULT_CHROMA_TENANT.to_string(),
        database: DEFAULT_CHROMA_DATABASE.to_string(),
    };
    let engine = ChromaKnowledgeEngine::with_config(config);

    let error = engine
        .list_documents(
            &knowledge_execution_context(1, 1, 42, None, "trace-chroma-list"),
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

async fn assert_chroma_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/heartbeat"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = ChromaKnowledgeEngine::with_config(ChromaConnectorConfig {
        base_url: mock_server.uri(),
        api_key: None,
        allow_private_network: true,
        default_collection_id: Some("health-collection".to_string()),
        tenant: DEFAULT_CHROMA_TENANT.to_string(),
        database: DEFAULT_CHROMA_DATABASE.to_string(),
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn chroma_health_maps_upstream_availability() {
    assert_chroma_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_chroma_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}
