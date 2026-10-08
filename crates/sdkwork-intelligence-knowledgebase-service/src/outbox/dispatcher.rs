use std::time::Duration;
use std::{fs::File, io::Read, path::Path, sync::Arc};

use async_trait::async_trait;
use reqwest::Client;
use sdkwork_utils_rust::is_blank;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::Url;
use zeroize::Zeroizing;

use crate::ports::knowledge_outbox_dispatcher::{
    KnowledgeOutboxDispatchError, KnowledgeOutboxDispatcher,
};
use crate::ports::knowledge_outbox_store::{
    PendingOutboxEvent, MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES,
};

const OUTBOX_WEBHOOK_ENV: &str = "SDKWORK_KNOWLEDGEBASE_OUTBOX_WEBHOOK_URL";
const OUTBOX_WEBHOOK_SECRET_FILE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_OUTBOX_WEBHOOK_SECRET_FILE";
const OUTBOX_WEBHOOK_TIMEOUT_SECS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_OUTBOX_WEBHOOK_TIMEOUT_SECS";
const MAXIMUM_WEBHOOK_SECRET_BYTES: u64 = 4 * 1024;
const EVENT_ID_HEADER: &str = "x-sdkwork-event-id";
const EVENT_SEQUENCE_HEADER: &str = "x-sdkwork-event-sequence";
const EVENT_TYPE_HEADER: &str = "x-sdkwork-event-type";
const EVENT_TIME_HEADER: &str = "x-sdkwork-event-time";
const EVENT_RETRY_COUNT_HEADER: &str = "x-sdkwork-event-retry-count";
const EVENT_SIGNATURE_HEADER: &str = "x-sdkwork-event-signature";

pub struct LoggingKnowledgeOutboxDispatcher;

pub struct FailClosedKnowledgeOutboxDispatcher;

#[async_trait]
impl KnowledgeOutboxDispatcher for FailClosedKnowledgeOutboxDispatcher {
    async fn dispatch(
        &self,
        _tenant_id: u64,
        _event: &PendingOutboxEvent,
    ) -> Result<(), KnowledgeOutboxDispatchError> {
        Err(KnowledgeOutboxDispatchError::Internal(
            "the Knowledgebase outbox dispatcher is not configured".to_string(),
        ))
    }
}

#[async_trait]
impl KnowledgeOutboxDispatcher for LoggingKnowledgeOutboxDispatcher {
    async fn dispatch(
        &self,
        tenant_id: u64,
        event: &PendingOutboxEvent,
    ) -> Result<(), KnowledgeOutboxDispatchError> {
        tracing::info!(
            tenant_id,
            event_id = event.id,
            event_type = %event.event_type,
            aggregate_type = %event.aggregate_type,
            aggregate_id = event.aggregate_id,
            "delivered knowledgebase outbox event to logging dispatcher"
        );
        Ok(())
    }
}

pub struct WebhookKnowledgeOutboxDispatcher {
    client: Client,
    webhook_url: Url,
    webhook_secret: Zeroizing<String>,
}

impl WebhookKnowledgeOutboxDispatcher {
    pub fn from_env() -> Result<Self, KnowledgeOutboxDispatchError> {
        let webhook_url = std::env::var(OUTBOX_WEBHOOK_ENV).map_err(|_| {
            KnowledgeOutboxDispatchError::Internal(format!(
                "{OUTBOX_WEBHOOK_ENV} is not configured"
            ))
        })?;
        if is_blank(Some(webhook_url.as_str())) {
            return Err(KnowledgeOutboxDispatchError::Internal(format!(
                "{OUTBOX_WEBHOOK_ENV} must not be blank"
            )));
        }
        let webhook_url = validate_webhook_url(&webhook_url, is_development_environment())?;
        let webhook_secret = read_webhook_secret_file_from_env()?;

        let timeout_secs = std::env::var(OUTBOX_WEBHOOK_TIMEOUT_SECS_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(10)
            .clamp(1, 120);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|error| KnowledgeOutboxDispatchError::Internal(error.to_string()))?;

        Ok(Self {
            client,
            webhook_url,
            webhook_secret,
        })
    }
}

#[async_trait]
impl KnowledgeOutboxDispatcher for WebhookKnowledgeOutboxDispatcher {
    async fn dispatch(
        &self,
        _tenant_id: u64,
        event: &PendingOutboxEvent,
    ) -> Result<(), KnowledgeOutboxDispatchError> {
        if event.payload_json.len() > MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES {
            return Err(KnowledgeOutboxDispatchError::Internal(format!(
                "outbox payload exceeds {MAX_KNOWLEDGE_OUTBOX_PAYLOAD_BYTES} bytes"
            )));
        }
        let event_time = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        serde_json::from_str::<serde_json::Value>(&event.payload_json).map_err(|_| {
            KnowledgeOutboxDispatchError::Internal(
                "outbox payload is not a valid JSON event".to_string(),
            )
        })?;
        let body_bytes = event.payload_json.as_bytes();

        let request = self
            .client
            .post(self.webhook_url.clone())
            .header("content-type", "application/json")
            .header(EVENT_ID_HEADER, event.event_uuid.as_str())
            .header(EVENT_SEQUENCE_HEADER, event.id.to_string())
            .header(EVENT_TYPE_HEADER, event.event_type.as_str())
            .header(EVENT_TIME_HEADER, event_time.as_str())
            .header(EVENT_RETRY_COUNT_HEADER, event.retry_count.to_string())
            .header(
                EVENT_SIGNATURE_HEADER,
                sign_webhook_payload(&self.webhook_secret, &event_time, body_bytes)?,
            );

        let response = request
            .body(event.payload_json.clone())
            .send()
            .await
            .map_err(|error| KnowledgeOutboxDispatchError::DeliveryFailed(error.to_string()))?;

        if !response.status().is_success() {
            return Err(KnowledgeOutboxDispatchError::DeliveryFailed(format!(
                "webhook returned HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }
}

fn sign_webhook_payload(
    secret: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<String, KnowledgeOutboxDispatchError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|error| KnowledgeOutboxDispatchError::Internal(error.to_string()))?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let signature = mac.finalize().into_bytes();
    Ok(format!(
        "sha256={}",
        signature
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn validate_webhook_url(
    value: &str,
    development: bool,
) -> Result<Url, KnowledgeOutboxDispatchError> {
    let url = Url::parse(value).map_err(|_| {
        KnowledgeOutboxDispatchError::Internal(format!(
            "{OUTBOX_WEBHOOK_ENV} must be an absolute URL"
        ))
    })?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(KnowledgeOutboxDispatchError::Internal(format!(
            "{OUTBOX_WEBHOOK_ENV} must not contain credentials, query, or fragment"
        )));
    }
    let secure = url.scheme() == "https";
    let development_loopback = development
        && url.scheme() == "http"
        && url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
    if !secure && !development_loopback {
        return Err(KnowledgeOutboxDispatchError::Internal(format!(
            "{OUTBOX_WEBHOOK_ENV} must use HTTPS, except for a development loopback URL"
        )));
    }
    Ok(url)
}

fn read_webhook_secret_file_from_env() -> Result<Zeroizing<String>, KnowledgeOutboxDispatchError> {
    let path = std::env::var(OUTBOX_WEBHOOK_SECRET_FILE_ENV).map_err(|_| {
        KnowledgeOutboxDispatchError::Internal(format!(
            "{OUTBOX_WEBHOOK_SECRET_FILE_ENV} is not configured"
        ))
    })?;
    if is_blank(Some(path.as_str())) {
        return Err(KnowledgeOutboxDispatchError::Internal(format!(
            "{OUTBOX_WEBHOOK_SECRET_FILE_ENV} must not be blank"
        )));
    }
    read_webhook_secret_file(Path::new(&path))
}

fn read_webhook_secret_file(
    path: &Path,
) -> Result<Zeroizing<String>, KnowledgeOutboxDispatchError> {
    let file = File::open(path).map_err(|_| {
        KnowledgeOutboxDispatchError::Internal(
            "the outbox webhook secret file is not readable".to_string(),
        )
    })?;
    let metadata = file.metadata().map_err(|_| {
        KnowledgeOutboxDispatchError::Internal(
            "the outbox webhook secret file metadata is unavailable".to_string(),
        )
    })?;
    if !metadata.is_file() || metadata.len() < 32 || metadata.len() > MAXIMUM_WEBHOOK_SECRET_BYTES {
        return Err(KnowledgeOutboxDispatchError::Internal(
            "the outbox webhook secret file must contain 32 to 4096 bytes".to_string(),
        ));
    }
    let mut secret = Zeroizing::new(String::with_capacity(metadata.len() as usize));
    file.take(MAXIMUM_WEBHOOK_SECRET_BYTES + 1)
        .read_to_string(&mut secret)
        .map_err(|_| {
            KnowledgeOutboxDispatchError::Internal(
                "the outbox webhook secret file must contain UTF-8 text".to_string(),
            )
        })?;
    while matches!(secret.as_bytes().last(), Some(b'\r' | b'\n')) {
        secret.pop();
    }
    if secret.len() < 32
        || secret.len() > MAXIMUM_WEBHOOK_SECRET_BYTES as usize
        || secret.chars().any(char::is_control)
    {
        return Err(KnowledgeOutboxDispatchError::Internal(
            "the outbox webhook secret is outside its security bounds".to_string(),
        ));
    }
    Ok(secret)
}

fn is_development_environment() -> bool {
    std::env::var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT")
        .ok()
        .is_some_and(|value| value.eq_ignore_ascii_case("development"))
}

pub fn knowledge_outbox_dispatcher_from_env() -> Arc<dyn KnowledgeOutboxDispatcher> {
    match WebhookKnowledgeOutboxDispatcher::from_env() {
        Ok(dispatcher) => Arc::new(dispatcher),
        Err(error) => {
            if !is_development_environment() {
                tracing::error!(
                    error = %error,
                    "outbox webhook dispatcher is required outside development"
                );
                return Arc::new(FailClosedKnowledgeOutboxDispatcher);
            }
            tracing::debug!(
                error = %error,
                "outbox webhook dispatcher unavailable; using logging dispatcher for development"
            );
            Arc::new(LoggingKnowledgeOutboxDispatcher)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    #[test]
    fn webhook_url_requires_https_outside_development() {
        assert!(validate_webhook_url("https://events.example.test/wiki", false).is_ok());
        assert!(validate_webhook_url("http://127.0.0.1:8080/wiki", true).is_ok());
        assert!(validate_webhook_url("http://events.example.test/wiki", true).is_err());
        assert!(validate_webhook_url("http://127.0.0.1:8080/wiki", false).is_err());
        assert!(
            validate_webhook_url("https://user:secret@events.example.test/wiki", false).is_err()
        );
        assert!(
            validate_webhook_url("https://events.example.test/wiki?token=secret", false).is_err()
        );
    }

    #[tokio::test]
    async fn webhook_sends_the_authority_event_unchanged_with_signed_delivery_metadata() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let dispatcher = WebhookKnowledgeOutboxDispatcher {
            client: Client::new(),
            webhook_url: Url::parse(&server.uri()).expect("wiremock URL"),
            webhook_secret: Zeroizing::new("test-only-webhook-signing-secret-32-bytes".to_string()),
        };
        let payload = r#"{"id":"b9cb15ba-f69a-4ab5-a34f-a80ba9348680","type":"knowledgebase.wiki.route.revoked.v1","specversion":"1.0","source":"sdkwork-knowledgebase","time":"2026-07-21T00:00:00Z","tenantId":"100001","organizationId":"0","subject":"wiki-publication:2ca86ece-5057-459c-99b6-e57d889efea0","sequenceNo":"42","data":{"providerResourceUuid":"2ca86ece-5057-459c-99b6-e57d889efea0","providerGeneration":"3","navigationGeneration":"4","searchGeneration":"5","operation":"REVOKE"}}"#;
        let event = PendingOutboxEvent {
            id: 42,
            event_uuid: "b9cb15ba-f69a-4ab5-a34f-a80ba9348680".to_string(),
            event_type: "knowledgebase.wiki.route.revoked.v1".to_string(),
            aggregate_type: "wiki_publication".to_string(),
            aggregate_id: 7,
            retry_count: 2,
            payload_json: payload.to_string(),
        };

        dispatcher
            .dispatch(100_001, &event)
            .await
            .expect("dispatch");

        let requests = server.received_requests().await.expect("received requests");
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.body.as_slice(), payload.as_bytes());
        assert_eq!(request.headers[EVENT_ID_HEADER], event.event_uuid);
        assert_eq!(request.headers[EVENT_SEQUENCE_HEADER], "42");
        assert_eq!(request.headers[EVENT_TYPE_HEADER], event.event_type);
        assert_eq!(request.headers[EVENT_RETRY_COUNT_HEADER], "2");
        let event_time = request.headers[EVENT_TIME_HEADER]
            .to_str()
            .expect("event time header");
        let expected = sign_webhook_payload(
            &dispatcher.webhook_secret,
            event_time,
            payload.as_bytes(),
        )
        .expect("webhook signing accepts the configured secret");
        assert_eq!(request.headers[EVENT_SIGNATURE_HEADER], expected);
    }
}
