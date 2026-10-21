//! Redis runtime configuration for the Knowledgebase standalone gateway.
//!
//! Reads the canonical `SDKWORK_KNOWLEDGEBASE_REDIS_*` environment contract so
//! the deployed public ingress wires distributed (multi-replica HA) rate
//! limiting, idempotency, and concurrent-admission stores instead of falling
//! back to per-process memory stores.

use sdkwork_utils_rust::parse_bool;
use url::Url;

pub const REDIS_ENABLED_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_ENABLED";
const REDIS_HOST_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_HOST";
const REDIS_PORT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_PORT";
const REDIS_DATABASE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_DATABASE";
const REDIS_USERNAME_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_USERNAME";
const REDIS_PASSWORD_FILE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_PASSWORD_FILE";
const REDIS_PASSWORD_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_PASSWORD";
const REDIS_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_URL";
const REDIS_KEY_PREFIX_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_KEY_PREFIX";
const REDIS_TLS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_REDIS_TLS";

const DEFAULT_KEY_PREFIX: &str = "sdkwork:knowledgebase";

#[derive(Clone)]
pub struct RedisRuntimeConfig {
    url: String,
    key_prefix: String,
}

/// Redacts userinfo so a `tracing` field or error context can never print the Redis password.
fn redacted_redis_url(value: &str) -> String {
    match Url::parse(value) {
        Ok(mut url) => {
            if !url.username().is_empty() {
                let _ = url.set_username("***");
            }
            if url.password().is_some() {
                let _ = url.set_password(Some("***"));
            }
            url.into()
        }
        // Never echo an unparsable value: it may still contain a password fragment.
        Err(_) => "<invalid redis url>".to_string(),
    }
}

impl std::fmt::Debug for RedisRuntimeConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedisRuntimeConfig")
            .field("url", &redacted_redis_url(&self.url))
            .field("key_prefix", &self.key_prefix)
            .finish()
    }
}

impl RedisRuntimeConfig {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }

    /// Resolves the Redis configuration from the environment.
    ///
    /// Returns `Ok(None)` when Redis is not enabled. Returns an error when the
    /// configuration is invalid or a required secret file cannot be read so
    /// production-like boot fails closed instead of silently degrading.
    pub fn from_env() -> Result<Option<Self>, String> {
        let enabled = match std::env::var(REDIS_ENABLED_ENV).ok() {
            Some(value) => parse_bool(value.trim())
                .ok_or_else(|| format!("{REDIS_ENABLED_ENV} is invalid: {value:?}"))?,
            None => false,
        };
        let explicit_url = std::env::var(REDIS_URL_ENV)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if !enabled && explicit_url.is_none() {
            return Ok(None);
        }

        let key_prefix = std::env::var(REDIS_KEY_PREFIX_ENV)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_KEY_PREFIX.to_owned());
        validate_key_prefix(&key_prefix)?;

        let password = redis_password()?;
        let mut url = match explicit_url {
            Some(value) => parse_redis_url(&value)?,
            None => structured_redis_url()?,
        };
        if let Some(username) = std::env::var(REDIS_USERNAME_ENV)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            url.set_username(&username)
                .map_err(|_| format!("{REDIS_USERNAME_ENV} cannot be encoded in a Redis URL"))?;
        }
        if let Some(password) = password {
            url.set_password(Some(&password))
                .map_err(|_| "Redis password cannot be encoded in a Redis URL".to_owned())?;
        }

        Ok(Some(Self {
            url: url.to_string(),
            key_prefix,
        }))
    }
}

fn parse_redis_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|error| format!("{REDIS_URL_ENV} is invalid: {error}"))?;
    if !matches!(url.scheme(), "redis" | "rediss") {
        return Err(format!(
            "{REDIS_URL_ENV} must use the redis or rediss scheme"
        ));
    }
    if url.host_str().is_none() {
        return Err(format!("{REDIS_URL_ENV} must include a host"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(format!(
            "{REDIS_URL_ENV} must not contain a query string or fragment"
        ));
    }
    Ok(url)
}

fn structured_redis_url() -> Result<Url, String> {
    let host = std::env::var(REDIS_HOST_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_owned());
    let port = std::env::var(REDIS_PORT_ENV)
        .ok()
        .map(|value| {
            value
                .trim()
                .parse::<u16>()
                .map_err(|error| format!("{REDIS_PORT_ENV} is invalid: {error}"))
        })
        .transpose()?
        .unwrap_or(6379);
    let database = std::env::var(REDIS_DATABASE_ENV)
        .ok()
        .map(|value| {
            value
                .trim()
                .parse::<u8>()
                .map_err(|error| format!("{REDIS_DATABASE_ENV} is invalid: {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let tls = match std::env::var(REDIS_TLS_ENV).ok() {
        Some(value) => parse_bool(value.trim())
            .ok_or_else(|| format!("{REDIS_TLS_ENV} is invalid: {value:?}"))?,
        None => false,
    };
    let scheme = if tls { "rediss" } else { "redis" };
    Url::parse(&format!("{scheme}://{host}:{port}/{database}"))
        .map_err(|error| format!("cannot build a Redis URL from {REDIS_HOST_ENV}: {error}"))
}

fn redis_password() -> Result<Option<String>, String> {
    if let Some(file_path) = std::env::var(REDIS_PASSWORD_FILE_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    {
        let password = std::fs::read_to_string(&file_path)
            .map_err(|error| {
                format!("{REDIS_PASSWORD_FILE_ENV} {file_path:?} cannot be read: {error}")
            })?
            .trim()
            .to_owned();
        if password.is_empty() {
            return Err(format!("{REDIS_PASSWORD_FILE_ENV} {file_path:?} is empty"));
        }
        return Ok(Some(password));
    }
    Ok(std::env::var(REDIS_PASSWORD_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty()))
}

fn validate_key_prefix(key_prefix: &str) -> Result<(), String> {
    if key_prefix.is_empty() {
        return Err(format!("{REDIS_KEY_PREFIX_ENV} must not be empty"));
    }
    // ':' is the standard Redis namespace separator (for example
    // "sdkwork:knowledgebase") and is allowed; whitespace and control
    // characters are not valid in key prefixes.
    if key_prefix
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(format!(
            "{REDIS_KEY_PREFIX_ENV} must not contain whitespace or control characters"
        ));
    }
    Ok(())
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
    fn disabled_without_configuration() {
        with_env(&[(REDIS_ENABLED_ENV, "0"), (REDIS_URL_ENV, "")], || {
            let config = RedisRuntimeConfig::from_env().expect("env resolution");
            assert!(config.is_none());
        });
    }

    #[test]
    fn builds_url_from_structured_environment() {
        with_env(
            &[
                (REDIS_ENABLED_ENV, "true"),
                (REDIS_HOST_ENV, "redis.internal"),
                (REDIS_PORT_ENV, "6380"),
                (REDIS_DATABASE_ENV, "2"),
                (REDIS_KEY_PREFIX_ENV, "kb"),
            ],
            || {
                let config = RedisRuntimeConfig::from_env()
                    .expect("env resolution")
                    .expect("enabled");
                assert_eq!(config.url(), "redis://redis.internal:6380/2");
                assert_eq!(config.key_prefix(), "kb");
            },
        );
    }

    #[test]
    fn accepts_explicit_url_and_rejects_query_strings() {
        with_env(&[(REDIS_URL_ENV, "rediss://cache:6379/0")], || {
            let config = RedisRuntimeConfig::from_env()
                .expect("env resolution")
                .expect("enabled");
            assert_eq!(config.url(), "rediss://cache:6379/0");
        });
        with_env(&[(REDIS_URL_ENV, "redis://cache:6379/0?foo=bar")], || {
            assert!(RedisRuntimeConfig::from_env().is_err());
        });
    }

    #[test]
    fn rejects_blank_key_prefix() {
        with_env(
            &[(REDIS_ENABLED_ENV, "1"), (REDIS_KEY_PREFIX_ENV, "a b")],
            || {
                assert!(RedisRuntimeConfig::from_env().is_err());
            },
        );
    }

    #[test]
    fn default_namespaced_key_prefix_is_accepted() {
        with_env(&[(REDIS_ENABLED_ENV, "1")], || {
            let config = RedisRuntimeConfig::from_env()
                .expect("env resolution")
                .expect("enabled");
            assert_eq!(config.key_prefix(), "sdkwork:knowledgebase");
        });
    }
}
