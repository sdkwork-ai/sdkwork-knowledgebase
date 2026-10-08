use sdkwork_intelligence_knowledgebase_service::knowledge_engine::{
    InMemoryKnowledgeEngineRegistry, KnowledgeEngine, KnowledgeEngineRegistry,
};
use sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError;
use sdkwork_knowledgebase_engine_dify::{DifyConnectorConfig, DifyKnowledgeEngine};
use std::sync::Arc;

#[test]
fn registry_rejects_duplicate_implementation_ids_without_replacing_engine() {
    let engine: Arc<dyn KnowledgeEngine> =
        Arc::new(DifyKnowledgeEngine::with_config(DifyConnectorConfig {
            base_url: "http://127.0.0.1:1".to_string(),
            api_key: Default::default(),
            allow_private_network: false,
            default_dataset_id: None,
        }));
    let implementation_id = engine.descriptor().implementation_id;
    let mut registry = InMemoryKnowledgeEngineRegistry::new();

    registry
        .register(engine.clone())
        .expect("first registration");
    let error = registry
        .register(engine.clone())
        .expect_err("duplicate registration must be rejected");

    assert!(matches!(
        error,
        KnowledgeEngineError::Validation(message)
            if message.contains(&implementation_id)
    ));
    let resolved = registry
        .resolve_by_id(&implementation_id)
        .expect("original engine remains registered");
    assert!(std::sync::Arc::ptr_eq(&engine, &resolved));
}
