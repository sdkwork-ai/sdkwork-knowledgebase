//! Host-neutral Knowledgebase API assembly.

use std::sync::Arc;

use axum::Router;
use sdkwork_intelligence_knowledgebase_service::ports::group_launch_ticket_consumer::GroupLaunchTicketConsumer;
use sdkwork_routes_knowledgebase_app_api::bootstrap::{
    resolve_database_url, validate_process_config,
};
use sdkwork_routes_knowledgebase_app_api::KnowledgebaseRuntime;
use sdkwork_web_bootstrap::{ApiAssemblyContribution, HttpRouteManifest};

pub type ApiAssembly = ApiAssemblyContribution;

type BootstrapError = Box<dyn std::error::Error + Send + Sync>;

async fn runtime_from_environment_with_group_launch_ticket_consumer(
    group_launch_ticket_consumer: Option<Arc<dyn GroupLaunchTicketConsumer>>,
) -> Result<Arc<KnowledgebaseRuntime>, BootstrapError> {
    validate_process_config();
    let database_url = resolve_database_url();
    let tenant_id = std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    let runtime = KnowledgebaseRuntime::connect(&database_url, tenant_id).await?;
    let runtime = match group_launch_ticket_consumer {
        Some(consumer) => runtime.with_group_launch_ticket_consumer(consumer),
        None => runtime,
    };
    runtime.readiness_check().await?;
    Ok(Arc::new(runtime))
}

fn route_manifest() -> HttpRouteManifest {
    let manifests = [
        sdkwork_routes_knowledgebase_app_api::gateway_route_manifest(),
        sdkwork_routes_knowledgebase_backend_api::gateway_route_manifest(),
        sdkwork_routes_knowledgebase_internal_api::gateway_route_manifest(),
        sdkwork_routes_knowledgebase_open_api::gateway_route_manifest(),
    ];
    HttpRouteManifest::from_owned_routes(
        manifests
            .into_iter()
            .flat_map(|manifest| manifest.routes().to_vec())
            .collect(),
    )
}

pub async fn assemble_api_router(
    runtime: Arc<KnowledgebaseRuntime>,
) -> Result<ApiAssembly, String> {
    let router = Router::new()
        .merge(runtime.build_full_app_router())
        .merge(runtime.build_backend_business_router())
        .merge(runtime.build_internal_business_router())
        .merge(runtime.build_open_business_router());
    ApiAssemblyContribution::from_manifest(
        "sdkwork-knowledgebase",
        "SDKWork Knowledgebase API",
        router,
        route_manifest(),
        vec![
            sdkwork_routes_knowledgebase_app_api::knowledgebase_app_context_injector(),
            sdkwork_routes_knowledgebase_backend_api::knowledgebase_backend_context_injector(),
            sdkwork_routes_knowledgebase_open_api::knowledgebase_open_api_context_injector(),
        ],
        runtime.readiness_check_adapter(),
    )
}

pub async fn assemble_business_routes(
    runtime: Arc<KnowledgebaseRuntime>,
) -> Result<ApiAssembly, String> {
    assemble_api_router(runtime).await
}

pub fn assemble_app_api_contribution(
    runtime: Arc<KnowledgebaseRuntime>,
) -> Result<ApiAssembly, String> {
    ApiAssemblyContribution::from_manifest(
        "sdkwork-knowledgebase",
        "SDKWork Knowledgebase App API",
        runtime.build_full_app_router(),
        sdkwork_routes_knowledgebase_app_api::gateway_route_manifest(),
        vec![sdkwork_routes_knowledgebase_app_api::knowledgebase_app_context_injector()],
        runtime.readiness_check_adapter(),
    )
}

pub async fn assemble_api_router_from_environment() -> Result<ApiAssembly, BootstrapError> {
    let runtime = runtime_from_environment_with_group_launch_ticket_consumer(None).await?;
    assemble_api_router(runtime)
        .await
        .map_err(|error| Box::new(std::io::Error::other(error)) as BootstrapError)
}

pub async fn assemble_app_api_contribution_from_environment() -> Result<ApiAssembly, BootstrapError>
{
    let runtime = runtime_from_environment_with_group_launch_ticket_consumer(None).await?;
    assemble_app_api_contribution(runtime)
        .map_err(|error| Box::new(std::io::Error::other(error)) as BootstrapError)
}

pub async fn assemble_api_router_from_environment_with_group_launch_ticket_consumer<T>(
    group_launch_ticket_consumer: Option<T>,
) -> Result<ApiAssembly, BootstrapError>
where
    T: GroupLaunchTicketConsumer + 'static,
{
    let runtime = runtime_from_environment_with_group_launch_ticket_consumer(
        group_launch_ticket_consumer
            .map(|consumer| Arc::new(consumer) as Arc<dyn GroupLaunchTicketConsumer>),
    )
    .await?;
    assemble_api_router(runtime)
        .await
        .map_err(|error| Box::new(std::io::Error::other(error)) as BootstrapError)
}

pub async fn assemble_business_routes_from_environment() -> Result<ApiAssembly, BootstrapError> {
    assemble_api_router_from_environment().await
}

/// Assemble the Knowledgebase contribution against a caller-provided database
/// pool so the platform cloud gateway can share its process-wide PostgreSQL
/// pool.
pub async fn assemble_api_router_with_pool(
    pool: sdkwork_database_sqlx::DatabasePool,
) -> Result<ApiAssembly, String> {
    let runtime = runtime_from_environment_with_group_launch_ticket_consumer(None)
        .await
        .map_err(|error| format!("{error}"))?;
    let router = Router::new()
        .merge(runtime.build_full_app_router())
        .merge(runtime.build_backend_business_router())
        .merge(runtime.build_internal_business_router())
        .merge(runtime.build_open_business_router());
    ApiAssemblyContribution::from_manifest(
        "sdkwork-knowledgebase",
        "SDKWork Knowledgebase API",
        router,
        route_manifest(),
        vec![
            sdkwork_routes_knowledgebase_app_api::knowledgebase_app_context_injector(),
            sdkwork_routes_knowledgebase_backend_api::knowledgebase_backend_context_injector(),
            sdkwork_routes_knowledgebase_open_api::knowledgebase_open_api_context_injector(),
        ],
        Arc::new(sdkwork_web_bootstrap::DatabasePoolReadinessCheck::new(pool)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_manifest_is_non_empty_and_collision_free() {
        let manifest = route_manifest();
        assert!(!manifest.routes().is_empty());
        ApiAssemblyContribution::from_manifest(
            "sdkwork-knowledgebase",
            "SDKWork Knowledgebase API",
            Router::new(),
            manifest,
            Vec::new(),
            Arc::new(sdkwork_web_bootstrap::AlwaysReady),
        )
        .expect("knowledgebase route manifest must produce aligned OpenAPI and permissions");
    }
}
