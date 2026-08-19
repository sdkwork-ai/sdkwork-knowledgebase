//! Postgres session tenant context for RLS policies (Phase 2.1/2.2).
//!
//! The RLS session keys and deployment-scope resolution live in
//! `sdkwork-knowledgebase-database-host::postgres_scope` (single source of truth shared with
//! the lifecycle bootstrap); this module re-exports them and adds per-connection session
//! helpers.

use sdkwork_knowledgebase_observability::deployment_tenant_id;
use sqlx::Executor;

pub use sdkwork_knowledgebase_database_host::postgres_scope::{
    require_postgres_rls_organization_id, require_postgres_rls_tenant_id,
    POSTGRES_ORGANIZATION_SESSION_KEY, POSTGRES_TENANT_SESSION_KEY,
};

/// Resolves the deployment-bound tenant id used for Postgres RLS session context.
pub fn resolve_postgres_rls_tenant_id() -> u64 {
    deployment_tenant_id()
}

/// Sets `app.current_tenant_id` for explicit administrative or integration-test connections.
///
/// Deployable one-tenant-per-process runtimes inject this setting through the PostgreSQL
/// connection URL before the process-shared pool is created. Request-shared multi-tenant
/// checkout remains unsupported until transaction-local context is implemented.
pub async fn set_postgres_session_tenant_id<'e, E>(
    executor: E,
    tenant_id: u64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query("SELECT set_config($1, $2, false)")
        .bind(POSTGRES_TENANT_SESSION_KEY)
        .bind(tenant_id.to_string())
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn set_postgres_session_organization_id<'e, E>(
    executor: E,
    organization_id: u64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query("SELECT set_config($1, $2, false)")
        .bind(POSTGRES_ORGANIZATION_SESSION_KEY)
        .bind(organization_id.to_string())
        .execute(executor)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        require_postgres_rls_organization_id, require_postgres_rls_tenant_id,
        POSTGRES_ORGANIZATION_SESSION_KEY, POSTGRES_TENANT_SESSION_KEY,
    };
    use std::sync::{Mutex, MutexGuard};

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn env_test_guard() -> MutexGuard<'static, ()> {
        ENV_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    #[test]
    fn tenant_session_key_matches_adr() {
        assert_eq!(POSTGRES_TENANT_SESSION_KEY, "app.current_tenant_id");
        assert_eq!(
            POSTGRES_ORGANIZATION_SESSION_KEY,
            "app.current_organization_id"
        );
    }

    #[test]
    fn require_tenant_id_defaults_to_one_in_development() {
        let _guard = env_test_guard();
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_TENANT_ID");
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "development");
        assert_eq!(
            require_postgres_rls_tenant_id().expect("development default"),
            1
        );
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT");
    }

    #[test]
    fn require_tenant_id_fails_closed_in_production_like() {
        let _guard = env_test_guard();
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_TENANT_ID");
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "production");
        assert!(require_postgres_rls_tenant_id().is_err());
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT");
    }

    #[test]
    fn require_organization_id_defaults_to_personal_scope() {
        let _guard = env_test_guard();
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID");
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "production");
        assert_eq!(
            require_postgres_rls_organization_id().expect("personal scope default"),
            0
        );
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID");
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT");
    }
}
