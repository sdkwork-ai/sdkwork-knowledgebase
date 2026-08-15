use sdkwork_api_knowledgebase_assembly::assemble_api_router_from_environment_with_group_launch_ticket_consumer;
use sdkwork_api_knowledgebase_standalone_gateway::{
    resolve_group_launch_ticket_consumer_from_env, serve_router_with_runtime_shutdown,
    RedisRuntimeConfig,
};
use sdkwork_iam_web_adapter::{
    build_web_framework_builder_with_open_api_prefixes,
    iam_web_request_context_resolver_from_database_pool_for_audiences,
    resolve_iam_database_pool_from_env, IamAuditEmitter, IamSecurityEventEmitter,
};
use sdkwork_web_bootstrap::{
    shared_concurrent_admission_store, shared_idempotency_store, shared_rate_limit_store,
    ComposedApiAssembly, CompositeReadinessCheck, ReadinessCheck, RedisReadinessCheck,
};
use std::sync::Arc;

/// Application audience of the access tokens issued for the Knowledgebase PC app.
/// IAM tokens carry `aud == app_id`; the production claim policy validates it.
const APPLICATION_ID: &str = "sdkwork-knowledgebase-pc";

/// Emitter identity recorded in IAM audit/security-event rows.
const AUDIT_APP_ID: &str = "sdkwork-knowledgebase";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sdkwork_database_sqlx::enable_process_shared_database_pool();
    let environment = unify_process_environment();
    let requires_hardening = matches!(environment.as_str(), "production" | "staging");

    let listen_addr = std::env::var("SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND")
        .unwrap_or_else(|_| "0.0.0.0:18081".to_string());
    let group_launch_ticket_consumer = resolve_group_launch_ticket_consumer_from_env().await?;
    let knowledgebase = assemble_api_router_from_environment_with_group_launch_ticket_consumer(
        group_launch_ticket_consumer,
    )
    .await?;
    let iam = sdkwork_api_iam_assembly::assemble_app_api_contribution().await?;
    let mut composed =
        ComposedApiAssembly::try_compose("SDKWork Knowledgebase API", vec![knowledgebase, iam])?;

    // Resolver bound to the process-shared database pool with the production
    // issuer/audience claim policy attached when the deployment is production-like.
    let iam_pool = resolve_iam_database_pool_from_env()
        .await
        .ok_or("IAM database pool could not be resolved; configure the database env contract")?;
    let resolver = iam_web_request_context_resolver_from_database_pool_for_audiences(
        iam_pool.clone(),
        &[APPLICATION_ID],
    )
    .await?;

    let manifest = composed.route_manifest.clone();
    let mut framework = build_web_framework_builder_with_open_api_prefixes(
        resolver,
        manifest,
        infra_public_path_prefixes(),
        sdkwork_api_knowledgebase_assembly::knowledgebase_open_api_prefixes(),
    );
    // The observability layer (wrap_router_with_metrics in serve_router) owns
    // the process-level /metrics surface; skip the framework's duplicate
    // infra /metrics route so the gateway boots without overlapping routes.
    framework = framework.skip_infra_metrics();

    // Distributed Redis stores make rate limiting, idempotency, and concurrent
    // admission multi-replica safe. Production-like boot fails closed without them.
    let redis = RedisRuntimeConfig::from_env()?;
    if let Some(redis) = redis {
        let store_prefix = format!("{}:web", redis.key_prefix());
        framework = framework
            .rate_limit_store(shared_rate_limit_store(
                redis.url(),
                format!("{store_prefix}:rate-limit"),
            )?)
            .idempotency_store(shared_idempotency_store(
                redis.url(),
                format!("{store_prefix}:idempotency"),
            )?)
            .concurrent_admission_store(shared_concurrent_admission_store(
                redis.url(),
                format!("{store_prefix}:concurrent-admission"),
            )?);
        composed.readiness_check = Arc::new(CompositeReadinessCheck::new(vec![
            composed.readiness_check.clone(),
            Arc::new(RedisReadinessCheck::new(redis.url())?) as Arc<dyn ReadinessCheck>,
        ]));
    } else if requires_hardening {
        return Err(
            "production/staging Knowledgebase gateway requires Redis; configure \
             SDKWORK_KNOWLEDGEBASE_REDIS_URL or SDKWORK_KNOWLEDGEBASE_REDIS_ENABLED"
                .into(),
        );
    }

    // Durable audit and security-event persistence through the IAM web adapter.
    if requires_hardening {
        let postgres_pool = iam_pool
            .as_postgres()
            .cloned()
            .ok_or("production/staging Knowledgebase gateway requires PostgreSQL")?;
        framework = framework
            .audit_emitter(Arc::new(IamAuditEmitter::new(
                postgres_pool.clone(),
                AUDIT_APP_ID,
                environment.as_str(),
            )))
            .security_event_emitter(Arc::new(IamSecurityEventEmitter::new(
                postgres_pool,
                environment.as_str(),
            )));
    }

    let router = composed.into_hosted(framework).router;
    serve_router_with_runtime_shutdown(
        &listen_addr,
        "sdkwork-api-knowledgebase-standalone-gateway",
        router,
    )
    .await?;
    Ok(())
}

fn infra_public_path_prefixes() -> Vec<String> {
    sdkwork_web_bootstrap::infra_public_path_prefixes()
}

/// Resolves the effective deployment environment and makes every environment
/// detector in the process agree on it.
///
/// `SDKWORK_KNOWLEDGEBASE_ENVIRONMENT` is the single authority for the
/// Knowledgebase gateway; `SDKWORK_ENVIRONMENT`/`SDKWORK_IAM_ENVIRONMENT`/
/// `SDKWORK_IM_ENVIRONMENT` are honored as a fallback so the SDKWork web
/// framework (whose default is production) and the knowledgebase-local
/// environment checks can never silently disagree. The unified value is
/// exported as `SDKWORK_ENVIRONMENT` before any framework component resolves it.
fn unify_process_environment() -> String {
    let effective = effective_environment();
    let framework_value = match effective.as_str() {
        "dev" | "development" => "development",
        "test" | "testing" => "test",
        // production, staging, and any unknown value fail closed to production.
        _ => "production",
    };
    std::env::set_var("SDKWORK_ENVIRONMENT", framework_value);
    effective
}

fn effective_environment() -> String {
    if let Some(value) = std::env::var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
    {
        return value;
    }
    [
        "SDKWORK_ENVIRONMENT",
        "SDKWORK_IAM_ENVIRONMENT",
        "SDKWORK_IM_ENVIRONMENT",
    ]
    .iter()
    .find_map(|key| std::env::var(key).ok())
    .map(|value| value.trim().to_ascii_lowercase())
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "production".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Environment variables are process-global; serialize env-mutating tests so
    // parallel test threads cannot observe each other's configuration.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F>(entries: &[(&str, &str)], run: F)
    where
        F: FnOnce() + std::panic::UnwindSafe,
    {
        let _guard = ENV_LOCK.lock().expect("env test lock poisoned");
        let previous = entries
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in entries {
            std::env::set_var(key, value);
        }
        let result = std::panic::catch_unwind(run);
        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        assert!(result.is_ok());
    }

    #[test]
    fn knowledgebase_environment_is_the_authority() {
        with_env(
            &[
                ("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", "production"),
                ("SDKWORK_ENVIRONMENT", "development"),
            ],
            || {
                let environment = unify_process_environment();
                assert_eq!(environment, "production");
                assert_eq!(std::env::var("SDKWORK_ENVIRONMENT").unwrap(), "production");
            },
        );
    }

    #[test]
    fn framework_environment_is_the_fallback() {
        with_env(
            &[
                ("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", ""),
                ("SDKWORK_ENVIRONMENT", "test"),
            ],
            || {
                let environment = unify_process_environment();
                assert_eq!(environment, "test");
                assert_eq!(std::env::var("SDKWORK_ENVIRONMENT").unwrap(), "test");
            },
        );
    }

    #[test]
    fn defaults_fail_closed_to_production() {
        with_env(
            &[
                ("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT", ""),
                ("SDKWORK_ENVIRONMENT", ""),
                ("SDKWORK_IAM_ENVIRONMENT", ""),
                ("SDKWORK_IM_ENVIRONMENT", ""),
            ],
            || {
                let environment = unify_process_environment();
                assert_eq!(environment, "production");
            },
        );
    }
}
