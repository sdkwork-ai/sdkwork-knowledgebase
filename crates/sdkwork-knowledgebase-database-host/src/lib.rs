use std::path::PathBuf;
use std::sync::Arc;

use sdkwork_database_config::{DatabaseConfig, DatabaseEngine};
use sdkwork_database_lifecycle::{lifecycle_options_from_env, LifecycleOrchestrator};
use sdkwork_database_spi::{DatabaseAssetProvider, DatabaseManifest, DefaultDatabaseModule};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};

pub mod postgres_scope;

pub use postgres_scope::{
    postgres_url_with_deployment_scope, require_postgres_rls_organization_id,
    require_postgres_rls_tenant_id, POSTGRES_ORGANIZATION_SESSION_KEY, POSTGRES_TENANT_SESSION_KEY,
};

pub struct KnowledgebaseDatabaseHost {
    pool: DatabasePool,
    module: Arc<DefaultDatabaseModule>,
}

impl KnowledgebaseDatabaseHost {
    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub fn module(&self) -> Arc<DefaultDatabaseModule> {
        self.module.clone()
    }
}

pub async fn bootstrap_knowledgebase_database(
    pool: DatabasePool,
) -> Result<KnowledgebaseDatabaseHost, String> {
    let app_root = resolve_app_root();
    let module = Arc::new(
        DefaultDatabaseModule::from_app_root(&app_root)
            .map_err(|error| format!("load knowledgebase database module failed: {error}"))?,
    );
    let manifest = DatabaseManifest::from_file(module.manifest_path())
        .map_err(|error| format!("read knowledgebase database manifest failed: {error}"))?;
    let options = lifecycle_options_from_env("KNOWLEDGEBASE", &manifest);
    let orchestrator = LifecycleOrchestrator::new(pool.clone(), module.clone())
        .with_applied_by("sdkwork-knowledgebase");

    orchestrator
        .init()
        .await
        .map_err(|error| format!("knowledgebase database init failed: {error}"))?;

    if options.auto_migrate {
        orchestrator
            .migrate()
            .await
            .map_err(|error| format!("knowledgebase database migrate failed: {error}"))?;
    }

    Ok(KnowledgebaseDatabaseHost { pool, module })
}

pub async fn bootstrap_knowledgebase_database_from_env() -> Result<KnowledgebaseDatabaseHost, String>
{
    let _ = dotenvy::dotenv();
    let mut config = DatabaseConfig::from_env("KNOWLEDGEBASE")
        .map_err(|error| format!("read knowledgebase database config failed: {error}"))?;
    if config.engine == DatabaseEngine::Postgres {
        let tenant_id = require_postgres_rls_tenant_id().map_err(|error| error.to_string())?;
        let organization_id =
            require_postgres_rls_organization_id().map_err(|error| error.to_string())?;
        config.url = postgres_url_with_deployment_scope(&config.url, tenant_id, organization_id)
            .map_err(|error| error.to_string())?;
    } else {
        return Err(
            "knowledgebase server persistence requires a PostgreSQL database url".to_string(),
        );
    }
    let pool = create_pool_from_config(config)
        .await
        .map_err(|error| format!("create knowledgebase database pool failed: {error}"))?;
    bootstrap_knowledgebase_database(pool).await
}

fn resolve_app_root() -> PathBuf {
    std::env::var("SDKWORK_KNOWLEDGEBASE_APP_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        })
}
