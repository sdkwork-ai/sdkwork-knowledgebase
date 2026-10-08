use std::time::Duration;

use reqwest::Url;

use crate::{ProviderError, ProviderErrorCategory, ProviderOperation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTargetPolicy {
    Production,
    Development,
}

impl ProviderTargetPolicy {
    pub fn from_environment() -> Self {
        match std::env::var("SDKWORK_KNOWLEDGEBASE_ENVIRONMENT") {
            Ok(value)
                if value.eq_ignore_ascii_case("production")
                    || value.eq_ignore_ascii_case("staging") =>
            {
                Self::Production
            }
            _ => Self::Development,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderOrigin {
    scheme: String,
    host: String,
    port: u16,
}

impl ProviderOrigin {
    pub fn parse(value: &str, target_policy: ProviderTargetPolicy) -> Result<Self, ProviderError> {
        let url = Url::parse(value).map_err(|_| target_error("provider base URL is invalid"))?;
        validate_url_shape(&url, target_policy)?;
        Ok(Self {
            scheme: url.scheme().to_ascii_lowercase(),
            host: url.host_str().expect("validated host").to_ascii_lowercase(),
            port: url
                .port_or_known_default()
                .ok_or_else(|| target_error("provider base URL has no valid port"))?,
        })
    }

    pub(crate) fn validate(
        &self,
        url: &Url,
        policy: ProviderTargetPolicy,
    ) -> Result<(), ProviderError> {
        validate_url_shape(url, policy)?;
        let candidate = Self {
            scheme: url.scheme().to_ascii_lowercase(),
            host: url.host_str().expect("validated host").to_ascii_lowercase(),
            port: url
                .port_or_known_default()
                .ok_or_else(|| target_error("provider URL has no valid port"))?,
        };
        if candidate != *self {
            return Err(target_error(
                "provider request URL does not match the configured origin",
            ));
        }
        Ok(())
    }

    /// Reconstructs the canonical origin URL for DNS resolution and socket pinning.
    pub(crate) fn to_url(&self) -> Result<Url, ProviderError> {
        Url::parse(&format!("{}://{}:{}", self.scheme, self.host, self.port))
            .map_err(|_| target_error("provider origin URL cannot be reconstructed"))
    }
}

fn validate_url_shape(url: &Url, policy: ProviderTargetPolicy) -> Result<(), ProviderError> {
    let scheme_allowed = match policy {
        ProviderTargetPolicy::Production => url.scheme() == "https",
        ProviderTargetPolicy::Development => matches!(url.scheme(), "http" | "https"),
    };
    if !scheme_allowed {
        return Err(target_error(
            "provider URL scheme is not allowed for the active environment",
        ));
    }
    if url.host_str().is_none() {
        return Err(target_error("provider URL host is required"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(target_error(
            "provider URL must not contain embedded credentials",
        ));
    }
    if url.fragment().is_some() {
        return Err(target_error("provider URL must not contain a fragment"));
    }
    Ok(())
}

fn target_error(message: &str) -> ProviderError {
    ProviderError::new(
        ProviderErrorCategory::InvalidTarget,
        ProviderOperation::Health,
        "unresolved",
        None,
        None,
        false,
        None,
        message,
    )
}

#[derive(Debug, Clone)]
pub struct ProviderRuntimeConfig {
    pub target_policy: ProviderTargetPolicy,
    /// Explicit per-runtime opt-in for provider targets on private network segments
    /// (loopback, RFC1918, link-local). The default `false` fails closed: such
    /// targets are rejected with the typed `InvalidTarget` error. Self-hosted engine
    /// deployments set this to `true` — each engine config parses an explicit
    /// `<ENGINE>_ALLOW_PRIVATE_NETWORK=1|true` flag — when the engine's production
    /// placement is a trusted private network segment. Setting it is an operator
    /// trust decision for that segment; DNS-resolution socket pinning still applies.
    pub allow_private_network_targets: bool,
    pub allowed_origin: ProviderOrigin,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_response_bytes: usize,
    pub max_error_preview_bytes: usize,
    pub max_attempts: u32,
    pub retry_base_delay: Duration,
    pub retry_max_delay: Duration,
    pub circuit_failure_threshold: u32,
    pub circuit_open_duration: Duration,
    pub max_concurrency: usize,
}

impl ProviderRuntimeConfig {
    pub fn for_base_url(base_url: &str) -> Result<Self, ProviderError> {
        Self::for_base_url_with_private_targets(base_url, false)
    }

    pub fn for_base_url_with_policy(
        base_url: &str,
        target_policy: ProviderTargetPolicy,
    ) -> Result<Self, ProviderError> {
        Self::for_base_url_with_target_options(base_url, target_policy, false)
    }

    /// Builds a config for `base_url` with the target policy resolved from the
    /// environment and an explicit private-network opt-in. Fail-closed when the flag
    /// is `false`.
    pub fn for_base_url_with_private_targets(
        base_url: &str,
        allow_private_network_targets: bool,
    ) -> Result<Self, ProviderError> {
        Self::for_base_url_with_target_options(
            base_url,
            ProviderTargetPolicy::from_environment(),
            allow_private_network_targets,
        )
    }

    pub fn for_base_url_with_target_options(
        base_url: &str,
        target_policy: ProviderTargetPolicy,
        allow_private_network_targets: bool,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            target_policy,
            allow_private_network_targets,
            allowed_origin: ProviderOrigin::parse(base_url, target_policy)?,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            max_response_bytes: 4 * 1024 * 1024,
            max_error_preview_bytes: 8 * 1024,
            max_attempts: 3,
            retry_base_delay: Duration::from_millis(100),
            retry_max_delay: Duration::from_secs(2),
            circuit_failure_threshold: 5,
            circuit_open_duration: Duration::from_secs(30),
            max_concurrency: 32,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), ProviderError> {
        if self.connect_timeout.is_zero()
            || self.request_timeout.is_zero()
            || self.max_response_bytes == 0
            || self.max_error_preview_bytes == 0
            || self.max_attempts == 0
            || self.circuit_failure_threshold == 0
            || self.max_concurrency == 0
        {
            return Err(ProviderError::new(
                ProviderErrorCategory::Validation,
                ProviderOperation::Health,
                "unresolved",
                None,
                None,
                false,
                None,
                "provider runtime bounds must be greater than zero",
            ));
        }
        Ok(())
    }
}
