//! Database schema, row mapping, and SQL query modules.
//!
//! This module aggregates database access modules from the parent flat layout.
//! New database modules should be placed directly under this directory.

pub mod bootstrap;
pub mod pgvector_schema;
pub mod postgres;
pub mod postgres_tenant_session;
pub mod sql_timestamp;

pub use bootstrap::{
    connect_knowledgebase_any_pool_from_url, connect_knowledgebase_pool_from_env,
    connect_knowledgebase_pool_from_url, database_config_from_url,
    knowledgebase_database_engine_from_url, knowledgebase_process_pool_budget_from_url,
    KnowledgebaseDatabasePool, KnowledgebaseProcessPoolBudget,
};
pub use pgvector_schema::{
    pgvector_schema, resolve_pgvector_schema, DEFAULT_PGVECTOR_SCHEMA, PGVECTOR_SCHEMA_ENV,
};
pub use postgres::{
    connect_postgres_and_install_schema, connect_postgres_pool,
    connect_postgres_via_framework_lifecycle, is_postgres_database_url, postgres_health_check,
    PostgresRepositoryError,
};
pub use postgres_tenant_session::{
    require_postgres_rls_organization_id, require_postgres_rls_tenant_id,
    resolve_postgres_rls_tenant_id, set_postgres_session_organization_id,
    set_postgres_session_tenant_id, POSTGRES_ORGANIZATION_SESSION_KEY, POSTGRES_TENANT_SESSION_KEY,
};

pub async fn connect_knowledgebase_and_install_schema(
    database_url: &str,
) -> Result<sqlx::AnyPool, sqlx::Error> {
    // 服务端权威持久化仅支持 PostgreSQL（DATABASE_SPEC：authoritative-server）
    postgres::connect_postgres_via_framework_lifecycle(database_url)
        .await
        .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))
}

pub async fn knowledgebase_health_check(pool: &sqlx::AnyPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT 1::bigint")
        .fetch_one(pool)
        .await
        .map(|_| ())
}
