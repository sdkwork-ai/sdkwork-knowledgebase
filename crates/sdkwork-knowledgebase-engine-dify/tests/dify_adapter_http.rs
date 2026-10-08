use sdkwork_intelligence_knowledgebase_service::knowledge_engine::KnowledgeEngine;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_provider_credential_resolver::KnowledgeEngineProviderCredential;
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineCapability, KnowledgeEngineHealthStatus, KnowledgeEngineSearchRequest,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingState,
};
use sdkwork_knowledgebase_engine_dify::{
    DifyConnectorConfig, DifyKnowledgeEngine, DIFY_IMPLEMENTATION_ID,
};
use sdkwork_knowledgebase_test_support::provider_execution::provider_execution_context;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn assert_dify_health(upstream_status: u16, expected: KnowledgeEngineHealthStatus) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/datasets/health-dataset"))
        .respond_with(ResponseTemplate::new(upstream_status))
        .expect(if upstream_status >= 500 { 3 } else { 1 })
        .mount(&mock_server)
        .await;
    let engine = DifyKnowledgeEngine::with_config(DifyConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("health-key".to_string()),
        allow_private_network: true,
        default_dataset_id: Some("health-dataset".to_string()),
    });

    assert_eq!(engine.health().await.expect("health").status, expected);
}

#[tokio::test]
async fn dify_health_maps_upstream_availability() {
    assert_dify_health(200, KnowledgeEngineHealthStatus::Available).await;
    assert_dify_health(503, KnowledgeEngineHealthStatus::Degraded).await;
}

fn active_binding(remote_resource_id: &str) -> KnowledgeEngineProviderBinding {
    KnowledgeEngineProviderBinding {
        id: 7,
        uuid: "binding-7".to_string(),
        tenant_id: 1,
        organization_id: 2,
        space_id: 42,
        implementation_id: DIFY_IMPLEMENTATION_ID.to_string(),
        remote_resource_type: "dataset".to_string(),
        remote_resource_id: remote_resource_id.to_string(),
        credential_reference_id: Some(9),
        lifecycle_state: KnowledgeEngineProviderBindingState::Active,
        capability_snapshot: vec![KnowledgeEngineCapability::Search],
        capability_snapshot_version: 1,
        last_tested_at: Some("2026-07-20T00:00:00Z".to_string()),
        activated_at: Some("2026-07-20T00:00:01Z".to_string()),
        disabled_at: None,
        last_error_category: None,
        created_by: "actor-1".to_string(),
        updated_by: "actor-1".to_string(),
        created_at: "2026-07-20T00:00:00Z".to_string(),
        updated_at: "2026-07-20T00:00:01Z".to_string(),
        version: 2,
    }
}

#[tokio::test]
async fn dify_search_uses_binding_owned_remote_resource_id() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/datasets/binding-dataset/retrieve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "records": [{
                "segment": {
                    "id": "seg-9",
                    "content": "space scoped answer",
                    "document": { "name": "Space Doc" }
                },
                "score": 0.88
            }]
        })))
        .mount(&mock_server)
        .await;

    let config = DifyConnectorConfig {
        base_url: mock_server.uri(),
        api_key: zeroize::Zeroizing::new("test-api-key".to_string()),
        allow_private_network: true,
        default_dataset_id: Some("template-dataset".to_string()),
    };
    let engine = DifyKnowledgeEngine::with_config(config)
        .bind_provider(
            &active_binding("binding-dataset"),
            Some(KnowledgeEngineProviderCredential::new("test-api-key").expect("test credential")),
        )
        .expect("bind provider");

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

    assert_eq!(result.implementation_id, DIFY_IMPLEMENTATION_ID);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].document.document_id, "42/seg-9");
    assert_eq!(result.hits[0].snippet, "space scoped answer");
}
