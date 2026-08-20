//! App API route boundary for SDKWork Knowledgebase.

use std::sync::Arc;

use axum::Router;
use sdkwork_web_core::HttpRouteManifest;

mod adapters;
mod agent_chat_runtime;
mod auth;
pub mod bootstrap;
#[cfg(feature = "dev-auth")]
pub mod dev_auth;
mod error;
pub mod hosted;
mod hosted_access;
mod hosted_backend;
mod hosted_commerce;
mod hosted_context_binding;
mod hosted_group_launch;
mod hosted_open;
mod hosted_support;
mod hosted_wechat;
mod hosted_wiki;
pub mod http_route_manifest;
mod knowledge_engine_adapters;
pub mod manifest;
pub mod pagination;
pub mod paths;
mod ports;
mod routes;
pub mod runtime;
mod tenant_quota_enforcement;
mod web_bootstrap;

pub use error::{ApiError, ApiProblem, ApiResult};
pub use http_route_manifest::app_route_manifest;
pub use ports::{
    KnowledgeAgentAppService, KnowledgeAppApi, KnowledgeAppRequestContext, KnowledgeBrowserApi,
    KnowledgeCommerceAppService, KnowledgeContextBindingAppService, KnowledgeDocumentAppService,
    KnowledgeDriveImportAppService, KnowledgeGitImportAppService, KnowledgeGroupLaunchAppService,
    KnowledgeIngestAppService, KnowledgeOkfAppService, KnowledgeRetrievalAppService,
    KnowledgeSpaceAppService, KnowledgeWechatAppService, KnowledgeWikiPublicationAppService,
};
pub use routes::{
    build_router_with_agent_and_retrieval_services, build_router_with_agent_service,
    build_router_with_app_api, build_router_with_browser, build_router_with_full_app_api,
    build_router_with_retrieval_service, build_router_with_shared_agent_and_retrieval_services,
    build_router_with_shared_agent_service, build_router_with_shared_app_api,
    build_router_with_shared_app_api_and_readiness, build_router_with_shared_browser,
    build_router_with_shared_retrieval_service, ReadinessCheck,
};
pub use runtime::KnowledgebaseRuntime;
pub use sdkwork_knowledgebase_contract::ProblemDetails;
pub use web_bootstrap::{
    knowledgebase_app_context_injector, knowledgebase_public_path_prefixes,
    wrap_router_with_web_framework, wrap_router_with_web_framework_from_env,
};

pub fn gateway_route_manifest() -> HttpRouteManifest {
    app_route_manifest()
}

/// Mounts the shared browser API as a gateway sub-router.
///
/// This is the canonical gateway mount point for the app API.
pub fn gateway_mount(browser: Arc<dyn KnowledgeBrowserApi>) -> Router {
    build_router_with_shared_browser(browser)
}
