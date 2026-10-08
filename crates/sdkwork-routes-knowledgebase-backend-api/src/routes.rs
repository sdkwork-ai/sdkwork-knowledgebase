use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::{
    auth::configured_runtime_organization_id, handlers, health, paths, ports::KnowledgeBackendApi,
    KnowledgebaseReadinessCheck,
};

#[derive(Clone)]
pub struct BackendState {
    pub(crate) api: Arc<dyn KnowledgeBackendApi>,
    pub(crate) runtime_tenant_id: u64,
    pub(crate) runtime_organization_id: u64,
}

pub fn build_router_with_backend_api<A>(api: A, runtime_tenant_id: u64) -> Router
where
    A: KnowledgeBackendApi,
{
    build_router_with_shared_backend_api(Arc::new(api), runtime_tenant_id)
}

pub fn build_router_with_shared_backend_api(
    api: Arc<dyn KnowledgeBackendApi>,
    runtime_tenant_id: u64,
) -> Router {
    build_router_with_shared_backend_api_and_readiness(api, runtime_tenant_id, None)
}

pub fn build_router_with_shared_backend_api_and_readiness(
    api: Arc<dyn KnowledgeBackendApi>,
    runtime_tenant_id: u64,
    readiness: Option<KnowledgebaseReadinessCheck>,
) -> Router {
    health::mount_knowledgebase_infra_routes(
        build_business_router_with_shared_backend_api(api, runtime_tenant_id),
        health::knowledgebase_service_router_config(readiness),
    )
}

pub fn build_business_router_with_shared_backend_api(
    api: Arc<dyn KnowledgeBackendApi>,
    runtime_tenant_id: u64,
) -> Router {
    let state = BackendState {
        api,
        runtime_tenant_id,
        // Resolved once at boot so the per-request auth path never reads the
        // process environment.
        runtime_organization_id: configured_runtime_organization_id(),
    };
    Router::new()
        .route(
            paths::SOURCES,
            get(handlers::list_sources).post(handlers::create_source),
        )
        .route(
            paths::OKF_COMPILE_JOBS,
            post(handlers::create_okf_compile_job),
        )
        .route(paths::OKF_CANDIDATES, get(handlers::list_okf_candidates))
        .route(
            paths::OKF_CANDIDATE_APPROVE,
            post(handlers::approve_okf_candidate),
        )
        .route(
            paths::OKF_CANDIDATE_REJECT,
            post(handlers::reject_okf_candidate),
        )
        .route(
            paths::OKF_CONCEPT_PUBLISH,
            post(handlers::publish_okf_concept),
        )
        .route(paths::OKF_PROFILES, post(handlers::create_okf_profile))
        .route(paths::OKF_PROFILE, patch(handlers::update_okf_profile))
        .route(paths::OKF_INDEX_REBUILD, post(handlers::rebuild_okf_index))
        .route(paths::OKF_LOG_ENTRIES, post(handlers::create_okf_log_entry))
        .route(paths::OKF_EXPORTS, post(handlers::create_okf_export))
        .route(paths::OKF_EXPORT, get(handlers::retrieve_okf_export))
        .route(paths::OKF_IMPORTS, post(handlers::create_okf_import))
        .route(
            paths::OKF_BUNDLE_FILES,
            get(handlers::list_okf_bundle_files),
        )
        .route(paths::OKF_LINT_RUNS, post(handlers::create_okf_lint_run))
        .route(paths::OKF_EVAL_RUNS, post(handlers::create_okf_eval_run))
        .route(
            paths::INDEXES,
            get(handlers::list_indexes).post(handlers::create_index),
        )
        .route(paths::INDEX, get(handlers::retrieve_index))
        .route(paths::INDEX_REBUILD, post(handlers::rebuild_index))
        .route(
            paths::RETRIEVAL_PROFILES,
            post(handlers::create_retrieval_profile),
        )
        .route(
            paths::RETRIEVAL_PROFILE,
            get(handlers::retrieve_retrieval_profile).patch(handlers::update_retrieval_profile),
        )
        .route(
            paths::RETRIEVAL_TRACES,
            get(handlers::list_retrieval_traces),
        )
        .route(
            paths::RETRIEVAL_TRACE,
            get(handlers::retrieve_retrieval_trace),
        )
        .route(
            paths::PROVIDER_HEALTH,
            get(handlers::retrieve_provider_health),
        )
        .route(
            paths::PROVIDER_CREDENTIAL_REFERENCES,
            get(handlers::list_provider_credential_references)
                .post(handlers::create_provider_credential_reference),
        )
        .route(
            paths::PROVIDER_CREDENTIAL_REFERENCE,
            get(handlers::retrieve_provider_credential_reference),
        )
        .route(
            paths::PROVIDER_CREDENTIAL_REFERENCE_ROTATE,
            post(handlers::rotate_provider_credential_reference),
        )
        .route(
            paths::PROVIDER_CREDENTIAL_REFERENCE_REVOKE,
            post(handlers::revoke_provider_credential_reference),
        )
        .route(
            paths::SPACE_PROVIDER_BINDINGS,
            get(handlers::list_provider_bindings).post(handlers::create_provider_binding),
        )
        .route(
            paths::SPACE_PROVIDER_BINDING,
            get(handlers::retrieve_provider_binding).patch(handlers::update_provider_binding),
        )
        .route(
            paths::SPACE_PROVIDER_BINDING_TEST,
            post(handlers::test_provider_binding),
        )
        .route(
            paths::SPACE_PROVIDER_BINDING_ACTIVATE,
            post(handlers::activate_provider_binding),
        )
        .route(
            paths::SPACE_PROVIDER_BINDING_DISABLE,
            post(handlers::disable_provider_binding),
        )
        .route(
            paths::SPACE_PROVIDER_MIGRATIONS,
            get(handlers::list_provider_migrations).post(handlers::create_provider_migration),
        )
        .route(
            paths::SPACE_PROVIDER_MIGRATION,
            get(handlers::retrieve_provider_migration),
        )
        .route(
            paths::SPACE_PROVIDER_MIGRATION_ROLLBACK,
            post(handlers::rollback_provider_migration),
        )
        .route(
            paths::GROUP_LAUNCH_CAPABILITY,
            get(handlers::retrieve_group_launch_capability),
        )
        .route(
            paths::TENANT_LANDING,
            get(handlers::retrieve_current_tenant),
        )
        .route(paths::SPACES, get(handlers::list_spaces))
        .route(paths::SPACE_MEMBERS, get(handlers::list_space_members))
        .route(
            paths::COMPLIANCE_AUDIT_EVENTS_EXPORT,
            post(handlers::export_audit_events),
        )
        .route(
            paths::COMPLIANCE_AUDIT_EVENTS_ANONYMIZE,
            post(handlers::anonymize_audit_subject),
        )
        // Explicit transport-level request body bound (1 MiB): backend payloads are bounded
        // small DTOs; reject oversized bodies before JSON deserialization.
        .layer(axum::extract::DefaultBodyLimit::max(1_048_576))
        .with_state(state)
}

pub fn gateway_mount_business(api: Arc<dyn KnowledgeBackendApi>, runtime_tenant_id: u64) -> Router {
    build_business_router_with_shared_backend_api(api, runtime_tenant_id)
}
