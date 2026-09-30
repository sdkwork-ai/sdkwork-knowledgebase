use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH, CONTENT_TYPE, RETRY_AFTER,
};
use reqwest::{Client, Method, StatusCode, Url};
use sdkwork_knowledgebase_contract::provider_binding::{
    KnowledgeEngineDataScope, KnowledgeEngineExecutionContext,
};
use sdkwork_utils_rust::{is_blank, SDKWORK_TRACE_ID_HEADER};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::{OnceCell, Semaphore};

use crate::telemetry::default_telemetry;
use crate::{
    ProviderError, ProviderErrorCategory, ProviderOperation, ProviderRuntimeConfig,
    ProviderTelemetry, ProviderTelemetryEvent,
};

#[derive(Debug, Clone)]
pub struct ProviderExecutionContext {
    pub implementation_id: String,
    pub tenant_id: u64,
    pub organization_id: u64,
    pub actor_id: String,
    pub permission_scope: Vec<String>,
    pub data_scope: KnowledgeEngineDataScope,
    pub space_id: u64,
    pub binding_id: Option<String>,
    pub trace_id: String,
    pub deadline: Option<Instant>,
}

impl ProviderExecutionContext {
    pub fn new(implementation_id: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self {
            implementation_id: implementation_id.into(),
            tenant_id: 0,
            organization_id: 0,
            actor_id: String::new(),
            permission_scope: Vec::new(),
            data_scope: KnowledgeEngineDataScope {
                allowed_space_ids: Vec::new(),
                allowed_source_ids: Vec::new(),
                allowed_document_ids: Vec::new(),
            },
            space_id: 0,
            binding_id: None,
            trace_id: trace_id.into(),
            deadline: None,
        }
    }

    pub fn for_system_health(implementation_id: impl Into<String>) -> Self {
        Self::new(implementation_id, sdkwork_utils_rust::uuid())
    }

    pub fn from_knowledge_engine(
        context: &KnowledgeEngineExecutionContext,
        implementation_id: impl Into<String>,
        operation: ProviderOperation,
    ) -> Result<Self, ProviderError> {
        let implementation_id = implementation_id.into();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                ProviderError::new(
                    ProviderErrorCategory::Internal,
                    operation,
                    implementation_id.clone(),
                    context.binding_id.map(|value| value.to_string()),
                    None,
                    false,
                    None,
                    "system clock is unavailable",
                )
            })?
            .as_millis();
        let remaining_ms = u128::from(context.deadline_unix_ms)
            .checked_sub(now)
            .filter(|remaining| *remaining > 0)
            .ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorCategory::Timeout,
                    operation,
                    implementation_id.clone(),
                    context.binding_id.map(|value| value.to_string()),
                    None,
                    false,
                    None,
                    "provider request deadline has expired",
                )
            })?;
        let remaining = Duration::from_millis(u64::try_from(remaining_ms).unwrap_or(u64::MAX));
        let deadline = Instant::now().checked_add(remaining).ok_or_else(|| {
            ProviderError::new(
                ProviderErrorCategory::Validation,
                operation,
                implementation_id.clone(),
                context.binding_id.map(|value| value.to_string()),
                None,
                false,
                None,
                "provider request deadline is outside the supported range",
            )
        })?;
        let provider_context = Self {
            implementation_id,
            tenant_id: context.tenant_id,
            organization_id: context.organization_id,
            actor_id: context.actor_id.clone(),
            permission_scope: context.permission_scope.clone(),
            data_scope: context.data_scope.clone(),
            space_id: context.space_id,
            binding_id: context.binding_id.map(|value| value.to_string()),
            trace_id: context.trace_id.clone(),
            deadline: Some(deadline),
        };
        provider_context.validate_for_operation(operation)?;
        Ok(provider_context)
    }

    pub fn from_knowledge_engine_request(
        context: &KnowledgeEngineExecutionContext,
        implementation_id: impl Into<String>,
        operation: ProviderOperation,
        request_tenant_id: u64,
        request_space_id: u64,
    ) -> Result<Self, ProviderError> {
        let implementation_id = implementation_id.into();
        if request_tenant_id == 0
            || request_tenant_id != context.tenant_id
            || request_space_id == 0
            || request_space_id != context.space_id
        {
            return Err(ProviderError::new(
                ProviderErrorCategory::PermissionDenied,
                operation,
                implementation_id,
                context.binding_id.map(|value| value.to_string()),
                None,
                false,
                None,
                "Provider request scope does not match the execution context",
            ));
        }
        Self::from_knowledge_engine(context, implementation_id, operation)
    }

    fn validate_for_operation(&self, operation: ProviderOperation) -> Result<(), ProviderError> {
        if is_blank(Some(self.trace_id.as_str())) {
            return Err(self.context_error(
                operation,
                ProviderErrorCategory::Validation,
                "provider execution trace id is required",
            ));
        }
        if let Some(deadline) = self.deadline {
            if deadline <= Instant::now() {
                return Err(self.context_error(
                    operation,
                    ProviderErrorCategory::Timeout,
                    "provider request deadline has expired",
                ));
            }
        }
        if operation == ProviderOperation::Health && self.tenant_id == 0 {
            return Ok(());
        }
        if self.tenant_id == 0 || is_blank(Some(self.actor_id.as_str())) {
            return Err(self.context_error(
                operation,
                ProviderErrorCategory::PermissionDenied,
                "authenticated Provider execution scope is required",
            ));
        }
        if self.space_id == 0
            || self.data_scope.allowed_space_ids.is_empty()
            || !self.data_scope.allowed_space_ids.contains(&self.space_id)
        {
            return Err(self.context_error(
                operation,
                ProviderErrorCategory::PermissionDenied,
                "Provider execution space is outside the authenticated data scope",
            ));
        }
        if self.binding_id.as_deref().is_none_or(str::is_empty) {
            return Err(self.context_error(
                operation,
                ProviderErrorCategory::Validation,
                "active Provider binding identity is required",
            ));
        }
        if self.deadline.is_none() {
            return Err(self.context_error(
                operation,
                ProviderErrorCategory::Validation,
                "provider request deadline is required",
            ));
        }
        Ok(())
    }

    fn has_authenticated_scope(&self) -> bool {
        self.tenant_id != 0
            || !self.actor_id.is_empty()
            || self.space_id != 0
            || self.binding_id.is_some()
            || self.deadline.is_some()
    }

    fn context_error(
        &self,
        operation: ProviderOperation,
        category: ProviderErrorCategory,
        safe_message: &'static str,
    ) -> ProviderError {
        ProviderError::new(
            category,
            operation,
            self.implementation_id.clone(),
            self.binding_id.clone(),
            None,
            false,
            None,
            safe_message,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProviderHttpRequest {
    pub operation: ProviderOperation,
    pub method: Method,
    pub url: Url,
    pub headers: HeaderMap,
    pub body: Option<Vec<u8>>,
    pub idempotent: bool,
    pub max_response_bytes: Option<usize>,
}

impl ProviderHttpRequest {
    pub fn new(
        operation: ProviderOperation,
        method: Method,
        url: impl AsRef<str>,
    ) -> Result<Self, ProviderError> {
        let url = Url::parse(url.as_ref()).map_err(|_| {
            ProviderError::new(
                ProviderErrorCategory::InvalidTarget,
                operation,
                "unresolved",
                None,
                None,
                false,
                None,
                "provider request URL is invalid",
            )
        })?;
        Ok(Self {
            operation,
            method,
            url,
            headers: HeaderMap::new(),
            body: None,
            idempotent: false,
            max_response_bytes: None,
        })
    }

    pub fn idempotent(mut self, idempotent: bool) -> Self {
        self.idempotent = idempotent;
        self
    }

    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn bearer_auth(self, token: &str) -> Result<Self, ProviderError> {
        self.sensitive_header("authorization", &format!("Bearer {token}"))
    }

    pub fn optional_bearer_auth(self, token: Option<&str>) -> Result<Self, ProviderError> {
        match token {
            Some(token) => self.bearer_auth(token),
            None => Ok(self),
        }
    }

    pub fn sensitive_header(mut self, name: &str, value: &str) -> Result<Self, ProviderError> {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| self.validation("invalid provider header name"))?;
        let mut value = HeaderValue::from_str(value)
            .map_err(|_| self.validation("invalid provider header value"))?;
        value.set_sensitive(true);
        self.headers.insert(name, value);
        Ok(self)
    }

    pub fn json<T: Serialize>(mut self, value: &T) -> Result<Self, ProviderError> {
        self.body = Some(
            serde_json::to_vec(value)
                .map_err(|_| self.validation("provider request JSON serialization failed"))?,
        );
        self.headers
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(self)
    }

    fn validation(&self, message: &str) -> ProviderError {
        ProviderError::new(
            ProviderErrorCategory::Validation,
            self.operation,
            "unresolved",
            None,
            None,
            false,
            None,
            message,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProviderHttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    operation: ProviderOperation,
    implementation_id: String,
    binding_id: Option<String>,
}

impl ProviderHttpResponse {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, ProviderError> {
        serde_json::from_slice(&self.body).map_err(|_| {
            ProviderError::new(
                ProviderErrorCategory::InvalidResponse,
                self.operation,
                self.implementation_id.clone(),
                self.binding_id.clone(),
                Some(self.status.as_u16()),
                false,
                None,
                "provider returned invalid JSON",
            )
        })
    }
}

#[derive(Debug, Default)]
struct CircuitState {
    consecutive_failures: u32,
    open_until: Option<Instant>,
}

#[derive(Clone)]
pub struct ProviderRuntime {
    /// Lazily built HTTP client whose target host is DNS-resolved, validated as public,
    /// and pinned (every public resolved address, so a provider's DNS failover records
    /// stay reachable), so a rebinding DNS record cannot redirect provider traffic into
    /// an internal network after validation. Only successful builds are cached: a
    /// transient DNS or construction failure retries on the next call instead of
    /// poisoning the provider for the process lifetime.
    client: OnceCell<Client>,
    base_url: Url,
    config: ProviderRuntimeConfig,
    concurrency: Arc<Semaphore>,
    circuit: Arc<Mutex<CircuitState>>,
    telemetry: Arc<dyn ProviderTelemetry>,
}

impl ProviderRuntime {
    pub fn for_base_url(base_url: &str) -> Result<Self, ProviderError> {
        Self::new(ProviderRuntimeConfig::for_base_url(base_url)?)
    }

    pub fn new(config: ProviderRuntimeConfig) -> Result<Self, ProviderError> {
        config.validate()?;
        let base_url = config.allowed_origin.to_url().map_err(|_| {
            ProviderError::new(
                ProviderErrorCategory::Internal,
                ProviderOperation::Health,
                "unresolved",
                None,
                None,
                false,
                None,
                "provider origin URL cannot be reconstructed",
            )
        })?;
        Ok(Self {
            client: OnceCell::new(),
            base_url,
            concurrency: Arc::new(Semaphore::new(config.max_concurrency)),
            circuit: Arc::new(Mutex::new(CircuitState::default())),
            telemetry: default_telemetry(),
            config,
        })
    }

    pub fn with_telemetry(mut self, telemetry: Arc<dyn ProviderTelemetry>) -> Self {
        self.telemetry = telemetry;
        self
    }

    async fn pinned_client(&self) -> Result<&Client, ProviderError> {
        self.client
            .get_or_try_init(|| build_pinned_client(&self.base_url, &self.config))
            .await
    }

    pub async fn execute(
        &self,
        context: &ProviderExecutionContext,
        request: ProviderHttpRequest,
    ) -> Result<ProviderHttpResponse, ProviderError> {
        let started = Instant::now();
        if context.has_authenticated_scope() {
            context.validate_for_operation(request.operation)?;
        }
        tracing::debug!(
            implementation_id = %context.implementation_id,
            operation = %request.operation,
            "provider operation started"
        );
        if let Err(error) = self
            .config
            .allowed_origin
            .validate(&request.url, self.config.target_policy)
            .map_err(|error| self.contextualize(error, context, request.operation))
        {
            self.record_telemetry(
                context,
                request.operation,
                Some(error.category),
                error.status_code,
                0,
                started.elapsed(),
                0,
            );
            return Err(error);
        }
        if let Err(error) = self.ensure_circuit_closed(context, request.operation) {
            self.record_telemetry(
                context,
                request.operation,
                Some(error.category),
                error.status_code,
                0,
                started.elapsed(),
                0,
            );
            return Err(error);
        }
        let _permit = match self.concurrency.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let error = self.error(
                    context,
                    request.operation,
                    ProviderErrorCategory::BulkheadSaturated,
                    None,
                    true,
                    None,
                    "provider concurrency limit is saturated",
                );
                self.record_telemetry(
                    context,
                    request.operation,
                    Some(error.category),
                    error.status_code,
                    0,
                    started.elapsed(),
                    0,
                );
                return Err(error);
            }
        };

        let max_response_bytes = request
            .max_response_bytes
            .unwrap_or(self.config.max_response_bytes)
            .min(self.config.max_response_bytes);
        let mut attempts = 0;
        loop {
            attempts += 1;
            let result = self
                .execute_once(context, &request, max_response_bytes)
                .await;
            match result {
                Ok(response) => {
                    self.record_success();
                    self.record_telemetry(
                        context,
                        request.operation,
                        None,
                        Some(response.status.as_u16()),
                        attempts,
                        started.elapsed(),
                        response.body.len(),
                    );
                    return Ok(response);
                }
                Err(error)
                    if request.idempotent
                        && error.retryable
                        && attempts < self.config.max_attempts =>
                {
                    let delay = error
                        .retry_after
                        .unwrap_or_else(|| self.retry_delay(context, attempts));
                    if let Err(error) = self
                        .sleep_with_deadline(context, request.operation, delay)
                        .await
                    {
                        self.record_failure();
                        self.record_telemetry(
                            context,
                            request.operation,
                            Some(error.category),
                            error.status_code,
                            attempts,
                            started.elapsed(),
                            0,
                        );
                        return Err(error);
                    }
                }
                Err(error) => {
                    if error.retryable {
                        self.record_failure();
                    }
                    self.record_telemetry(
                        context,
                        request.operation,
                        Some(error.category),
                        error.status_code,
                        attempts,
                        started.elapsed(),
                        0,
                    );
                    return Err(error);
                }
            }
        }
    }

    async fn execute_once(
        &self,
        context: &ProviderExecutionContext,
        request: &ProviderHttpRequest,
        max_response_bytes: usize,
    ) -> Result<ProviderHttpResponse, ProviderError> {
        let client = self.pinned_client().await?;
        let mut headers = request.headers.clone();
        if !is_blank(Some(context.trace_id.as_str())) {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(SDKWORK_TRACE_ID_HEADER.as_bytes()),
                HeaderValue::from_str(context.trace_id.trim()),
            ) {
                headers.insert(name, value);
            }
        }
        let mut builder = client
            .request(request.method.clone(), request.url.clone())
            .headers(headers);
        if let Some(body) = request.body.clone() {
            builder = builder.body(body);
        }

        let timeout = self.remaining_timeout(context, request.operation)?;
        let response = tokio::time::timeout(timeout, builder.send())
            .await
            .map_err(|_| self.timeout_error(context, request.operation))?
            .map_err(|error| {
                let category = if error.is_timeout() {
                    ProviderErrorCategory::Timeout
                } else {
                    ProviderErrorCategory::Unavailable
                };
                self.error(
                    context,
                    request.operation,
                    category,
                    None,
                    true,
                    None,
                    "provider request failed",
                )
            })?;
        let status = response.status();
        let retry_after = parse_retry_after(response.headers().get(RETRY_AFTER));
        let declared_length = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());
        let response_headers = response.headers().clone();
        if declared_length.is_some_and(|length| length > max_response_bytes) {
            return Err(self.error(
                context,
                request.operation,
                ProviderErrorCategory::ResponseTooLarge,
                Some(status.as_u16()),
                false,
                None,
                "provider response exceeds the configured byte limit",
            ));
        }

        if !status.is_success() {
            let preview_limit = self.config.max_error_preview_bytes.min(max_response_bytes);
            let _bounded_diagnostic_body = collect_bounded(response, preview_limit).await;
            let (category, retryable) = classify_status(status);
            let detail = format!("provider returned HTTP {}", status.as_u16());
            return Err(self.error(
                context,
                request.operation,
                category,
                Some(status.as_u16()),
                retryable,
                retry_after,
                &detail,
            ));
        }

        let body = collect_bounded(response, max_response_bytes)
            .await
            .map_err(|category| {
                self.error(
                    context,
                    request.operation,
                    category,
                    Some(status.as_u16()),
                    category != ProviderErrorCategory::ResponseTooLarge,
                    None,
                    if category == ProviderErrorCategory::ResponseTooLarge {
                        "provider response exceeds the configured byte limit"
                    } else {
                        "provider response body could not be read"
                    },
                )
            })?;
        Ok(ProviderHttpResponse {
            status,
            headers: response_headers,
            body,
            operation: request.operation,
            implementation_id: context.implementation_id.clone(),
            binding_id: context.binding_id.clone(),
        })
    }

    fn remaining_timeout(
        &self,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
    ) -> Result<Duration, ProviderError> {
        match context.deadline {
            Some(deadline) => deadline
                .checked_duration_since(Instant::now())
                .filter(|remaining| !remaining.is_zero())
                .map(|remaining| remaining.min(self.config.request_timeout))
                .ok_or_else(|| self.timeout_error(context, operation)),
            None => Ok(self.config.request_timeout),
        }
    }

    async fn sleep_with_deadline(
        &self,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
        delay: Duration,
    ) -> Result<(), ProviderError> {
        if context
            .deadline
            .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
            .is_some_and(|remaining| remaining <= delay)
        {
            return Err(self.timeout_error(context, operation));
        }
        tokio::time::sleep(delay.min(self.config.retry_max_delay)).await;
        Ok(())
    }

    fn retry_delay(&self, context: &ProviderExecutionContext, attempt: u32) -> Duration {
        let exponent = attempt.saturating_sub(1).min(16);
        let base_ms = self
            .config
            .retry_base_delay
            .as_millis()
            .saturating_mul(1_u128 << exponent)
            .min(self.config.retry_max_delay.as_millis());
        let mut hasher = DefaultHasher::new();
        context.trace_id.hash(&mut hasher);
        attempt.hash(&mut hasher);
        let jitter_percent = 50 + (hasher.finish() % 51) as u128;
        Duration::from_millis(
            base_ms
                .saturating_mul(jitter_percent)
                .saturating_div(100)
                .min(u128::from(u64::MAX)) as u64,
        )
    }

    fn ensure_circuit_closed(
        &self,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
    ) -> Result<(), ProviderError> {
        let mut state = self
            .circuit
            .lock()
            .expect("provider circuit mutex poisoned");
        if state.open_until.is_some_and(|until| until > Instant::now()) {
            return Err(self.error(
                context,
                operation,
                ProviderErrorCategory::CircuitOpen,
                None,
                true,
                state
                    .open_until
                    .and_then(|until| until.checked_duration_since(Instant::now())),
                "provider circuit is open",
            ));
        }
        if state.open_until.is_some() {
            state.open_until = None;
            state.consecutive_failures = 0;
        }
        Ok(())
    }

    fn record_success(&self) {
        let mut state = self
            .circuit
            .lock()
            .expect("provider circuit mutex poisoned");
        state.consecutive_failures = 0;
        state.open_until = None;
    }

    fn record_failure(&self) {
        let mut state = self
            .circuit
            .lock()
            .expect("provider circuit mutex poisoned");
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= self.config.circuit_failure_threshold {
            state.open_until = Some(Instant::now() + self.config.circuit_open_duration);
        }
    }

    fn contextualize(
        &self,
        mut error: ProviderError,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
    ) -> ProviderError {
        error.operation = operation;
        error.implementation_id = context.implementation_id.clone();
        error.binding_id = context.binding_id.clone();
        error
    }

    #[allow(clippy::too_many_arguments)]
    fn error(
        &self,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
        category: ProviderErrorCategory,
        status_code: Option<u16>,
        retryable: bool,
        retry_after: Option<Duration>,
        safe_message: &str,
    ) -> ProviderError {
        ProviderError::new(
            category,
            operation,
            context.implementation_id.clone(),
            context.binding_id.clone(),
            status_code,
            retryable,
            retry_after,
            safe_message,
        )
    }

    fn timeout_error(
        &self,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
    ) -> ProviderError {
        self.error(
            context,
            operation,
            ProviderErrorCategory::Timeout,
            None,
            true,
            None,
            "provider operation deadline exceeded",
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn record_telemetry(
        &self,
        context: &ProviderExecutionContext,
        operation: ProviderOperation,
        error_category: Option<ProviderErrorCategory>,
        status_code: Option<u16>,
        attempts: u32,
        duration: Duration,
        response_bytes: usize,
    ) {
        self.telemetry.record(ProviderTelemetryEvent {
            implementation_id: context.implementation_id.clone(),
            operation,
            error_category,
            status_code,
            attempts,
            duration,
            response_bytes,
        });
    }
}

async fn collect_bounded(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, ProviderErrorCategory> {
    let mut body = Vec::with_capacity(limit.min(16 * 1024));
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|_| ProviderErrorCategory::Unavailable)?;
        let Some(chunk) = chunk else {
            return Ok(body);
        };
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(ProviderErrorCategory::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
}

fn classify_status(status: StatusCode) -> (ProviderErrorCategory, bool) {
    match status {
        StatusCode::UNAUTHORIZED => (ProviderErrorCategory::Authentication, false),
        StatusCode::FORBIDDEN => (ProviderErrorCategory::PermissionDenied, false),
        StatusCode::NOT_FOUND => (ProviderErrorCategory::NotFound, false),
        StatusCode::TOO_MANY_REQUESTS => (ProviderErrorCategory::RateLimited, true),
        status if status.is_server_error() => (ProviderErrorCategory::Unavailable, true),
        _ => (ProviderErrorCategory::InvalidResponse, false),
    }
}

fn parse_retry_after(value: Option<&HeaderValue>) -> Option<Duration> {
    let value = value?.to_str().ok()?.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let retry_at = httpdate::parse_http_date(value).ok()?;
    retry_at.duration_since(SystemTime::now()).ok()
}

/// Builds the reqwest client for one provider origin with DNS validation and socket
/// pinning: the origin hostname is resolved, every address is required to be public,
/// and the validated socket is pinned on the client so a later DNS rebinding cannot
/// redirect provider traffic into an internal network.
async fn build_pinned_client(
    base_url: &Url,
    config: &ProviderRuntimeConfig,
) -> Result<Client, ProviderError> {
    let sockets =
        crate::target_security::resolve_public_socket_addrs(base_url, config.connect_timeout)
            .await?;
    let mut builder = Client::builder()
        .connect_timeout(config.connect_timeout)
        .timeout(config.request_timeout)
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(config.max_concurrency)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("sdkwork-knowledgebase-provider-runtime/0.1");
    if let Some(host) = base_url.host_str() {
        if host.parse::<std::net::IpAddr>().is_err() {
            builder = builder.resolve_to_addrs(host, &sockets);
        }
    }
    builder.build().map_err(|_| {
        ProviderError::new(
            ProviderErrorCategory::Internal,
            ProviderOperation::Health,
            "unresolved",
            None,
            None,
            false,
            None,
            "provider HTTP client construction failed",
        )
    })
}
