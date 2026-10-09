use std::path::PathBuf;
use std::sync::Arc;

use sdkwork_database_config::{DatabaseConfig, DatabaseEngine};
use sdkwork_database_lifecycle::{lifecycle_options_from_env, LifecycleOrchestrator};
use sdkwork_database_spi::{DatabaseAssetProvider, DatabaseManifest, DefaultDatabaseModule};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};

pub mod postgres_scope;

pub use postgres_scope::{
    postgres_url_with_deployment_scope, postgres_url_with_statement_timeout,
    require_postgres_rls_organization_id, require_postgres_rls_tenant_id,
    POSTGRES_ORGANIZATION_SESSION_KEY, POSTGRES_STATEMENT_TIMEOUT_OPTION,
    POSTGRES_TENANT_SESSION_KEY,
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
    bootstrap_knowledgebase_database_with_migration_pool(pool, None).await
}

/// Bootstraps the knowledgebase database. `migration_pool`, when provided,
/// runs the lifecycle orchestrator (init/migrate) on a connection WITHOUT the
/// runtime `statement_timeout` guard: DDL such as `CREATE INDEX` over a
/// populated table can legitimately exceed 30 s, and the runtime guard would
/// cancel and fail the upgrade.
pub async fn bootstrap_knowledgebase_database_with_migration_pool(
    pool: DatabasePool,
    migration_pool: Option<DatabasePool>,
) -> Result<KnowledgebaseDatabaseHost, String> {
    let app_root = resolve_app_root()?;
    let module = Arc::new(
        DefaultDatabaseModule::from_app_root(&app_root)
            .map_err(|error| format!("load knowledgebase database module failed: {error}"))?,
    );
    let manifest = DatabaseManifest::from_file(module.manifest_path())
        .map_err(|error| format!("read knowledgebase database manifest failed: {error}"))?;
    let options = lifecycle_options_from_env("KNOWLEDGEBASE", &manifest);
    let orchestrator = LifecycleOrchestrator::new(
        migration_pool.unwrap_or_else(|| pool.clone()),
        module.clone(),
    )
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
    let base_url = config.url.clone();
    if config.engine == DatabaseEngine::Postgres {
        let tenant_id = require_postgres_rls_tenant_id().map_err(|error| error.to_string())?;
        let organization_id =
            require_postgres_rls_organization_id().map_err(|error| error.to_string())?;
        config.url = postgres_url_with_deployment_scope(&base_url, tenant_id, organization_id)
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

/// Environment-driven bootstrap with a dedicated migration pool: migrations
/// run on the BASE url (no deployment scope, no statement_timeout guard) so
/// long-running DDL is not cancelled by the runtime's 30 s guard.
pub async fn bootstrap_knowledgebase_database_from_env_with_dedicated_migrations()
-> Result<KnowledgebaseDatabaseHost, String> {
    let _ = dotenvy::dotenv();
    let config = DatabaseConfig::from_env("KNOWLEDGEBASE")
        .map_err(|error| format!("read knowledgebase database config failed: {error}"))?;
    let base_url = config.url.clone();
    let mut runtime_config = config.clone();
    if runtime_config.engine == DatabaseEngine::Postgres {
        let tenant_id = require_postgres_rls_tenant_id().map_err(|error| error.to_string())?;
        let organization_id =
            require_postgres_rls_organization_id().map_err(|error| error.to_string())?;
        runtime_config.url =
            postgres_url_with_deployment_scope(&base_url, tenant_id, organization_id)
                .map_err(|error| error.to_string())?;
    } else {
        return Err(
            "knowledgebase server persistence requires a PostgreSQL database url".to_string(),
        );
    }
    let pool = create_pool_from_config(runtime_config)
        .await
        .map_err(|error| format!("create knowledgebase database pool failed: {error}"))?;
    let migration_config = DatabaseConfig {
        url: base_url,
        ..config
    };
    let migration_pool = create_pool_from_config(migration_config)
        .await
        .map_err(|error| format!("create knowledgebase migration pool failed: {error}"))?;
    bootstrap_knowledgebase_database_with_migration_pool(pool, Some(migration_pool)).await
}

fn resolve_app_root() -> Result<PathBuf, String> {
    if let Ok(from_env) = std::env::var("SDKWORK_KNOWLEDGEBASE_APP_ROOT") {
        return Ok(PathBuf::from(from_env));
    }
    // Debug builds keep the compile-time checkout fallback for `cargo run`.
    // Release binaries must be pointed at the repository root explicitly: the
    // compile-time path belongs to the build machine and does not exist in
    // production.
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")));
    }
    Err(
        "SDKWORK_KNOWLEDGEBASE_APP_ROOT is required outside debug builds; set it to the knowledgebase repository root"
            .to_string(),
    )
}
