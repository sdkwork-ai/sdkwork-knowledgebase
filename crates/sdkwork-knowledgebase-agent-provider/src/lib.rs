//! Agent provider adapter for SDKWork Knowledgebase.

pub mod agent_implementation;
pub mod agent_runtime;
pub mod async_bridge;
pub mod client;
pub mod cloud_router;
pub mod cloud_router_embeddings;
pub mod external_space_engine_provider;
pub mod knowledge_access;
mod mapper;
pub mod okf;
mod okf_ranking;
pub mod provider;
pub mod retrieval_plan;

pub use agent_implementation::{
    default_profile_agent_implementation_id, is_rig_agent_implementation,
    resolve_model_provider_for_implementation, resolve_rig_model_provider_id,
    validate_registered_agent_implementation, CONTRACT_MODEL_PROVIDER_ID,
};
pub use agent_runtime::{build_knowledge_agent_runtime, KnowledgeAgentRuntimeBuildRequest};
pub use client::KnowledgebaseRetrievalClient;
pub use cloud_router::{
    is_rig_model_provider, resolve_cloud_router_client, resolve_cloud_router_client_from_env,
    CloudRouterChatModelProvider, CLOUDROUTER_CHAT_COMPLETION_METHOD,
    CLOUDROUTER_OPEN_HTTP_URL_ENV, CLOUDROUTER_OPEN_SDK_CRATE,
    DEFAULT_CLOUDROUTER_UPSTREAM_MODEL_ID, RIG_DEFAULT_MODEL_ID, RIG_MODEL_PROVIDER_ID,
};
pub use cloud_router_embeddings::{
    cosine_similarity, deserialize_embedding_vector, serialize_embedding_vector,
    CloudRouterEmbeddingClient, CLOUDROUTER_EMBEDDINGS_METHOD,
    DEFAULT_CLOUDROUTER_EMBEDDING_MODEL_ID,
};
pub use external_space_engine_provider::{
    engine_hit_to_search_result, SpaceEngineKnowledgeProvider,
};
pub use knowledge_access::{
    default_top_k, enabled_bindings, resolve_chat_knowledge_mode, validate_bindings_support_mode,
    validate_rag_profile_requirements, KnowledgeAccessError, KnowledgeAccessGateway,
    KnowledgeAccessRequest, KnowledgeAccessResult, KnowledgeAccessRetrievalExecutor,
    KnowledgeRetrievalPlanResolver, KnowledgeSpaceModeResolver, SpaceKnowledgeEngineClient,
};
pub use okf::{
    citations_from_engine_hits, citations_from_okf_concepts,
    citations_from_okf_concepts_with_query, citations_from_rag_hits, OkfKnowledgeClient,
    OkfKnowledgeProvider,
};
pub use provider::{SdkworkKnowledgebaseProvider, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID};
pub use retrieval_plan::{
    default_rag_methods, kernel_methods_for_retrieval, merge_retrieval_plan,
    retrieval_methods_for_strategy, KnowledgeRetrievalPlan,
};
pub use sdkwork_knowledgebase_contract::okf::okf_document_id;
pub use sdkwork_knowledgebase_contract::OKF_KNOWLEDGE_PROVIDER_ID;
