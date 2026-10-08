use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineReadRequest;
use sdkwork_knowledgebase_engine_ragflow::{RagflowConnectorConfig, RagflowKnowledgeEngine};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn ragflow_read_document_fetches_chunk_detail() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/api/v1/datasets/ds-42/documents/doc-1/chunks/chunk-9",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0,
            "data": {
                "content": "chunk body",
                "document_keyword": "Space Doc"
            }
        })))
        .mount(&mock_server)
        .await;

    let config = RagflowConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_dataset_id: Some("ds-42".to_string()),
    };
    let engine = RagflowKnowledgeEngine::with_config(config);

    let document = engine
        .read_document(
            &provider_execution_context(1, 2, 42, 7, "trace-adapter-read"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 42,
                document_id: "doc-1#chunk-9".to_string(),
            },
        )
        .await
        .expect("read");

    assert_eq!(document.title, "Space Doc");
    assert_eq!(document.content, "chunk body");
    assert_eq!(document.document_id, "doc-1#chunk-9");
}
