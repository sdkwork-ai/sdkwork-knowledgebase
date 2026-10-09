use axum::{http::StatusCode, routing::get, Json, Router};
use sdkwork_routes_knowledgebase_app_api::ReadinessCheck;
use serde_json::{json, Value};

async fn livez() -> StatusCode {
    StatusCode::OK
}

async fn readyz_check(readiness: ReadinessCheck) -> Result<Json<Value>, StatusCode> {
    sdkwork_web_bootstrap::ReadinessCheck::check(readiness.as_ref())
        .await
        .map_err(|error| {
            tracing::warn!(?error, "knowledgebase worker readiness check failed");
            sdkwork_knowledgebase_observability::set_readiness_status(false);
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    sdkwork_knowledgebase_observability::set_readiness_status(true);
    Ok(Json(json!({ "status": "ok" })))
}

pub fn worker_health_router(readiness: ReadinessCheck) -> Router {
    let ready_probe = readiness.clone();
    let health_probe = readiness;
    Router::new()
        .route("/livez", get(livez))
        .route(
            "/readyz",
            get(move || {
                let readiness = ready_probe.clone();
                async move { readyz_check(readiness).await }
            }),
        )
        .route(
            "/healthz",
            get(move || {
                let readiness = health_probe.clone();
                async move { readyz_check(readiness).await }
            }),
        )
        .merge(sdkwork_knowledgebase_observability::metrics_route())
}

pub async fn serve_worker_health(listener: tokio::net::TcpListener, readiness: ReadinessCheck) {
    let listen_addr = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_default();
    tracing::info!(%listen_addr, "knowledgebase worker health endpoint listening");
    axum::serve(listener, worker_health_router(readiness))
        .await
        .expect("serve knowledgebase worker health");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sdkwork_routes_knowledgebase_backend_api::DbReadinessCheck;
    use std::sync::Arc;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn livez_returns_ok_without_readiness_dependency() {
        sqlx::any::install_default_drivers();
        // Lazy pool: livez must not depend on any reachable database, and server
        // persistence is PostgreSQL-only (DATABASE_SPEC: authoritative-server).
        let pool = sqlx::AnyPool::connect_lazy("postgresql://localhost/worker_health_probe")
            .expect("pool options");
        let app = worker_health_router(Arc::new(DbReadinessCheck::new(pool)));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/livez")
                    .body(Body::empty())
                    .expect("livez request"),
            )
            .await
            .expect("livez response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
