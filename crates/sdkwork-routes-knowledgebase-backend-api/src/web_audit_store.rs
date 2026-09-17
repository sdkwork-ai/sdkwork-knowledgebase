//! Framework HTTP audit persistence and dynamic policy sources for Knowledgebase web surfaces.

use std::sync::Arc;

use sdkwork_knowledgebase_observability::environment::is_production_like_environment;
use sdkwork_web_axum::WebFrameworkLayer;
use sdkwork_web_core::{AuditEmitter, WebRequestContextResolver};
use sdkwork_web_store_sqlx::{
    connect_and_bootstrap_webstore_database_from_env, shared_audit_emitter_pg,
    shared_dynamic_policy_bundle_pg, SqlxDynamicPolicyBundle,
};
use tokio::sync::OnceCell;

struct KnowledgebaseWebStoreBundle {
    audit_emitter: Arc<dyn AuditEmitter>,
    policy_bundle: SqlxDynamicPolicyBundle,
}

static SHARED_WEB_STORE: OnceCell<Option<Arc<KnowledgebaseWebStoreBundle>>> = OnceCell::const_new();

async fn build_knowledgebase_web_store_bundle() -> Option<Arc<KnowledgebaseWebStoreBundle>> {
    let host = connect_and_bootstrap_webstore_database_from_env()
        .await
        .ok()?;
    if let Err(error) = crate::web_policy_bootstrap::seed_default_tenant_web_policies(&host).await {
        eprintln!("[knowledgebase] web policy bootstrap skipped: {error}");
    }
    if let Some(postgres) = host.pool().as_postgres().cloned() {
        return Some(Arc::new(KnowledgebaseWebStoreBundle {
            audit_emitter: shared_audit_emitter_pg(postgres.clone()),
            policy_bundle: shared_dynamic_policy_bundle_pg(postgres),
        }));
    }
    None
}

async fn shared_knowledgebase_web_store_bundle() -> Option<Arc<KnowledgebaseWebStoreBundle>> {
    SHARED_WEB_STORE
        .get_or_init(|| async { build_knowledgebase_web_store_bundle().await })
        .await
        .clone()
}

pub async fn attach_knowledgebase_audit_emitter<R>(
    layer: WebFrameworkLayer<R>,
) -> WebFrameworkLayer<R>
where
    R: WebRequestContextResolver + Clone,
{
    match shared_knowledgebase_web_store_bundle().await {
        Some(bundle) => layer
            .with_audit_emitter(bundle.audit_emitter.clone())
            .with_dynamic_cors_policy_source(bundle.policy_bundle.cors_policy_source.clone())
            .with_dynamic_rate_limit_policy_source(
                bundle.policy_bundle.rate_limit_policy_source.clone(),
            )
            .with_dynamic_tenant_runtime_profile_source(
                bundle.policy_bundle.tenant_runtime_profile_source.clone(),
            ),
        None if is_production_like_environment() => {
            eprintln!(
                "Web audit emitter is required for production-like environments; configure SDKWORK_DATABASE_URL and bootstrap framework_audit_event migrations"
            );
            std::process::exit(1);
        }
        None => layer,
    }
}
