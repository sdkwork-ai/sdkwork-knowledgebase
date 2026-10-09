//! HTTP metrics primitives for SDKWork Knowledgebase services.

pub mod audit;
pub use audit::{
    install_audit_persistence, record_backend_admin_operation,
    record_backend_admin_resource_operation, record_document_visibility_changed,
    record_provider_migration_transition, record_space_member_granted, record_space_member_revoked,
    AuditPersistenceError, AuditPersistenceEvent, BackendAdminResourceAudit,
};
pub mod billing_metrics;
pub use billing_metrics::{
    record_context_pack_completed, record_embedding_failed, record_ingest_job_failed,
    record_ingest_job_succeeded, record_retrieval_completed,
};
pub mod environment;
pub use environment::{
    deployment_tenant_id, is_development_environment, is_production_like_environment,
    knowledgebase_environment,
};
pub mod tenant_quota;
pub use tenant_quota::KnowledgebaseTenantQuotaLimits;
pub mod health;
pub mod request_correlation;
pub mod tracing_support;

#[cfg(feature = "otel")]
mod otel;

mod okf_metrics;
mod provider_metrics;

use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use okf_metrics::render_okf_prometheus_metrics;
use provider_metrics::render_provider_prometheus_metrics;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub use okf_metrics::{
    record_okf_bundle_exported, record_okf_bundle_imported, record_okf_bundle_lint_completed,
    record_okf_concept_publish, record_okf_concept_upsert,
};
pub use provider_metrics::{install_provider_metrics, KnowledgebaseProviderMetrics};

static REQUESTS_TOTAL: AtomicU64 = AtomicU64::new(0);
static REQUEST_ERRORS_TOTAL: AtomicU64 = AtomicU64::new(0);
static REQUEST_AUTH_FAILURES_TOTAL: AtomicU64 = AtomicU64::new(0);
static REQUEST_DURATION_MS_TOTAL: AtomicU64 = AtomicU64::new(0);
/// Fixed latency buckets (ms) so P95/P99 retrieval latency targets are measurable from
/// Prometheus histograms instead of a bare cumulative counter.
const DURATION_BUCKETS_MS: [u64; 11] = [5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000, 10_000];
static REQUEST_DURATION_BUCKETS: [AtomicU64; 11] = [const { AtomicU64::new(0) }; 11];
static HEALTH_STATUS: AtomicU64 = AtomicU64::new(1);
static OUTBOX_REQUEUED_TOTAL: AtomicU64 = AtomicU64::new(0);
static OUTBOX_PUBLISHED_TOTAL: AtomicU64 = AtomicU64::new(0);
static OUTBOX_FAILED_TOTAL: AtomicU64 = AtomicU64::new(0);
static OUTBOX_DEAD_LETTERED_TOTAL: AtomicU64 = AtomicU64::new(0);
static WORKER_MAINTENANCE_FAILURES_TOTAL: AtomicU64 = AtomicU64::new(0);
static WORKER_PHASE_FAILURES_TOTAL: [AtomicU64; 9] = [
    AtomicU64::new(0), // outbox
    AtomicU64::new(0), // ingestion
    AtomicU64::new(0), // provider_migration
    AtomicU64::new(0), // group_archive
    AtomicU64::new(0), // wiki_backfill
    AtomicU64::new(0), // wiki_drive_relay
    AtomicU64::new(0), // wiki_drive_events
    AtomicU64::new(0), // wiki_sources
    AtomicU64::new(0), // wiki_delivery_renewal
];

/// Canonical worker phase names, index-aligned with [`WORKER_PHASE_FAILURES_TOTAL`].
pub const WORKER_PHASE_NAMES: [&str; 9] = [
    "outbox",
    "ingestion",
    "provider_migration",
    "group_archive",
    "wiki_backfill",
    "wiki_drive_relay",
    "wiki_drive_events",
    "wiki_sources",
    "wiki_delivery_renewal",
];

/// Update the exported readiness gauge (`1` = dependencies ready, `0` = not ready).
pub fn set_readiness_status(ready: bool) {
    HEALTH_STATUS.store(u64::from(ready), Ordering::Relaxed);
}

pub fn record_outbox_maintenance_batch(
    requeued: usize,
    published: usize,
    failed: usize,
    dead_lettered: usize,
) {
    OUTBOX_REQUEUED_TOTAL.fetch_add(requeued as u64, Ordering::Relaxed);
    OUTBOX_PUBLISHED_TOTAL.fetch_add(published as u64, Ordering::Relaxed);
    OUTBOX_FAILED_TOTAL.fetch_add(failed as u64, Ordering::Relaxed);
    OUTBOX_DEAD_LETTERED_TOTAL.fetch_add(dead_lettered as u64, Ordering::Relaxed);
}

pub fn record_worker_maintenance_failure() {
    WORKER_MAINTENANCE_FAILURES_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// Records a worker phase failure (error, panic, or time-budget exhaustion).
///
/// `phase_index` indexes [`WORKER_PHASE_NAMES`]; out-of-range indices are ignored.
pub fn record_worker_phase_failure(phase_index: usize) {
    if let Some(counter) = WORKER_PHASE_FAILURES_TOTAL.get(phase_index) {
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

fn render_worker_phase_failure_metrics() -> String {
    WORKER_PHASE_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            format!(
                "knowledgebase_worker_phase_failures_total{{phase=\"{name}\"}} {}\n",
                WORKER_PHASE_FAILURES_TOTAL
                    .get(index)
                    .map(|counter| counter.load(Ordering::Relaxed))
                    .unwrap_or(0)
            )
        })
        .collect()
}

pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let started = Instant::now();
    REQUESTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    let response = next.run(request).await;
    if response.status().is_server_error() {
        REQUEST_ERRORS_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
    if matches!(
        response.status(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
    ) {
        REQUEST_AUTH_FAILURES_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    REQUEST_DURATION_MS_TOTAL.fetch_add(elapsed_ms, Ordering::Relaxed);
    for (index, bucket) in DURATION_BUCKETS_MS.iter().enumerate() {
        if elapsed_ms <= *bucket {
            REQUEST_DURATION_BUCKETS[index].fetch_add(1, Ordering::Relaxed);
        }
    }
    response
}

fn render_duration_histogram() -> String {
    let mut body = String::new();
    for (index, bucket) in DURATION_BUCKETS_MS.iter().enumerate() {
        body.push_str(&format!(
            "knowledge_api_request_duration_ms_bucket{{le=\"{bucket}\"}} {}
",
            REQUEST_DURATION_BUCKETS[index].load(Ordering::Relaxed)
        ));
    }
    let total = REQUESTS_TOTAL.load(Ordering::Relaxed);
    body.push_str(&format!(
        "knowledge_api_request_duration_ms_bucket{{le=\"+Inf\"}} {}
",
        total
    ));
    body.push_str(&format!(
        "knowledge_api_request_duration_ms_sum {}
",
        REQUEST_DURATION_MS_TOTAL.load(Ordering::Relaxed)
    ));
    body.push_str(&format!(
        "knowledge_api_request_duration_ms_count {}
",
        total
    ));
    body
}

pub async fn metrics_handler() -> impl IntoResponse {
    let body = format!(
        "# HELP knowledge_api_requests_total Total HTTP requests handled by knowledgebase API servers.\n\
         # TYPE knowledge_api_requests_total counter\n\
         knowledge_api_requests_total {}\n\
         # HELP knowledge_api_request_errors_total Total HTTP 5xx responses.\n\
         # TYPE knowledge_api_request_errors_total counter\n\
         knowledge_api_request_errors_total {}\n\
         # HELP knowledge_api_auth_failures_total Total HTTP 401/403 responses.\n\
         # TYPE knowledge_api_auth_failures_total counter\n\
         knowledge_api_auth_failures_total {}\n\
         # HELP knowledge_api_request_duration_ms_total Cumulative request duration in milliseconds.\n\
         # TYPE knowledge_api_request_duration_ms_total counter\n\
         knowledge_api_request_duration_ms_total {}\n\
         # HELP knowledge_api_request_duration_ms Request duration histogram (ms) for P95/P99 latency targets.\n\
         # TYPE knowledge_api_request_duration_ms histogram\n\
         {}{}\n\
         # HELP knowledgebase_health_status Service readiness gauge (1=ready, 0=not ready).\n\
         # TYPE knowledgebase_health_status gauge\n\
         knowledgebase_health_status {}\n\
         # HELP knowledgebase_outbox_events_total Knowledgebase outbox events handled by durable delivery status.\n\
         # TYPE knowledgebase_outbox_events_total counter\n\
         knowledgebase_outbox_events_total{{status=\"requeued\"}} {}\n\
         knowledgebase_outbox_events_total{{status=\"published\"}} {}\n\
         knowledgebase_outbox_events_total{{status=\"failed\"}} {}\n\
         knowledgebase_outbox_events_total{{status=\"dead_lettered\"}} {}\n\
         # HELP knowledgebase_worker_maintenance_failures_total Knowledgebase worker maintenance ticks that failed.\n\
         # TYPE knowledgebase_worker_maintenance_failures_total counter\n\
         knowledgebase_worker_maintenance_failures_total {}\n\
         # HELP knowledgebase_worker_phase_failures_total Knowledgebase worker maintenance phases that errored, panicked, or exceeded their time budget.\n\
         # TYPE knowledgebase_worker_phase_failures_total counter\n\
         {}{}{}{}{}",
        REQUESTS_TOTAL.load(Ordering::Relaxed),
        REQUEST_ERRORS_TOTAL.load(Ordering::Relaxed),
        REQUEST_AUTH_FAILURES_TOTAL.load(Ordering::Relaxed),
        REQUEST_DURATION_MS_TOTAL.load(Ordering::Relaxed),
        render_duration_histogram(),
        "",
        HEALTH_STATUS.load(Ordering::Relaxed),
        OUTBOX_REQUEUED_TOTAL.load(Ordering::Relaxed),
        OUTBOX_PUBLISHED_TOTAL.load(Ordering::Relaxed),
        OUTBOX_FAILED_TOTAL.load(Ordering::Relaxed),
        OUTBOX_DEAD_LETTERED_TOTAL.load(Ordering::Relaxed),
        WORKER_MAINTENANCE_FAILURES_TOTAL.load(Ordering::Relaxed),
        render_worker_phase_failure_metrics(),
        render_okf_prometheus_metrics(),
        audit::render_audit_prometheus_metrics(),
        billing_metrics::render_billing_prometheus_metrics(),
        render_provider_prometheus_metrics(),
    );

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4"),
        )],
        body,
    )
}

pub fn metrics_route() -> axum::Router {
    install_provider_metrics();
    axum::Router::new().route("/metrics", axum::routing::get(metrics_handler))
}

pub fn wrap_router_with_metrics(router: axum::Router) -> axum::Router {
    tracing_support::wrap_router_with_request_id(
        router
            .merge(metrics_route())
            .layer(axum::middleware::from_fn(metrics_middleware)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::routing::get;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn metrics_endpoint_exports_prometheus_counters() {
        let app =
            wrap_router_with_metrics(axum::Router::new().route("/healthz", get(|| async { "ok" })));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("knowledge_api_requests_total"));
        assert!(text.contains("knowledge_api_auth_failures_total"));
        assert!(text.contains("knowledgebase_health_status"));
        assert!(text.contains("knowledgebase_outbox_events_total{status=\"published\"}"));
        assert!(text.contains("knowledgebase_worker_maintenance_failures_total"));
        assert!(text.contains("knowledge_audit_document_visibility_changed_total"));
        assert!(text.contains("kb_okf_concept_publish_total"));
        assert!(text.contains("knowledge_retrievals_total"));
        assert!(text.contains("knowledge_context_packs_total"));
        assert!(text.contains("knowledge_provider_operations_total"));
        assert!(text.contains("knowledge_provider_operation_duration_seconds"));
    }
}
