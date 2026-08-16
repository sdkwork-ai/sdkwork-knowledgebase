//! API assembly for sdkwork-knowledgebase.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM: preserve authored environment bootstrap exports.

mod bootstrap;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_api_router_from_environment,
    assemble_api_router_from_environment_with_group_launch_ticket_consumer,
    assemble_api_router_with_pool, assemble_api_router_with_pool_and_provider_credential_resolver,
    assemble_api_router_with_pool_for_environment, assemble_app_api_contribution,
    assemble_app_api_contribution_from_environment, assemble_business_routes,
    assemble_business_routes_from_environment, ApiAssembly,
};

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}

/// Public path prefixes of the Knowledgebase Open API surface, projected from
/// the assembly-owned route crate for gateway Web Framework wiring.
pub fn knowledgebase_open_api_prefixes() -> Vec<String> {
    sdkwork_routes_knowledgebase_open_api::knowledgebase_open_api_prefixes()
}
