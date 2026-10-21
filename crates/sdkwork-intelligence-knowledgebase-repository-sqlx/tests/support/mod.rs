//! Shared PostgreSQL integration-test harness.
//!
//! Every Knowledgebase table is protected by `FORCE ROW LEVEL SECURITY`, so a test pool must
//! carry the same `app.current_tenant_id` / `app.current_organization_id` session scope that the
//! production bootstrap injects. The schema is installed once through the production lifecycle,
//! then the pool is opened from the deployment-scoped URL so every connection (not only the first
//! checked-out one) sees the scope.

#![allow(dead_code)]

use std::time::Duration;

use sdkwork_database_config::workspace_database::normalize_workspace_postgres_url;
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    connect_postgres_and_install_schema, is_postgres_database_url,
};

/// Connection capacity for the small per-test pools.
const SCOPED_POOL_MAX_CONNECTIONS: u32 = 8;

pub fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

/// Removes wiki integration fixtures for one tenant/organization, children first.
///
/// The wiki tests share one PostgreSQL schema and many fixed fixture ids, so each test
/// starts from a clean scope.
pub async fn clean_wiki_scope(pool: &sqlx::AnyPool, tenant_id: u64, organization_id: u64) {
    for table in [
        "kb_drive_event_inbox",
        "kb_drive_source_checkpoint",
        "kb_source_file_rendition",
        "kb_source_file_projection",
        "kb_site_publication",
        "kb_outbox_event",
        "kb_audit_event",
        "kb_source",
        "kb_space",
    ] {
        let statement =
            format!("DELETE FROM {table} WHERE tenant_id = $1 AND organization_id = $2");
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(tenant_id as i64)
            .bind(organization_id as i64)
            .execute(pool)
            .await
            .expect("clean wiki integration fixtures");
    }
}

/// Installs/upgrades the schema and returns a pool whose sessions carry `tenant_id`/`organization_id`.
///
/// Returns `None` when no PostgreSQL URL is configured, so each caller keeps its existing
/// "skip when no database" behavior.
pub async fn scoped_pool(tenant_id: u64, organization_id: u64) -> Option<sqlx::AnyPool> {
    let base_url = optional_postgres_database_url()?;
    connect_postgres_and_install_schema(&base_url)
        .await
        .expect("install postgres knowledgebase schema")
        .close()
        .await;

    let normalized =
        normalize_workspace_postgres_url(base_url.trim()).expect("normalize postgres database url");
    let scoped_url =
        sdkwork_intelligence_knowledgebase_repository_sqlx::db::bootstrap::postgres_url_with_deployment_scope(
            &normalized,
            tenant_id,
            organization_id,
        )
        .expect("inject postgres RLS deployment scope");

    sqlx::any::install_default_drivers();
    Some(
        sqlx::any::AnyPoolOptions::new()
            .max_connections(SCOPED_POOL_MAX_CONNECTIONS)
            .acquire_timeout(Duration::from_secs(60))
            .connect(&scoped_url)
            .await
            .expect("connect postgres pool with RLS deployment scope"),
    )
}
