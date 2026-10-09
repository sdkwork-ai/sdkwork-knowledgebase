use sdkwork_api_knowledgebase_standalone_gateway::init_tracing;
use sdkwork_knowledgebase_contract::{
    parse_canonical_nonnegative_signed_i64, parse_canonical_positive_signed_i64,
};
use sdkwork_knowledgebase_worker::{
    health, run_polling_loop, MaintenanceConfig, MaintenancePollingConfig,
    WikiBackfillMaintenanceConfig, WikiDriveEventMaintenanceConfig,
};
use sdkwork_routes_knowledgebase_app_api::{bootstrap, KnowledgebaseRuntime};

#[tokio::main]
async fn main() {
    sdkwork_database_sqlx::enable_process_shared_database_pool();
    bootstrap::validate_process_config();
    init_tracing("worker");

    let database_url = bootstrap::resolve_database_url();
    let tenant_id = required_worker_tenant_id();
    let interval_ms = std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_POLL_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5_000);
    let outbox_limit = std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_OUTBOX_BATCH_SIZE")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| (1..=200).contains(value))
        .unwrap_or(50);
    let ingestion_job_limit =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_JOB_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(25);
    let ingestion_job_lease_seconds =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_JOB_LEASE_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| (30..=3_600).contains(value))
            .unwrap_or(300);
    let provider_migration_limit =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_PROVIDER_MIGRATION_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| (1..=200).contains(value))
            .unwrap_or(25);
    let provider_migration_lease_seconds =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_PROVIDER_MIGRATION_LEASE_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| (5..=3_600).contains(value))
            .unwrap_or(120);
    let worker_id = resolve_worker_id();
    let group_archive_limit =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_GROUP_ARCHIVE_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| (1..=200).contains(value))
            .unwrap_or(25);
    let ingestion_max_attempts =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_MAX_ATTEMPTS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| (1..=100).contains(value))
            .unwrap_or(20);
    let phase_timeout_seconds = std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_PHASE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (5..=3_600).contains(value))
        .unwrap_or(60);
    let wiki_backfill = resolve_wiki_backfill_config(tenant_id);
    let wiki_drive_events = resolve_wiki_drive_event_config(tenant_id);
    let health_addr = std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_HEALTH_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:18085".to_string());

    let runtime = KnowledgebaseRuntime::connect(&database_url, tenant_id)
        .await
        .expect("initialize knowledgebase worker runtime");
    runtime
        .readiness_check()
        .await
        .expect("knowledgebase worker readiness check failed");

    tracing::info!(
        database_engine = database_engine_label(&database_url),
        tenant_id,
        interval_ms,
        outbox_limit,
        ingestion_job_limit,
        ingestion_job_lease_seconds,
        provider_migration_limit,
        provider_migration_lease_seconds,
        worker_id = %worker_id,
        group_archive_limit,
        ingestion_max_attempts,
        phase_timeout_seconds,
        wiki_backfill_enabled = wiki_backfill.is_some(),
        wiki_drive_events_enabled = true,
        %health_addr,
        "starting knowledgebase worker loop"
    );

    let readiness = runtime.readiness_check_adapter();
    // Bind before spawning so a port collision fails fast with a clear exit
    // instead of panicking inside a detached task while the worker keeps
    // polling with no health endpoints.
    let health_listener = tokio::net::TcpListener::bind(&health_addr)
        .await
        .unwrap_or_else(|error| {
            eprintln!(
                "failed to bind knowledgebase worker health listener on {health_addr}: {error}"
            );
            std::process::exit(1);
        });
    tokio::spawn(async move {
        health::serve_worker_health(health_listener, readiness).await;
    });

    run_polling_loop(
        runtime,
        MaintenancePollingConfig {
            interval_ms,
            maintenance: MaintenanceConfig {
                worker_id,
                ingestion_job_lease: time::Duration::seconds(ingestion_job_lease_seconds as i64),
                provider_migration_lease: std::time::Duration::from_secs(
                    provider_migration_lease_seconds,
                ),
                outbox_limit,
                ingestion_job_limit,
                provider_migration_limit,
                group_archive_limit,
                ingestion_max_attempts,
                wiki_backfill,
                wiki_drive_events,
                phase_timeout: std::time::Duration::from_secs(phase_timeout_seconds),
            },
        },
    )
    .await;
}

fn resolve_worker_id() -> String {
    for key in [
        "SDKWORK_KNOWLEDGEBASE_WORKER_ID",
        "POD_UID",
        "HOSTNAME",
        "COMPUTERNAME",
    ] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() && value.chars().count() <= 255 {
                return value.to_string();
            }
        }
    }
    format!("knowledgebase-worker-process-{}", std::process::id())
}

fn required_worker_tenant_id() -> u64 {
    let value = std::env::var("SDKWORK_KNOWLEDGEBASE_TENANT_ID")
        .expect("SDKWORK_KNOWLEDGEBASE_TENANT_ID is required for the knowledgebase worker");
    parse_canonical_positive_signed_i64(&value)
        .expect("SDKWORK_KNOWLEDGEBASE_TENANT_ID must be a canonical positive signed BIGINT")
}

fn resolve_wiki_backfill_config(tenant_id: u64) -> Option<WikiBackfillMaintenanceConfig> {
    let organization_id =
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ORGANIZATION_ID").ok();
    let actor_id = std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ACTOR_ID").ok();
    match (organization_id, actor_id) {
        (None, None) => None,
        (Some(organization_id), Some(actor_id)) => {
            let organization_id = parse_canonical_nonnegative_signed_i64(&organization_id)
                .expect("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ORGANIZATION_ID must be a canonical nonnegative signed BIGINT");
            let actor_id = parse_canonical_positive_signed_i64(&actor_id)
                .expect("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ACTOR_ID must be a canonical positive signed BIGINT");
            let page_size = match std::env::var(
                "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_BATCH_SIZE",
            ) {
                Ok(value) => value
                    .parse::<u32>()
                    .ok()
                    .filter(|value| (1..=200).contains(value))
                    .expect(
                        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_BATCH_SIZE must be between 1 and 200",
                    ),
                Err(_) => 25,
            };
            Some(WikiBackfillMaintenanceConfig {
                tenant_id,
                organization_id,
                actor_id,
                page_size,
            })
        }
        _ => panic!(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ORGANIZATION_ID and SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ACTOR_ID must be configured together"
        ),
    }
}

fn resolve_wiki_drive_event_config(tenant_id: u64) -> WikiDriveEventMaintenanceConfig {
    let actor_id = std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID")
        .ok()
        .and_then(|value| parse_canonical_positive_signed_i64(&value).ok())
        .unwrap_or_else(|| {
            panic!(
                "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID is required and must be a canonical positive signed BIGINT"
            )
        });
    WikiDriveEventMaintenanceConfig {
        tenant_id,
        organization_id:
            sdkwork_routes_knowledgebase_app_api::bootstrap::resolve_deployment_organization_id(),
        actor_id,
        checkpoint_page_size: bounded_u32_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_CHECKPOINT_PAGE_SIZE",
            50,
            200,
        ),
        event_batch_size: bounded_u32_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_BATCH_SIZE",
            25,
            100,
        ),
        lease_seconds: bounded_u64_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_LEASE_SECONDS",
            120,
            3_600,
        ),
        retry_delay_seconds: bounded_u64_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_RETRY_DELAY_SECONDS",
            30,
            86_400,
        ),
        max_attempts: bounded_u32_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_MAX_ATTEMPTS",
            20,
            100,
        ),
        source_batch_size: bounded_u32_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_BATCH_SIZE",
            10,
            100,
        ),
        source_lease_seconds: bounded_u64_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_LEASE_SECONDS",
            120,
            3_600,
        ),
        source_retry_delay_seconds: bounded_u64_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_RETRY_DELAY_SECONDS",
            30,
            86_400,
        ),
        source_max_attempts: bounded_u32_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_MAX_ATTEMPTS",
            10,
            100,
        ),
        delivery_renewal_page_size: bounded_u32_env(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_DELIVERY_RENEWAL_PAGE_SIZE",
            50,
            200,
        ),
    }
}

fn bounded_u32_env(name: &str, default: u32, max: u32) -> u32 {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u32>()
            .ok()
            .filter(|value| (1..=max).contains(value))
            .unwrap_or_else(|| panic!("{name} must be between 1 and {max}")),
        Err(std::env::VarError::NotPresent) => default,
        Err(error) => panic!("{name} could not be read: {error}"),
    }
}

fn bounded_u64_env(name: &str, default: u64, max: u64) -> u64 {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| (1..=max).contains(value))
            .unwrap_or_else(|| panic!("{name} must be between 1 and {max}")),
        Err(std::env::VarError::NotPresent) => default,
        Err(error) => panic!("{name} could not be read: {error}"),
    }
}

fn database_engine_label(database_url: &str) -> &'static str {
    let normalized = database_url.trim().to_ascii_lowercase();
    if normalized.starts_with("postgres://") || normalized.starts_with("postgresql://") {
        "postgres"
    } else {
        "other"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        database_engine_label, required_worker_tenant_id, resolve_wiki_backfill_config,
        resolve_wiki_drive_event_config, resolve_worker_id,
    };
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const WIKI_EVENT_ENV_KEYS: [&str; 10] = [
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_CHECKPOINT_PAGE_SIZE",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_BATCH_SIZE",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_LEASE_SECONDS",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_RETRY_DELAY_SECONDS",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_MAX_ATTEMPTS",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_BATCH_SIZE",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_LEASE_SECONDS",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_RETRY_DELAY_SECONDS",
        "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_MAX_ATTEMPTS",
    ];

    #[test]
    fn worker_tenant_id_requires_a_canonical_positive_signed_bigint() {
        for invalid in ["0", "01", "+1", " 1", "1 ", "tenant", "9223372036854775808"] {
            std::env::set_var("SDKWORK_KNOWLEDGEBASE_TENANT_ID", invalid);
            assert!(
                std::panic::catch_unwind(required_worker_tenant_id).is_err(),
                "{invalid} must be rejected"
            );
        }
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_TENANT_ID", "9223372036854775807");
        assert_eq!(required_worker_tenant_id(), i64::MAX as u64);
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_TENANT_ID");
    }

    #[test]
    fn worker_log_database_label_never_contains_a_connection_secret() {
        assert_eq!(
            database_engine_label("postgres://user:secret@db.internal/knowledge"),
            "postgres"
        );
        assert_eq!(
            database_engine_label("other://data/knowledgebase.db"),
            "other"
        );
    }

    #[test]
    fn worker_identity_prefers_explicit_stable_identity() {
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_WORKER_ID", "pod-uid-123");
        assert_eq!(resolve_worker_id(), "pod-uid-123");
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_WORKER_ID");
    }

    #[test]
    fn wiki_backfill_requires_paired_explicit_scope_and_actor() {
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ORGANIZATION_ID");
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ACTOR_ID");
        assert_eq!(resolve_wiki_backfill_config(7), None);

        std::env::set_var(
            "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ORGANIZATION_ID",
            "0",
        );
        assert!(std::panic::catch_unwind(|| resolve_wiki_backfill_config(7)).is_err());
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ACTOR_ID", "42");
        let config = resolve_wiki_backfill_config(7).expect("Wiki backfill config");
        assert_eq!(config.tenant_id, 7);
        assert_eq!(config.organization_id, 0);
        assert_eq!(config.actor_id, 42);
        assert_eq!(config.page_size, 25);

        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ORGANIZATION_ID");
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_BACKFILL_ACTOR_ID");
    }

    #[test]
    fn wiki_drive_event_config_requires_an_explicit_actor_and_uses_bounded_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_wiki_event_env();
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID", "42");
        assert!(std::panic::catch_unwind(|| resolve_wiki_drive_event_config(7)).is_err());

        std::env::set_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID", "9001");
        let config = resolve_wiki_drive_event_config(7);
        assert_eq!(config.tenant_id, 7);
        assert_eq!(config.organization_id, 42);
        assert_eq!(config.actor_id, 9001);
        assert_eq!(config.checkpoint_page_size, 50);
        assert_eq!(config.event_batch_size, 25);
        assert_eq!(config.lease_seconds, 120);
        assert_eq!(config.retry_delay_seconds, 30);
        assert_eq!(config.max_attempts, 20);
        assert_eq!(config.source_batch_size, 10);
        assert_eq!(config.source_lease_seconds, 120);
        assert_eq!(config.source_retry_delay_seconds, 30);
        assert_eq!(config.source_max_attempts, 10);

        clear_wiki_event_env();
        std::env::remove_var("SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID");
    }

    #[test]
    fn wiki_drive_event_config_rejects_explicit_invalid_bounds() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_wiki_event_env();
        std::env::set_var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID", "9001");
        for (name, value) in [
            (
                "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_CHECKPOINT_PAGE_SIZE",
                "201",
            ),
            ("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_BATCH_SIZE", "0"),
            (
                "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_LEASE_SECONDS",
                "invalid",
            ),
            (
                "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_RETRY_DELAY_SECONDS",
                "86401",
            ),
            (
                "SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_MAX_ATTEMPTS",
                "101",
            ),
            ("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_BATCH_SIZE", "101"),
        ] {
            std::env::set_var(name, value);
            assert!(
                std::panic::catch_unwind(|| resolve_wiki_drive_event_config(7)).is_err(),
                "{name}={value} must be rejected"
            );
            std::env::remove_var(name);
        }
        clear_wiki_event_env();
    }

    fn clear_wiki_event_env() {
        for name in WIKI_EVENT_ENV_KEYS {
            std::env::remove_var(name);
        }
    }
}
