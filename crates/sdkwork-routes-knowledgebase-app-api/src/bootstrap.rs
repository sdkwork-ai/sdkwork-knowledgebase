use sdkwork_knowledgebase_contract::{
    parse_canonical_nonnegative_signed_i64, parse_canonical_positive_signed_i64,
};
use sdkwork_utils_rust::is_blank;

pub use sdkwork_knowledgebase_observability::{
    is_development_environment, is_production_like_environment, knowledgebase_environment,
};

/// Resolves the authoritative PostgreSQL URL and fails closed on missing or SQLite config.
pub fn resolve_database_url() -> String {
    match std::env::var("SDKWORK_DATABASE_URL") {
        Ok(url)
            if matches!(
                url.trim().to_ascii_lowercase().as_str(),
                value if value.starts_with("postgres://") || value.starts_with("postgresql://")
            ) =>
        {
            url
        }
        Ok(_) => {
            eprintln!("SDKWORK_DATABASE_URL must use PostgreSQL for the application server");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("SDKWORK_DATABASE_URL must be set for the application server");
            std::process::exit(1);
        }
    }
}

pub fn validate_process_config() {
    validate_snowflake_node_id_for_production();
    validate_secrets_encryption_for_production();

    let organization_id = resolve_deployment_organization_id();
    if organization_id == 0 && !is_development_environment() {
        eprintln!(
            "SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID must be set when SDKWORK_KNOWLEDGEBASE_ENVIRONMENT is not development"
        );
        std::process::exit(1);
    }

    let tenant_id = std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID")
        .ok()
        .map(|value| {
            parse_canonical_positive_signed_i64(&value).unwrap_or_else(|_| {
                eprintln!(
                    "SDKWORK_KNOWLEDGEBASE_TENANT_ID must be a canonical positive signed BIGINT"
                );
                std::process::exit(1);
            })
        })
        .unwrap_or(0);
    if tenant_id == 0 && !is_development_environment() {
        eprintln!(
            "SDKWORK_KNOWLEDGEBASE_TENANT_ID must be set when SDKWORK_KNOWLEDGEBASE_ENVIRONMENT is not development"
        );
        std::process::exit(1);
    }
}

pub fn resolve_deployment_organization_id() -> u64 {
    std::env::var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID")
        .ok()
        .map(|value| {
            parse_canonical_nonnegative_signed_i64(&value).unwrap_or_else(|_| {
                eprintln!(
                    "SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID must be a canonical signed BIGINT"
                );
                std::process::exit(1);
            })
        })
        .unwrap_or(0)
}

fn validate_secrets_encryption_for_production() {
    if !is_production_like_environment() {
        return;
    }

    if sdkwork_intelligence_knowledgebase_service::wechat::encryption_key_configured() {
        return;
    }

    eprintln!(
        "SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE or SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY must be set for production-like environments"
    );
    std::process::exit(1);
}

fn validate_snowflake_node_id_for_production() {
    let node_id = std::env::var("SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID").ok();
    let Some(node_id) = node_id else {
        return;
    };
    if is_blank(Some(node_id.as_str())) {
        eprintln!("SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID must not be empty");
        std::process::exit(1);
    }
    if let Err(error) =
        sdkwork_intelligence_knowledgebase_repository_sqlx::SnowflakeKnowledgeIdGenerator::from_node_id_config(
            Some(node_id.trim()),
        )
    {
        eprintln!("invalid SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID: {error}");
        std::process::exit(1);
    }
    if is_production_like_environment()
        && !std::env::var("SDKWORK_KNOWLEDGEBASE_ALLOW_STATIC_SNOWFLAKE_NODE_ID")
            .ok()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
    {
        eprintln!(
            "static SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID requires SDKWORK_KNOWLEDGEBASE_ALLOW_STATIC_SNOWFLAKE_NODE_ID=true in production-like environments"
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_environment_requires_explicit_value() {
        if std::env::var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT").is_ok() {
            return;
        }
        assert!(!is_development_environment());
    }

    #[test]
    fn production_like_environment_is_not_development_by_default() {
        if std::env::var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT").is_ok() {
            return;
        }
        assert!(!is_development_environment());
    }

    #[test]
    fn process_config_requires_organization_without_runtime_bypass() {
        if std::env::var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT").is_ok() {
            return;
        }
        let organization_id = std::env::var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID").ok();
        assert!(organization_id.is_none());
    }
}
