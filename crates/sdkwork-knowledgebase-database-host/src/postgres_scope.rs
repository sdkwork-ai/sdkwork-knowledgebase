//! PostgreSQL deployment scope injection for RLS-protected Knowledgebase pools.
//!
//! Every Knowledgebase server pool binds one fixed tenant and one non-zero organization
//! through PostgreSQL connection `options` (`app.current_tenant_id` /
//! `app.current_organization_id`). Business RLS policies require both columns and use
//! `FORCE ROW LEVEL SECURITY`, so a pool without the deployment scope silently reads zero
//! rows. This module is the single source of truth for resolving and injecting that scope;
//! both the repository bootstrap (`sdkwork-knowledgebase-repository-sqlx`) and the
//! one-shot lifecycle bootstrap (`bootstrap_knowledgebase_database_from_env`) reuse it.

use sdkwork_database_sqlx::PoolError;
use sdkwork_knowledgebase_contract::{
    parse_canonical_nonnegative_signed_i64, parse_canonical_positive_signed_i64,
};
use sdkwork_knowledgebase_observability::is_production_like_environment;
use url::Url;

/// Session variable read by RLS policies on tenant-scoped tables.
pub const POSTGRES_TENANT_SESSION_KEY: &str = "app.current_tenant_id";
pub const POSTGRES_ORGANIZATION_SESSION_KEY: &str = "app.current_organization_id";

/// Returns the tenant id required for PostgreSQL pool checkout, failing closed in
/// production-like environments.
pub fn require_postgres_rls_tenant_id() -> Result<u64, PoolError> {
    match std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID") {
        Ok(value) => parse_canonical_positive_signed_i64(&value).map_err(|_| {
            PoolError::InvalidUrl(
                "SDKWORK_KNOWLEDGEBASE_TENANT_ID must be a canonical positive signed BIGINT"
                    .to_string(),
            )
        }),
        Err(std::env::VarError::NotPresent) if !is_production_like_environment() => Ok(1),
        Err(_) => Err(PoolError::InvalidUrl(
            "SDKWORK_KNOWLEDGEBASE_TENANT_ID must be set for production-like Postgres deployments"
                .to_string(),
        )),
    }
}

/// Returns the organization id required for PostgreSQL pool checkout, failing closed in
/// production-like environments. Non-production fallback is `0` to stay compatible with
/// fixtures that predate organization isolation.
pub fn require_postgres_rls_organization_id() -> Result<u64, PoolError> {
    match std::env::var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID") {
        Ok(value) => parse_canonical_nonnegative_signed_i64(&value).map_err(|_| {
            PoolError::InvalidUrl(
                "SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID must be a canonical nonnegative signed BIGINT"
                    .to_string(),
            )
        }),
        Err(std::env::VarError::NotPresent) if !is_production_like_environment() => Ok(0),
        Err(_) => Err(PoolError::InvalidUrl(
            "SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID must be set for production-like Postgres deployments"
                .to_string(),
        )),
    }
}

/// Injects the deployment-owned tenant/organization scope (and a 30 s `statement_timeout`)
/// into a PostgreSQL connection URL.
///
/// Caller-supplied `options` that already carry `app.current_tenant_id` or
/// `app.current_organization_id` are rejected so no two sources of truth can compete for the
/// RLS session context.
pub fn postgres_url_with_deployment_scope(
    database_url: &str,
    tenant_id: u64,
    organization_id: u64,
) -> Result<String, PoolError> {
    let mut url = Url::parse(database_url)
        .map_err(|error| PoolError::InvalidUrl(format!("invalid PostgreSQL URL: {error}")))?;
    let mut query_pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let mut options_index = None;
    for (index, (key, value)) in query_pairs.iter().enumerate() {
        if !key.eq_ignore_ascii_case("options") {
            continue;
        }
        if options_index.replace(index).is_some() {
            return Err(PoolError::DatabaseConfig(
                "PostgreSQL URL must not contain duplicate options parameters".to_string(),
            ));
        }
        let normalized_options = value.to_ascii_lowercase();
        if normalized_options.contains(POSTGRES_TENANT_SESSION_KEY)
            || normalized_options.contains(POSTGRES_ORGANIZATION_SESSION_KEY)
        {
            return Err(PoolError::DatabaseConfig(
                "PostgreSQL URL must not set deployment-owned tenant or organization scope"
                    .to_string(),
            ));
        }
    }

    let scope_options = format!(
        "-c {POSTGRES_TENANT_SESSION_KEY}={tenant_id} -c {POSTGRES_ORGANIZATION_SESSION_KEY}={organization_id} -c statement_timeout=30000"
    );
    if let Some(index) = options_index {
        let existing = query_pairs[index].1.trim();
        query_pairs[index].1 = if existing.is_empty() {
            scope_options
        } else {
            format!("{existing} {scope_options}")
        };
    } else {
        query_pairs.push(("options".to_string(), scope_options));
    }

    url.query_pairs_mut().clear().extend_pairs(query_pairs);
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_option_preserves_existing_connection_options() {
        let configured = postgres_url_with_deployment_scope(
            "postgresql://app:secret@localhost/sdkwork_ai_dev?sslmode=verify-full&options=-c%20search_path%3Dsdkwork_ai_dev%2Cpublic",
            42,
            7,
        )
        .expect("tenant-scoped URL");
        let parsed = Url::parse(&configured).expect("valid URL");
        let options = parsed
            .query_pairs()
            .find(|(key, _)| key == "options")
            .map(|(_, value)| value.into_owned())
            .expect("options parameter");
        assert_eq!(
            options,
            "-c search_path=sdkwork_ai_dev,public -c app.current_tenant_id=42 -c app.current_organization_id=7 -c statement_timeout=30000"
        );
    }

    #[test]
    fn tenant_option_is_added_when_options_are_absent() {
        let configured =
            postgres_url_with_deployment_scope("postgresql://app@localhost/sdkwork_ai_dev", 7, 11)
                .expect("tenant-scoped URL");
        let parsed = Url::parse(&configured).expect("valid URL");
        assert!(parsed.query_pairs().any(|(key, value)| key == "options"
            && value
                == "-c app.current_tenant_id=7 -c app.current_organization_id=11 -c statement_timeout=30000"));
    }

    #[test]
    fn caller_owned_tenant_option_is_rejected() {
        let error = postgres_url_with_deployment_scope(
            "postgresql://app@localhost/sdkwork_ai_dev?options=-c%20app.current_tenant_id%3D99",
            7,
            11,
        )
        .expect_err("caller tenant option must fail closed");
        assert!(error.to_string().contains("deployment-owned"));
    }

    #[test]
    fn duplicate_options_are_rejected() {
        let error = postgres_url_with_deployment_scope(
            "postgresql://app@localhost/sdkwork_ai_dev?options=-c%20timezone%3DUTC&options=-c%20search_path%3Dsdkwork_ai_dev",
            7,
            11,
        )
        .expect_err("duplicate options must fail closed");
        assert!(error.to_string().contains("duplicate options"));
    }
}
