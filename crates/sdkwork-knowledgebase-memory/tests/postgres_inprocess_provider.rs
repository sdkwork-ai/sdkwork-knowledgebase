//! Optional PostgreSQL proof that the in-process provider returns a written memory.
//!
//! Runs only when `SDKWORK_DATABASE_URL` is set.

use sdkwork_intelligence_knowledgebase_service::ports::knowledge_memory_context::{
    KnowledgeMemoryContextProvider, KnowledgeMemoryContextRequest,
};
use sdkwork_knowledgebase_memory::InProcessMemoryContextProvider;
use sdkwork_memory_inprocess::InProcessMemoryRuntime;

#[tokio::test]
async fn postgres_inprocess_provider_returns_written_memory() {
    let Some(database_url) = std::env::var("SDKWORK_DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        eprintln!("skipping in-process provider test: set SDKWORK_DATABASE_URL");
        return;
    };
    std::env::set_var("SDKWORK_DATABASE_URL", &database_url);

    let runtime = InProcessMemoryRuntime::from_env()
        .await
        .expect("bootstrap memory runtime")
        .expect("memory runtime configured");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let actor_id = 710_000_000_u64 + (nonce % 100_000_000) as u64;
    let space = runtime
        .create_space(100_001, Some(actor_id), format!("kb provider {nonce}"))
        .await
        .expect("create memory space");
    runtime
        .create_memory(
            100_001,
            Some(actor_id),
            space.space_id,
            format!("Refund preference {nonce}: refunds are accepted within 30 days."),
        )
        .await
        .expect("write memory");

    let provider = InProcessMemoryContextProvider::new(std::sync::Arc::new(runtime));
    let result = provider
        .build_memory_context(KnowledgeMemoryContextRequest {
            tenant_id: 100_001,
            actor_id: Some(actor_id),
            query: "refund preferences".to_string(),
            memory_policy_ref: space.space_id.to_string(),
            max_tokens: 256,
            auth_token: None,
            access_token: None,
        })
        .await
        .expect("memory context");

    assert_eq!(result.fragments.len(), 1);
    assert!(result.fragments[0].content.contains("30 days"));
    assert_eq!(
        result.fragments[0].policy_ref.as_deref(),
        Some(space.space_id.to_string().as_str())
    );
}
