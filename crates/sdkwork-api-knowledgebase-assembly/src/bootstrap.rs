//! Host-neutral Knowledgebase API assembly.

use std::sync::Arc;

use axum::Router;
use sdkwork_agent_kernel::{
    ChainedSecretProvider, EnvFileSecretProvider, SecretProvider, VaultSecretProvider,
};
use sdkwork_intelligence_knowledgebase_service::ports::{
    group_launch_ticket_consumer::GroupLaunchTicketConsumer,
    knowledge_provider_credential_resolver::KnowledgeEngineProviderCredentialResolver,
};
use sdkwork_knowledgebase_provider_secret_adapter::{
    KnowledgebaseProviderCredentialEnvironment, KnowledgebaseProviderCredentialResolver,
    KnowledgebaseProviderCredentialResolverConfig,
};
use sdkwork_routes_knowledgebase_app_api::bootstrap::{
    resolve_database_url, validate_process_config,
};
use sdkwork_routes_knowledgebase_app_api::KnowledgebaseRuntime;
use sdkwork_web_bootstrap::{ApiAssemblyContribution, HttpRouteManifest, WebModule};

pub type ApiAssembly = ApiAssemblyContribution;

type BootstrapError = Box<dyn std::error::Error + Send + Sync>;

async fn runtime_from_environment_with_dependencies(
    group_launch_ticket_consumer: Option<Arc<dyn GroupLaunchTicketConsumer>>,
    provider_credential_resolver: Option<Arc<dyn KnowledgeEngineProviderCredentialResolver>>,
) -> Result<Arc<KnowledgebaseRuntime>, BootstrapError> {
    validate_process_config();
    let database_url = resolve_database_url();
    let tenant_id = std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    let runtime = match provider_credential_resolver {
        Some(resolver) => {
            KnowledgebaseRuntime::connect_with_provider_credential_resolver(
                &database_url,
                tenant_id,
                resolver,
            )
            .await?
        }
        None => KnowledgebaseRuntime::connect(&database_url, tenant_id).await?,
    };
    let runtime = match group_launch_ticket_consumer {
        Some(consumer) => runtime.with_group_launch_ticket_consumer(consumer),
        None => runtime,
    };
    runtime.readiness_check().await?;
    Ok(Arc::new(runtime))
}

async fn runtime_from_environment_with_group_launch_ticket_consumer(
    group_launch_ticket_consumer: Option<Arc<dyn GroupLaunchTicketConsumer>>,
) -> Result<Arc<KnowledgebaseRuntime>, BootstrapError> {
    runtime_from_environment_with_dependencies(group_launch_ticket_consumer, None).await
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

/// Assemble the Knowledgebase contribution with a host-managed Provider
/// credential resolver while retaining the process-shared database pool.
pub async fn assemble_api_router_with_pool_and_provider_credential_resolver(
    pool: sdkwork_database_sqlx::DatabasePool,
    provider_credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
) -> Result<ApiAssembly, String> {
    let runtime =
        runtime_from_environment_with_dependencies(None, Some(provider_credential_resolver))
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

/// Canonical host-neutral integration point for cloud and standalone hosts.
/// The owner assembly selects and constructs its Provider credential adapter;
/// consumers supply only the process pool and lifecycle environment.
pub async fn assemble_api_router_with_pool_for_environment(
    pool: sdkwork_database_sqlx::DatabasePool,
    environment: &str,
) -> Result<ApiAssembly, String> {
    if !matches!(
        environment.trim().to_ascii_lowercase().as_str(),
        "staging" | "production"
    ) {
        return assemble_api_router_with_pool(pool).await;
    }

    let resolver = managed_provider_credential_resolver(environment)?;
    assemble_api_router_with_pool_and_provider_credential_resolver(pool, resolver).await
}

fn managed_provider_credential_resolver(
    environment: &str,
) -> Result<Arc<dyn KnowledgeEngineProviderCredentialResolver>, String> {
    let environment = KnowledgebaseProviderCredentialEnvironment::parse(environment)
        .map_err(|error| error.to_string())?;
    let config = KnowledgebaseProviderCredentialResolverConfig::managed(environment)
        .map_err(|error| error.to_string())?;

    let mut providers: Vec<Box<dyn SecretProvider + Send + Sync>> = Vec::new();
    if let Some(vault) = VaultSecretProvider::from_process_environment() {
        providers.push(Box::new(vault));
    }
    providers.push(Box::new(EnvFileSecretProvider::from_process_environment()));

    let secret_provider: Arc<dyn SecretProvider> = Arc::new(ChainedSecretProvider::new(providers));
    let resolver = KnowledgebaseProviderCredentialResolver::managed(config, secret_provider)
        .map_err(|error| error.to_string())?;
    Ok(Arc::new(resolver))
}

/// Canonical Web Module definition for this application
/// (API_ASSEMBLY_SPEC §4.1.1): the complete HTTP surface — every route,
/// manifest, and OpenAPI document of this owner — as one installable module.
pub async fn web_module() -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(
        assemble_api_router_from_environment()
            .await
            .map_err(|error| error.to_string())?,
    ))
}

/// Same as [`web_module`] but composed on a process-shared database pool
/// (platform gateways, API_ASSEMBLY_SPEC §4.1.1).
pub async fn web_module_with_pool(
    pool: sdkwork_database_sqlx::DatabasePool,
) -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(
        assemble_api_router_with_pool(pool).await?,
    ))
}

/// Same as [`web_module_with_pool`] but resolved against an explicit
/// deployment environment (platform gateways, API_ASSEMBLY_SPEC §4.1.1).
///
/// Staging and production resolve managed provider credentials; every other
/// environment keeps the plain process-pool composition.
pub async fn web_module_with_pool_for_environment(
    pool: sdkwork_database_sqlx::DatabasePool,
    environment: &str,
) -> Result<WebModule, String> {
    Ok(WebModule::from_contribution(
        assemble_api_router_with_pool_for_environment(pool, environment).await?,
    ))
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
