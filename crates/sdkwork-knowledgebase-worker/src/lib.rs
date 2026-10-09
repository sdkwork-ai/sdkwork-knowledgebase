use sdkwork_api_knowledgebase_standalone_gateway::shutdown_signal;
use sdkwork_intelligence_knowledgebase_service::{
    outbox::OutboxPublishBatchResult,
    ports::knowledge_wiki_persistence::WikiPersistenceScope,
    provider_migration::ProviderMigrationBatchResult,
    wiki_backfill::{
        RunWikiPublicationBackfillRequest, WikiPublicationBackfillDisposition,
        MAX_WIKI_BACKFILL_PAGE_SIZE,
    },
    wiki_event_consumer::{
        KnowledgeWikiDriveCheckpointPageResult, ProcessKnowledgeWikiDriveCheckpointPageRequest,
    },
    wiki_source_processor::{
        KnowledgeWikiSourceCheckpointPageResult, ProcessKnowledgeWikiSourceCheckpointPageRequest,
    },
};
use sdkwork_knowledgebase_drive::DomainOutboxDispatchResult;
use sdkwork_knowledgebase_observability::WORKER_PHASE_NAMES;
use sdkwork_routes_knowledgebase_app_api::KnowledgebaseRuntime;

pub mod health;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WikiBackfillMaintenanceConfig {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub actor_id: u64,
    pub page_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WikiDriveEventMaintenanceConfig {
    pub tenant_id: u64,
    pub organization_id: u64,
    pub actor_id: u64,
    pub checkpoint_page_size: u32,
    pub event_batch_size: u32,
    pub lease_seconds: u64,
    pub retry_delay_seconds: u64,
    pub max_attempts: u32,
    pub source_batch_size: u32,
    pub source_lease_seconds: u64,
    pub source_retry_delay_seconds: u64,
    pub source_max_attempts: u32,
    pub delivery_renewal_page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceConfig {
    pub worker_id: String,
    pub ingestion_job_lease: time::Duration,
    pub provider_migration_lease: std::time::Duration,
    pub outbox_limit: u32,
    pub ingestion_job_limit: u32,
    pub provider_migration_limit: u32,
    pub group_archive_limit: u32,
    pub ingestion_max_attempts: u32,
    pub wiki_backfill: Option<WikiBackfillMaintenanceConfig>,
    pub wiki_drive_events: WikiDriveEventMaintenanceConfig,
    /// Per-phase time budget. A phase that exceeds its budget keeps running in
    /// the background and is reaped by later ticks; it never blocks or starves
    /// the other maintenance domains.
    pub phase_timeout: std::time::Duration,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaintenanceTickState {
    pub wiki_checkpoint_cursor: Option<u64>,
    pub wiki_source_cursor: Option<u64>,
    pub wiki_delivery_cursor: Option<u64>,
    pub renew_wiki_event_deliveries: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenancePollingConfig {
    pub interval_ms: u64,
    pub maintenance: MaintenanceConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaintenanceTickResult {
    pub outbox_requeued: usize,
    pub outbox_published: usize,
    pub outbox_failed: usize,
    pub outbox_dead_lettered: usize,
    pub ingestion_jobs_processed: usize,
    pub provider_migration_phases_processed: usize,
    pub provider_migrations_completed: usize,
    pub provider_migrations_rolled_back: usize,
    pub provider_migrations_failed: usize,
    pub group_archives_processed: usize,
    pub wiki_publications_initialized: usize,
    pub wiki_publications_failed: usize,
    pub wiki_backfill_next_after_space_id: Option<u64>,
    pub wiki_backfill_failed_space_ids: Vec<u64>,
    pub wiki_drive_outbox_events_processed: usize,
    pub wiki_drive_outbox_events_delivered: usize,
    pub wiki_drive_outbox_events_failed: usize,
    pub wiki_drive_checkpoints_processed: usize,
    pub wiki_drive_events_applied: usize,
    pub wiki_drive_events_retried: usize,
    pub wiki_drive_events_dead_lettered: usize,
    pub wiki_drive_public_changes: usize,
    pub wiki_sources_claimed: usize,
    pub wiki_sources_ready: usize,
    pub wiki_sources_auto_published: usize,
    pub wiki_sources_retried: usize,
    pub wiki_sources_quarantined: usize,
    pub wiki_auto_publications_deferred: usize,
    pub wiki_drive_next_after_checkpoint_id: Option<u64>,
    pub wiki_drive_blocked_checkpoint_id: Option<u64>,
    pub wiki_drive_source_next_after_checkpoint_id: Option<u64>,
    pub wiki_drive_event_deliveries_renewed: usize,
    pub wiki_drive_event_delivery_relays_verified: usize,
    pub wiki_drive_event_delivery_failures: usize,
    pub wiki_drive_next_after_event_delivery_checkpoint_id: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum MaintenanceTickError {
    #[error("outbox batch failed: {0}")]
    Outbox(String),
    #[error("ingestion job batch failed: {0}")]
    Ingestion(String),
    #[error("Provider migration batch failed: {0}")]
    ProviderMigration(String),
    #[error("Wiki publication compensation batch failed: {0}")]
    WikiBackfill(String),
    #[error("Wiki Drive event batch failed: {0}")]
    WikiDriveEvents(String),
    #[error("Wiki source processing batch failed: {0}")]
    WikiSourceProcessing(String),
}

/// Per-phase result of a maintenance tick.
enum PhaseResult {
    Outbox(OutboxPublishBatchResult),
    Ingestion(usize),
    ProviderMigration(ProviderMigrationBatchResult),
    GroupArchives(usize),
    WikiBackfill(WikiBackfillPhaseResult),
    WikiDriveRelay(DomainOutboxDispatchResult),
    WikiDrive(KnowledgeWikiDriveCheckpointPageResult),
    WikiSources(KnowledgeWikiSourceCheckpointPageResult),
    WikiDelivery(
        sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::WikiDriveEventDeliveryRenewalPageResult,
    ),
}

type PhaseHandle = tokio::task::JoinHandle<Result<PhaseResult, MaintenanceTickError>>;

/// Outcome of one wiki backfill phase invocation, including the resume cursor and the
/// per-space outcome lists the maintenance loop uses for exponential-backoff cooldown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiBackfillPhaseResult {
    pub initialized: usize,
    pub failed: usize,
    pub next_after_space_id: Option<u64>,
    pub failed_space_ids: Vec<u64>,
    pub succeeded_space_ids: Vec<u64>,
}

/// In-memory cooldown state for a persistently failing backfill candidate. Cooldown is
/// process-local: with multiple worker replicas each replica keeps its own backoff clock,
/// which is safe because retries are idempotent provisioning operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WikiBackfillCooldown {
    failures: u32,
    retry_after: std::time::Instant,
}

const WIKI_BACKFILL_COOLDOWN_BASE_SECS: u64 = 30;
const WIKI_BACKFILL_COOLDOWN_MAX_SECS: u64 = 3_600;

/// In-flight handles for phases that exceeded their budget on a previous tick.
/// At most one instance of every phase may run at any time.
#[derive(Default)]
struct PhaseSlots {
    outbox: Option<PhaseHandle>,
    ingestion: Option<PhaseHandle>,
    provider_migration: Option<PhaseHandle>,
    group_archive: Option<PhaseHandle>,
    wiki_backfill: Option<PhaseHandle>,
    wiki_drive_relay: Option<PhaseHandle>,
    wiki_drive_events: Option<PhaseHandle>,
    wiki_sources: Option<PhaseHandle>,
    wiki_delivery_renewal: Option<PhaseHandle>,
}

/// Drives one maintenance phase under a bounded time budget with panic and
/// error isolation.
///
/// - A phase that returns an error or panics is logged and metered; the
///   remaining phases of the tick still run.
/// - A phase that exceeds its budget is kept in-flight (at most one instance)
///   and continues in the background; later ticks give it more budget until it
///   finishes, so slow domains pace themselves without starving the others and
///   without piling up duplicate work.
async fn drive_phase(
    phase_index: usize,
    budget: std::time::Duration,
    slot: &mut Option<PhaseHandle>,
    spawn_phase: impl FnOnce() -> PhaseHandle,
) -> Option<PhaseResult> {
    let mut handle = match slot.take() {
        Some(handle) => handle,
        None => spawn_phase(),
    };
    match tokio::time::timeout(budget, &mut handle).await {
        Ok(Ok(Ok(result))) => Some(result),
        Ok(Ok(Err(error))) => {
            tracing::error!(
                target: "sdkwork.knowledgebase.worker",
                phase = WORKER_PHASE_NAMES[phase_index],
                error = %error,
                "knowledgebase worker phase failed; remaining phases continue"
            );
            sdkwork_knowledgebase_observability::record_worker_phase_failure(phase_index);
            None
        }
        Ok(Err(join_error)) => {
            tracing::error!(
                target: "sdkwork.knowledgebase.worker",
                phase = WORKER_PHASE_NAMES[phase_index],
                error = %join_error,
                "knowledgebase worker phase panicked; remaining phases continue"
            );
            sdkwork_knowledgebase_observability::record_worker_phase_failure(phase_index);
            None
        }
        Err(_elapsed) => {
            *slot = Some(handle);
            tracing::warn!(
                target: "sdkwork.knowledgebase.worker",
                phase = WORKER_PHASE_NAMES[phase_index],
                budget_seconds = budget.as_secs(),
                "knowledgebase worker phase exceeded its time budget; it continues in the background and is reaped by later ticks"
            );
            sdkwork_knowledgebase_observability::record_worker_phase_failure(phase_index);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Individual phases (shared by the sequential tick and the resilient loop).
// ---------------------------------------------------------------------------

async fn run_outbox_phase(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
) -> Result<OutboxPublishBatchResult, MaintenanceTickError> {
    runtime
        .publish_pending_outbox_events(config.outbox_limit)
        .await
        .map_err(MaintenanceTickError::Outbox)
}

async fn run_ingestion_phase(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
) -> Result<usize, MaintenanceTickError> {
    runtime
        .process_queued_ingestion_jobs(
            &config.worker_id,
            config.ingestion_job_lease,
            config.ingestion_job_limit,
            config.ingestion_max_attempts,
        )
        .await
        .map_err(MaintenanceTickError::Ingestion)
}

async fn run_provider_migration_phase(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
) -> Result<ProviderMigrationBatchResult, MaintenanceTickError> {
    runtime
        .process_provider_migrations(
            &config.worker_id,
            config.provider_migration_lease,
            config.provider_migration_limit,
        )
        .await
        .map_err(MaintenanceTickError::ProviderMigration)
}

async fn run_group_archive_phase(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
) -> usize {
    runtime
        .process_resumable_group_space_archives(config.group_archive_limit)
        .await
}

async fn run_wiki_backfill_phase(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
    after_space_id: Option<u64>,
    excluded_space_ids: Vec<u64>,
) -> Result<WikiBackfillPhaseResult, MaintenanceTickError> {
    let Some(config) = config.wiki_backfill else {
        return Ok(WikiBackfillPhaseResult {
            initialized: 0,
            failed: 0,
            next_after_space_id: None,
            failed_space_ids: Vec::new(),
            succeeded_space_ids: Vec::new(),
        });
    };
    if config.tenant_id == 0
        || config.actor_id == 0
        || config.page_size == 0
        || config.page_size > MAX_WIKI_BACKFILL_PAGE_SIZE
    {
        return Err(MaintenanceTickError::WikiBackfill(
            "maintenance configuration is invalid".to_string(),
        ));
    }

    let result = runtime
        .run_wiki_publication_backfill_page(RunWikiPublicationBackfillRequest {
            scope: WikiPersistenceScope {
                tenant_id: config.tenant_id,
                organization_id: config.organization_id,
            },
            after_space_id,
            excluded_space_ids,
            page_size: config.page_size,
            actor_id: config.actor_id,
            dry_run: false,
        })
        .await
        .map_err(MaintenanceTickError::WikiBackfill)?;
    let initialized = result
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == WikiPublicationBackfillDisposition::Initialized)
        .count();
    let failed = result
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == WikiPublicationBackfillDisposition::Failed)
        .count();
    let failed_space_ids = result
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == WikiPublicationBackfillDisposition::Failed)
        .map(|outcome| outcome.space_id)
        .collect();
    let succeeded_space_ids = result
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == WikiPublicationBackfillDisposition::Initialized)
        .map(|outcome| outcome.space_id)
        .collect();
    Ok(WikiBackfillPhaseResult {
        initialized,
        failed,
        next_after_space_id: result.next_after_space_id,
        failed_space_ids,
        succeeded_space_ids,
    })
}

/// Returns the space ids currently in backoff cooldown. Failed candidates rejoin the
/// candidate scan only after `30s * 2^failures` capped at 1h, so a permanently failing
/// space cannot be hammered every tick nor stall the rest of the domain.
fn wiki_backfill_cooldown_backoff(failures: u32) -> std::time::Duration {
    let exponent = failures.saturating_sub(1).min(7);
    let secs = WIKI_BACKFILL_COOLDOWN_BASE_SECS.saturating_mul(1_u64 << exponent);
    std::time::Duration::from_secs(secs.min(WIKI_BACKFILL_COOLDOWN_MAX_SECS))
}

fn active_backfill_cooldown_space_ids(
    cooldowns: &std::collections::HashMap<u64, WikiBackfillCooldown>,
) -> Vec<u64> {
    let now = std::time::Instant::now();
    cooldowns
        .iter()
        .filter(|(_, cooldown)| cooldown.retry_after > now)
        .map(|(space_id, _)| *space_id)
        .collect()
}

fn update_backfill_cooldowns(
    cooldowns: &mut std::collections::HashMap<u64, WikiBackfillCooldown>,
    backfill: &WikiBackfillPhaseResult,
) {
    // A space that finally initialized leaves the cooldown table; its retry counter is
    // reset naturally by removal.
    for space_id in &backfill.succeeded_space_ids {
        cooldowns.remove(space_id);
    }
    // Failed spaces accumulate their attempt count across cooldown periods so the backoff
    // really doubles; an expired cooldown only re-admits the candidate, it never resets
    // the exponential schedule.
    let now = std::time::Instant::now();
    for space_id in &backfill.failed_space_ids {
        let next_failures = cooldowns
            .get(space_id)
            .map(|cooldown| cooldown.failures.saturating_add(1))
            .unwrap_or(1);
        let retry_after = now + wiki_backfill_cooldown_backoff(next_failures);
        cooldowns.insert(
            *space_id,
            WikiBackfillCooldown {
                failures: next_failures,
                retry_after,
            },
        );
    }
}

async fn run_wiki_drive_relay_phase(
    runtime: &KnowledgebaseRuntime,
) -> Result<DomainOutboxDispatchResult, MaintenanceTickError> {
    runtime
        .relay_embedded_wiki_drive_outbox_events()
        .await
        .map_err(MaintenanceTickError::WikiDriveEvents)
}

async fn run_wiki_drive_phase(
    runtime: &KnowledgebaseRuntime,
    worker_id: &str,
    config: WikiDriveEventMaintenanceConfig,
    after_checkpoint_id: Option<u64>,
) -> Result<KnowledgeWikiDriveCheckpointPageResult, MaintenanceTickError> {
    if config.tenant_id == 0
        || config.checkpoint_page_size == 0
        || config.checkpoint_page_size > 200
        || config.event_batch_size == 0
        || config.event_batch_size > 100
        || config.actor_id == 0
        || config.lease_seconds == 0
        || config.lease_seconds > 3_600
        || config.retry_delay_seconds == 0
        || config.retry_delay_seconds > 86_400
        || config.max_attempts == 0
        || config.max_attempts > 100
        || config.source_batch_size == 0
        || config.source_batch_size > 100
        || config.source_lease_seconds == 0
        || config.source_lease_seconds > 3_600
        || config.source_retry_delay_seconds == 0
        || config.source_retry_delay_seconds > 86_400
        || config.source_max_attempts == 0
        || config.source_max_attempts > 100
        || config.delivery_renewal_page_size == 0
        || config.delivery_renewal_page_size > 200
    {
        return Err(MaintenanceTickError::WikiDriveEvents(
            "Wiki Drive event maintenance configuration is invalid".to_string(),
        ));
    }
    runtime
        .process_wiki_drive_event_checkpoint_page(ProcessKnowledgeWikiDriveCheckpointPageRequest {
            scope: WikiPersistenceScope {
                tenant_id: config.tenant_id,
                organization_id: config.organization_id,
            },
            after_checkpoint_id,
            worker_id: worker_id.to_string(),
            actor_id: config.actor_id,
            lease_seconds: config.lease_seconds,
            checkpoint_limit: config.checkpoint_page_size,
            event_limit_per_checkpoint: config.event_batch_size,
            retry_delay_seconds: config.retry_delay_seconds,
            max_attempts: config.max_attempts,
        })
        .await
        .map_err(MaintenanceTickError::WikiDriveEvents)
}

async fn run_wiki_source_phase(
    runtime: &KnowledgebaseRuntime,
    worker_id: &str,
    config: WikiDriveEventMaintenanceConfig,
    after_checkpoint_id: Option<u64>,
) -> Result<KnowledgeWikiSourceCheckpointPageResult, MaintenanceTickError> {
    runtime
        .process_wiki_source_checkpoint_page(ProcessKnowledgeWikiSourceCheckpointPageRequest {
            scope: WikiPersistenceScope {
                tenant_id: config.tenant_id,
                organization_id: config.organization_id,
            },
            after_checkpoint_id,
            worker_id: worker_id.to_string(),
            actor_id: config.actor_id,
            lease_seconds: config.source_lease_seconds,
            checkpoint_limit: config.checkpoint_page_size,
            source_limit_per_checkpoint: config.source_batch_size,
            retry_delay_seconds: config.source_retry_delay_seconds,
            max_attempts: config.source_max_attempts,
        })
        .await
        .map_err(MaintenanceTickError::WikiSourceProcessing)
}

async fn run_wiki_delivery_phase(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
    renew: bool,
    after_checkpoint_id: Option<u64>,
) -> Result<
    sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::WikiDriveEventDeliveryRenewalPageResult,
    MaintenanceTickError,
>{
    if !renew {
        return Ok(
            sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::WikiDriveEventDeliveryRenewalPageResult {
                checkpoints_scanned: 0,
                cloud_deliveries_renewed: 0,
                embedded_relays_verified: 0,
                failures: Vec::new(),
                next_after_checkpoint_id: after_checkpoint_id,
            },
        );
    }
    runtime
        .renew_wiki_drive_event_delivery_page(
            sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::RenewWikiDriveEventDeliveryPageRequest {
                scope: WikiPersistenceScope {
                    tenant_id: config.wiki_drive_events.tenant_id,
                    organization_id: config.wiki_drive_events.organization_id,
                },
                after_checkpoint_id,
                limit: config.wiki_drive_events.delivery_renewal_page_size,
            },
        )
        .await
        .map_err(MaintenanceTickError::WikiDriveEvents)
}

fn log_delivery_renewal_failures(
    result: &sdkwork_intelligence_knowledgebase_service::wiki_event_delivery::WikiDriveEventDeliveryRenewalPageResult,
) {
    for failure in &result.failures {
        tracing::warn!(
            target: "sdkwork.knowledgebase.wiki",
            event = "knowledgebase.wiki.drive_event_delivery_renewal_failed",
            checkpoint_id = failure.checkpoint_id,
            source_scope_uuid = %failure.source_scope_uuid,
            error_code = %failure.error_code,
            retry_scheduled = true,
            retry_policy = "next_renewal_scan",
            "Wiki Drive event delivery renewal failed and remains eligible for the next bounded scan"
        );
    }
}

/// Runs every maintenance phase sequentially and fails fast on the first error.
///
/// This is the deterministic composition used by integration tests. The
/// production polling loop uses [`run_polling_loop`], which drives the same
/// phases with per-phase time budgets, panic isolation, and background
/// continuation.
pub async fn run_maintenance_tick(
    runtime: &KnowledgebaseRuntime,
    config: &MaintenanceConfig,
    state: MaintenanceTickState,
) -> Result<MaintenanceTickResult, MaintenanceTickError> {
    let outbox = run_outbox_phase(runtime, config).await?;
    sdkwork_knowledgebase_observability::record_outbox_maintenance_batch(
        outbox.requeued,
        outbox.published,
        outbox.failed,
        outbox.dead_lettered,
    );
    let ingestion_jobs_processed = run_ingestion_phase(runtime, config).await?;
    let provider_migrations = run_provider_migration_phase(runtime, config).await?;
    let group_archives_processed = run_group_archive_phase(runtime, config).await;
    let wiki_backfill = run_wiki_backfill_phase(runtime, config, None, Vec::new()).await?;
    let wiki_drive_relay_result = run_wiki_drive_relay_phase(runtime).await?;
    let wiki_drive_result = run_wiki_drive_phase(
        runtime,
        &config.worker_id,
        config.wiki_drive_events,
        state.wiki_checkpoint_cursor,
    )
    .await?;
    let wiki_source_result = run_wiki_source_phase(
        runtime,
        &config.worker_id,
        config.wiki_drive_events,
        state.wiki_source_cursor,
    )
    .await?;
    let wiki_delivery_result = run_wiki_delivery_phase(
        runtime,
        config,
        state.renew_wiki_event_deliveries,
        state.wiki_delivery_cursor,
    )
    .await?;
    log_delivery_renewal_failures(&wiki_delivery_result);
    Ok(MaintenanceTickResult {
        outbox_requeued: outbox.requeued,
        outbox_published: outbox.published,
        outbox_failed: outbox.failed,
        outbox_dead_lettered: outbox.dead_lettered,
        ingestion_jobs_processed,
        provider_migration_phases_processed: provider_migrations.processed,
        provider_migrations_completed: provider_migrations.completed,
        provider_migrations_rolled_back: provider_migrations.rolled_back,
        provider_migrations_failed: provider_migrations.failed,
        group_archives_processed,
        wiki_publications_initialized: wiki_backfill.initialized,
        wiki_publications_failed: wiki_backfill.failed,
        wiki_backfill_next_after_space_id: wiki_backfill.next_after_space_id,
        wiki_backfill_failed_space_ids: wiki_backfill.failed_space_ids,
        wiki_drive_outbox_events_processed: wiki_drive_relay_result.processed,
        wiki_drive_outbox_events_delivered: wiki_drive_relay_result.delivered,
        wiki_drive_outbox_events_failed: wiki_drive_relay_result.failed,
        wiki_drive_checkpoints_processed: wiki_drive_result.checkpoints_processed,
        wiki_drive_events_applied: wiki_drive_result.events.applied,
        wiki_drive_events_retried: wiki_drive_result.events.retried,
        wiki_drive_events_dead_lettered: wiki_drive_result.events.dead_lettered,
        wiki_drive_public_changes: wiki_drive_result.events.public_changes,
        wiki_sources_claimed: wiki_source_result.sources_claimed,
        wiki_sources_ready: wiki_source_result.sources_ready,
        wiki_sources_auto_published: wiki_source_result.sources_auto_published,
        wiki_sources_retried: wiki_source_result.sources_retried,
        wiki_sources_quarantined: wiki_source_result.sources_quarantined,
        wiki_auto_publications_deferred: wiki_source_result.auto_publications_deferred,
        wiki_drive_next_after_checkpoint_id: wiki_drive_result.next_after_checkpoint_id,
        wiki_drive_blocked_checkpoint_id: wiki_drive_result.blocked_checkpoint_id,
        wiki_drive_source_next_after_checkpoint_id: wiki_source_result.next_after_checkpoint_id,
        wiki_drive_event_deliveries_renewed: wiki_delivery_result.cloud_deliveries_renewed,
        wiki_drive_event_delivery_relays_verified: wiki_delivery_result.embedded_relays_verified,
        wiki_drive_event_delivery_failures: wiki_delivery_result.failures.len(),
        wiki_drive_next_after_event_delivery_checkpoint_id: wiki_delivery_result
            .next_after_checkpoint_id,
    })
}

/// Production maintenance loop.
///
/// Every phase runs under its own time budget with panic and error isolation
/// (see [`drive_phase`]). A slow webhook, a panicking phase, or a stuck
/// checkpoint can delay only its own domain; ingestion, migration, archive,
/// and wiki maintenance always get their budget each tick. Missed ticks are
/// skipped (never burst-caught-up) so the loop cannot enter a tight poll storm.
pub async fn run_polling_loop(runtime: KnowledgebaseRuntime, config: MaintenancePollingConfig) {
    let runtime = std::sync::Arc::new(runtime);
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(
        config.interval_ms.max(250),
    ));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let phase_budget = config.maintenance.phase_timeout;
    let mut wiki_checkpoint_cursor = None;
    let mut wiki_source_cursor = None;
    let mut wiki_backfill_cursor = None;
    let mut wiki_backfill_cooldowns = std::collections::HashMap::new();
    let mut wiki_delivery_cursor = None;
    let renewal_interval = std::time::Duration::from_secs(
        std::env::var("SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_EVENT_DELIVERY_RENEWAL_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| (60..=86_400).contains(value))
            .unwrap_or(3_600),
    );
    let mut last_delivery_renewal = std::time::Instant::now()
        .checked_sub(renewal_interval)
        .unwrap_or_else(std::time::Instant::now);
    let mut phases = PhaseSlots::default();
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let renew_wiki_event_deliveries = last_delivery_renewal.elapsed() >= renewal_interval;
                let maintenance = config.maintenance.clone();
                let mut result = MaintenanceTickResult::default();

                // 0. outbox
                if let Some(PhaseResult::Outbox(outbox)) = drive_phase(0, phase_budget, &mut phases.outbox, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    || tokio::spawn(async move {
                        run_outbox_phase(&runtime, &maintenance).await.map(PhaseResult::Outbox)
                    })
                }).await {
                    sdkwork_knowledgebase_observability::record_outbox_maintenance_batch(
                        outbox.requeued,
                        outbox.published,
                        outbox.failed,
                        outbox.dead_lettered,
                    );
                    result.outbox_requeued = outbox.requeued;
                    result.outbox_published = outbox.published;
                    result.outbox_failed = outbox.failed;
                    result.outbox_dead_lettered = outbox.dead_lettered;
                }

                // 1. ingestion
                if let Some(PhaseResult::Ingestion(processed)) = drive_phase(1, phase_budget, &mut phases.ingestion, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    || tokio::spawn(async move {
                        run_ingestion_phase(&runtime, &maintenance).await.map(PhaseResult::Ingestion)
                    })
                }).await {
                    result.ingestion_jobs_processed = processed;
                }

                // 2. provider migration
                if let Some(PhaseResult::ProviderMigration(migrations)) = drive_phase(2, phase_budget, &mut phases.provider_migration, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    || tokio::spawn(async move {
                        run_provider_migration_phase(&runtime, &maintenance).await.map(PhaseResult::ProviderMigration)
                    })
                }).await {
                    result.provider_migration_phases_processed = migrations.processed;
                    result.provider_migrations_completed = migrations.completed;
                    result.provider_migrations_rolled_back = migrations.rolled_back;
                    result.provider_migrations_failed = migrations.failed;
                }

                // 3. group archive
                if let Some(PhaseResult::GroupArchives(processed)) = drive_phase(3, phase_budget, &mut phases.group_archive, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    || tokio::spawn(async move {
                        Ok(PhaseResult::GroupArchives(run_group_archive_phase(&runtime, &maintenance).await))
                    })
                }).await {
                    result.group_archives_processed = processed;
                }

                // 4. wiki backfill
                if let Some(PhaseResult::WikiBackfill(backfill)) = drive_phase(4, phase_budget, &mut phases.wiki_backfill, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    let after_space_id = wiki_backfill_cursor;
                    let excluded_space_ids = active_backfill_cooldown_space_ids(&wiki_backfill_cooldowns);
                    move || tokio::spawn(async move {
                        run_wiki_backfill_phase(&runtime, &maintenance, after_space_id, excluded_space_ids)
                            .await.map(PhaseResult::WikiBackfill)
                    })
                }).await {
                    update_backfill_cooldowns(&mut wiki_backfill_cooldowns, &backfill);
                    result.wiki_publications_initialized = backfill.initialized;
                    result.wiki_publications_failed = backfill.failed;
                    result.wiki_backfill_next_after_space_id = backfill.next_after_space_id;
                    result.wiki_backfill_failed_space_ids = backfill.failed_space_ids;
                }

                // 5. wiki drive relay
                if let Some(PhaseResult::WikiDriveRelay(relay)) = drive_phase(5, phase_budget, &mut phases.wiki_drive_relay, {
                    let runtime = runtime.clone();
                    || tokio::spawn(async move {
                        run_wiki_drive_relay_phase(&runtime).await.map(PhaseResult::WikiDriveRelay)
                    })
                }).await {
                    result.wiki_drive_outbox_events_processed = relay.processed;
                    result.wiki_drive_outbox_events_delivered = relay.delivered;
                    result.wiki_drive_outbox_events_failed = relay.failed;
                }

                // 6. wiki drive events
                if let Some(PhaseResult::WikiDrive(drive)) = drive_phase(6, phase_budget, &mut phases.wiki_drive_events, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    let after_checkpoint_id = wiki_checkpoint_cursor;
                    move || tokio::spawn(async move {
                        let worker_id = maintenance.worker_id.clone();
                        let wiki_drive_events = maintenance.wiki_drive_events;
                        run_wiki_drive_phase(&runtime, &worker_id, wiki_drive_events, after_checkpoint_id)
                            .await.map(PhaseResult::WikiDrive)
                    })
                }).await {
                    result.wiki_drive_checkpoints_processed = drive.checkpoints_processed;
                    result.wiki_drive_events_applied = drive.events.applied;
                    result.wiki_drive_events_retried = drive.events.retried;
                    result.wiki_drive_events_dead_lettered = drive.events.dead_lettered;
                    result.wiki_drive_public_changes = drive.events.public_changes;
                    result.wiki_drive_next_after_checkpoint_id = drive.next_after_checkpoint_id;
                    result.wiki_drive_blocked_checkpoint_id = drive.blocked_checkpoint_id;
                }

                // 7. wiki sources
                if let Some(PhaseResult::WikiSources(sources)) = drive_phase(7, phase_budget, &mut phases.wiki_sources, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    let after_checkpoint_id = wiki_source_cursor;
                    move || tokio::spawn(async move {
                        let worker_id = maintenance.worker_id.clone();
                        let wiki_drive_events = maintenance.wiki_drive_events;
                        run_wiki_source_phase(&runtime, &worker_id, wiki_drive_events, after_checkpoint_id)
                            .await.map(PhaseResult::WikiSources)
                    })
                }).await {
                    result.wiki_sources_claimed = sources.sources_claimed;
                    result.wiki_sources_ready = sources.sources_ready;
                    result.wiki_sources_auto_published = sources.sources_auto_published;
                    result.wiki_sources_retried = sources.sources_retried;
                    result.wiki_sources_quarantined = sources.sources_quarantined;
                    result.wiki_auto_publications_deferred = sources.auto_publications_deferred;
                    result.wiki_drive_source_next_after_checkpoint_id = sources.next_after_checkpoint_id;
                }

                // 8. wiki delivery renewal
                if let Some(PhaseResult::WikiDelivery(delivery)) = drive_phase(8, phase_budget, &mut phases.wiki_delivery_renewal, {
                    let runtime = runtime.clone();
                    let maintenance = maintenance.clone();
                    || tokio::spawn(async move {
                        run_wiki_delivery_phase(&runtime, &maintenance, renew_wiki_event_deliveries, wiki_delivery_cursor)
                            .await.map(PhaseResult::WikiDelivery)
                    })
                }).await {
                    log_delivery_renewal_failures(&delivery);
                    result.wiki_drive_event_deliveries_renewed = delivery.cloud_deliveries_renewed;
                    result.wiki_drive_event_delivery_relays_verified = delivery.embedded_relays_verified;
                    result.wiki_drive_event_delivery_failures = delivery.failures.len();
                    result.wiki_drive_next_after_event_delivery_checkpoint_id = delivery.next_after_checkpoint_id;
                    // Reset the interval clock only from an OBSERVED completed
                    // sweep (next = None): a phase timeout or error yields a
                    // struct-default None without running the sweep, and
                    // resetting there would stall a mid-scan renewal for a
                    // whole interval — the starvation this latch exists to
                    // prevent.
                    if renew_wiki_event_deliveries && delivery.next_after_checkpoint_id.is_none() {
                        last_delivery_renewal = std::time::Instant::now();
                    }
                }

                if let Some(blocked) = result.wiki_drive_blocked_checkpoint_id {
                    // A checkpoint with retried head events is not caught up; the exclusive
                    // cursor must restart at it so its RETRY events are repicked at deadline.
                    // Later checkpoints on the page were still processed (progress), but the
                    // cursor parks at the blocked checkpoint to preserve retry ordering.
                    wiki_checkpoint_cursor = Some(blocked.saturating_sub(1));
                } else if let Some(next) = result.wiki_drive_next_after_checkpoint_id {
                    wiki_checkpoint_cursor = Some(next);
                } else {
                    wiki_checkpoint_cursor = None;
                }
                if let Some(next) = result.wiki_drive_source_next_after_checkpoint_id {
                    wiki_source_cursor = Some(next);
                } else {
                    wiki_source_cursor = None;
                }
                if let Some(next) = result.wiki_backfill_next_after_space_id {
                    wiki_backfill_cursor = Some(next);
                } else {
                    // Full scan completed (or page is empty): next tick restarts from the
                    // beginning so newly provisioned spaces are discovered.
                    wiki_backfill_cursor = None;
                }
                if let Some(next) = result.wiki_drive_next_after_event_delivery_checkpoint_id {
                    wiki_delivery_cursor = Some(next);
                } else {
                    // Full sweep completed (or no sweep ran this tick): next
                    // renewal attempt restarts from the beginning, so
                    // checkpoints scanned in earlier passes keep getting their
                    // Drive event-delivery subscriptions renewed. (The interval
                    // clock is reset in the WikiDelivery result block above,
                    // only for an observed completed sweep.)
                    wiki_delivery_cursor = None;
                }

                if maintenance_tick_has_activity(&result) {
                    tracing::info!(
                        outbox_requeued = result.outbox_requeued,
                        outbox_published = result.outbox_published,
                        outbox_failed = result.outbox_failed,
                        outbox_dead_lettered = result.outbox_dead_lettered,
                        ingestion_jobs_processed = result.ingestion_jobs_processed,
                        provider_migration_phases_processed = result.provider_migration_phases_processed,
                        provider_migrations_completed = result.provider_migrations_completed,
                        provider_migrations_rolled_back = result.provider_migrations_rolled_back,
                        provider_migrations_failed = result.provider_migrations_failed,
                        group_archives_processed = result.group_archives_processed,
                        wiki_publications_initialized = result.wiki_publications_initialized,
                        wiki_publications_failed = result.wiki_publications_failed,
                        wiki_drive_outbox_events_processed = result.wiki_drive_outbox_events_processed,
                        wiki_drive_outbox_events_delivered = result.wiki_drive_outbox_events_delivered,
                        wiki_drive_outbox_events_failed = result.wiki_drive_outbox_events_failed,
                        wiki_drive_checkpoints_processed = result.wiki_drive_checkpoints_processed,
                        wiki_drive_events_applied = result.wiki_drive_events_applied,
                        wiki_drive_events_retried = result.wiki_drive_events_retried,
                        wiki_drive_events_dead_lettered = result.wiki_drive_events_dead_lettered,
                        wiki_drive_public_changes = result.wiki_drive_public_changes,
                        wiki_sources_claimed = result.wiki_sources_claimed,
                        wiki_sources_ready = result.wiki_sources_ready,
                        wiki_sources_auto_published = result.wiki_sources_auto_published,
                        wiki_sources_retried = result.wiki_sources_retried,
                        wiki_sources_quarantined = result.wiki_sources_quarantined,
                        wiki_auto_publications_deferred = result.wiki_auto_publications_deferred,
                        wiki_drive_event_deliveries_renewed = result.wiki_drive_event_deliveries_renewed,
                        wiki_drive_event_delivery_relays_verified = result.wiki_drive_event_delivery_relays_verified,
                        wiki_drive_event_delivery_failures = result.wiki_drive_event_delivery_failures,
                        "knowledgebase worker maintenance tick"
                    );
                }
            }
            _ = shutdown_signal() => {
                tracing::info!("knowledgebase worker shutdown signal received; exiting maintenance loop");
                break;
            }
        }
    }
}

fn maintenance_tick_has_activity(result: &MaintenanceTickResult) -> bool {
    result.outbox_requeued > 0
        || result.outbox_published > 0
        || result.outbox_failed > 0
        || result.outbox_dead_lettered > 0
        || result.ingestion_jobs_processed > 0
        || result.provider_migration_phases_processed > 0
        || result.group_archives_processed > 0
        || result.wiki_publications_initialized > 0
        || result.wiki_publications_failed > 0
        || result.wiki_drive_outbox_events_processed > 0
        || result.wiki_drive_outbox_events_delivered > 0
        || result.wiki_drive_outbox_events_failed > 0
        || result.wiki_drive_checkpoints_processed > 0
        || result.wiki_drive_events_applied > 0
        || result.wiki_drive_events_retried > 0
        || result.wiki_drive_events_dead_lettered > 0
        || result.wiki_drive_public_changes > 0
        || result.wiki_sources_claimed > 0
        || result.wiki_sources_ready > 0
        || result.wiki_sources_auto_published > 0
        || result.wiki_sources_retried > 0
        || result.wiki_sources_quarantined > 0
        || result.wiki_auto_publications_deferred > 0
        || result.wiki_drive_event_deliveries_renewed > 0
        || result.wiki_drive_event_delivery_relays_verified > 0
        || result.wiki_drive_event_delivery_failures > 0
}

#[cfg(test)]
mod tests {
    use super::{
        active_backfill_cooldown_space_ids, update_backfill_cooldowns,
        wiki_backfill_cooldown_backoff, MaintenanceTickResult, WikiBackfillCooldown,
        WikiBackfillPhaseResult,
    };
    use std::collections::HashMap;

    #[test]
    fn backfill_cooldown_backoff_doubles_up_to_cap() {
        assert_eq!(
            wiki_backfill_cooldown_backoff(1),
            std::time::Duration::from_secs(30)
        );
        assert_eq!(
            wiki_backfill_cooldown_backoff(2),
            std::time::Duration::from_secs(60)
        );
        assert_eq!(
            wiki_backfill_cooldown_backoff(8),
            std::time::Duration::from_secs(3_600)
        );
        assert_eq!(
            wiki_backfill_cooldown_backoff(100),
            std::time::Duration::from_secs(3_600)
        );
    }

    #[test]
    fn backfill_cooldown_excludes_active_and_accumulates_failures() {
        let mut cooldowns = HashMap::new();
        cooldowns.insert(
            501,
            WikiBackfillCooldown {
                failures: 1,
                retry_after: std::time::Instant::now() + std::time::Duration::from_secs(60),
            },
        );
        cooldowns.insert(
            502,
            WikiBackfillCooldown {
                failures: 3,
                retry_after: std::time::Instant::now() - std::time::Duration::from_secs(1),
            },
        );
        let active = active_backfill_cooldown_space_ids(&cooldowns);
        assert_eq!(active, vec![501]);
        assert_eq!(cooldowns.len(), 2);

        // 502 re-fails after its cooldown expired: the counter keeps climbing (4th failure)
        // instead of resetting, so the exponential schedule really doubles. 503 succeeds and
        // is removed from the table entirely.
        update_backfill_cooldowns(
            &mut cooldowns,
            &WikiBackfillPhaseResult {
                initialized: 1,
                failed: 1,
                next_after_space_id: None,
                failed_space_ids: vec![502],
                succeeded_space_ids: vec![503],
            },
        );
        assert_eq!(cooldowns.len(), 2);
        assert_eq!(cooldowns.get(&502).map(|state| state.failures), Some(4));
        assert_eq!(cooldowns.get(&503), None);
    }

    #[test]
    fn maintenance_tick_result_tracks_worker_outputs() {
        let result = MaintenanceTickResult {
            outbox_requeued: 1,
            outbox_published: 2,
            outbox_failed: 1,
            outbox_dead_lettered: 0,
            ingestion_jobs_processed: 3,
            provider_migration_phases_processed: 5,
            provider_migrations_completed: 1,
            provider_migrations_rolled_back: 1,
            provider_migrations_failed: 0,
            group_archives_processed: 4,
            wiki_publications_initialized: 6,
            wiki_publications_failed: 1,
            wiki_backfill_next_after_space_id: Some(11),
            wiki_backfill_failed_space_ids: vec![12],
            wiki_drive_outbox_events_processed: 4,
            wiki_drive_outbox_events_delivered: 3,
            wiki_drive_outbox_events_failed: 1,
            wiki_drive_checkpoints_processed: 2,
            wiki_drive_events_applied: 3,
            wiki_drive_events_retried: 1,
            wiki_drive_events_dead_lettered: 0,
            wiki_drive_public_changes: 2,
            wiki_sources_claimed: 5,
            wiki_sources_ready: 4,
            wiki_sources_auto_published: 3,
            wiki_sources_retried: 1,
            wiki_sources_quarantined: 1,
            wiki_auto_publications_deferred: 1,
            wiki_drive_next_after_checkpoint_id: Some(9),
            wiki_drive_blocked_checkpoint_id: Some(8),
            wiki_drive_source_next_after_checkpoint_id: Some(7),
            wiki_drive_event_deliveries_renewed: 1,
            wiki_drive_event_delivery_relays_verified: 0,
            wiki_drive_event_delivery_failures: 0,
            wiki_drive_next_after_event_delivery_checkpoint_id: Some(10),
        };
        assert_eq!(result.outbox_requeued, 1);
        assert_eq!(result.outbox_published, 2);
        assert_eq!(result.outbox_failed, 1);
        assert_eq!(result.outbox_dead_lettered, 0);
        assert_eq!(result.ingestion_jobs_processed, 3);
        assert_eq!(result.provider_migration_phases_processed, 5);
        assert_eq!(result.provider_migrations_completed, 1);
        assert_eq!(result.provider_migrations_rolled_back, 1);
        assert_eq!(result.provider_migrations_failed, 0);
        assert_eq!(result.group_archives_processed, 4);
        assert_eq!(result.wiki_publications_initialized, 6);
        assert_eq!(result.wiki_publications_failed, 1);
        assert_eq!(result.wiki_drive_outbox_events_processed, 4);
        assert_eq!(result.wiki_drive_outbox_events_delivered, 3);
        assert_eq!(result.wiki_drive_outbox_events_failed, 1);
        assert_eq!(result.wiki_drive_checkpoints_processed, 2);
        assert_eq!(result.wiki_drive_events_applied, 3);
        assert_eq!(result.wiki_drive_events_retried, 1);
        assert_eq!(result.wiki_drive_events_dead_lettered, 0);
        assert_eq!(result.wiki_drive_public_changes, 2);
        assert_eq!(result.wiki_sources_claimed, 5);
        assert_eq!(result.wiki_sources_ready, 4);
        assert_eq!(result.wiki_sources_auto_published, 3);
        assert_eq!(result.wiki_sources_retried, 1);
        assert_eq!(result.wiki_sources_quarantined, 1);
        assert_eq!(result.wiki_auto_publications_deferred, 1);
        assert_eq!(result.wiki_drive_next_after_checkpoint_id, Some(9));
        assert_eq!(result.wiki_drive_blocked_checkpoint_id, Some(8));
        assert_eq!(result.wiki_drive_source_next_after_checkpoint_id, Some(7));
        assert_eq!(result.wiki_backfill_next_after_space_id, Some(11));
        assert_eq!(result.wiki_backfill_failed_space_ids, vec![12]);
        assert_eq!(result.wiki_drive_event_deliveries_renewed, 1);
        assert_eq!(result.wiki_drive_event_delivery_relays_verified, 0);
        assert_eq!(result.wiki_drive_event_delivery_failures, 0);
        assert_eq!(
            result.wiki_drive_next_after_event_delivery_checkpoint_id,
            Some(10)
        );
    }
}
