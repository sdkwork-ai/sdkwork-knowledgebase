//! Optional OpenTelemetry OTLP export — enable the `otel` crate feature.

#[cfg(feature = "otel")]
pub fn init_otel_tracing(
    service_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::SdkTracerProvider;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_owned());
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();
    let tracer = provider.tracer(service_name.to_owned());
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let json_logs = std::env::var("SDKWORK_KNOWLEDGEBASE_LOG_FORMAT")
        .map(|value| value.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!("info,sdkwork_knowledgebase_{service_name}=debug"))
    });

    let fmt_layer = if json_logs {
        tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(false)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer().boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(telemetry)
        .try_init()?;

    tracing::info!(
        service = service_name,
        "knowledgebase otel tracing initialized"
    );
    Ok(())
}

#[cfg(not(feature = "otel"))]
pub fn init_otel_tracing(
    _service_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Err("sdkwork-knowledgebase-observability compiled without `otel` feature".into())
}
