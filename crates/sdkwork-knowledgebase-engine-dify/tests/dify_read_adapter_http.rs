use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineReadRequest;
use sdkwork_knowledgebase_engine_dify::{DifyConnectorConfig, DifyKnowledgeEngine};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn dify_read_document_fetches_segment_detail() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/datasets/ds-42/documents/doc-1/segments/seg-9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "content": "segment body",
                "document": { "name": "Space Doc" }
            }
        })))
        .mount(&mock_server)
        .await;

    let config = DifyConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_dataset_id: Some("ds-42".to_string()),
    };
    let engine = DifyKnowledgeEngine::with_config(config);

    let document = engine
        .read_document(
            &provider_execution_context(1, 2, 42, 7, "trace-adapter-read"),
            KnowledgeEngineReadRequest {
                tenant_id: 1,
                space_id: 42,
                document_id: "doc-1#seg-9".to_string(),
            },
        )
        .await
        .expect("read");

    assert_eq!(document.title, "Space Doc");
    assert_eq!(document.content, "segment body");
    assert_eq!(document.document_id, "doc-1#seg-9");
}
