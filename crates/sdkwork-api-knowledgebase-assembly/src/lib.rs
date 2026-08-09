//! API assembly for sdkwork-knowledgebase.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM: preserve authored environment bootstrap exports.

mod bootstrap;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_api_router_from_environment,
    assemble_api_router_from_environment_with_group_launch_ticket_consumer,
    assemble_api_router_with_pool, assemble_app_api_contribution,
    assemble_app_api_contribution_from_environment, assemble_business_routes,
    assemble_business_routes_from_environment, ApiAssembly,
};

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}
