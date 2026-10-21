use std::time::Duration;

const DRAIN_TIMEOUT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_GATEWAY_DRAIN_TIMEOUT_SECS";
const MAX_CONNECTIONS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_GATEWAY_MAX_CONNECTIONS";
const HEADER_READ_TIMEOUT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_GATEWAY_HEADER_READ_TIMEOUT_SECS";
const ENVIRONMENT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ENVIRONMENT";
const DEFAULT_DRAIN_TIMEOUT_SECONDS: u64 = 30;
const PRODUCTION_MINIMUM_DRAIN_TIMEOUT_SECONDS: u64 = 5;
const PRODUCTION_MAXIMUM_DRAIN_TIMEOUT_SECONDS: u64 = 300;
pub(crate) const DEFAULT_MAX_CONNECTIONS: usize = 4_096;
pub(crate) const DEFAULT_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
const MINIMUM_MAX_CONNECTIONS: usize = 1;
const MAXIMUM_MAX_CONNECTIONS: usize = 16_384;
const MINIMUM_HEADER_READ_TIMEOUT_SECONDS: u64 = 1;
const MAXIMUM_HEADER_READ_TIMEOUT_SECONDS: u64 = 30;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GatewayConfigError {
    MissingEnvironment,
    NonUnicodeEnvironmentVariable {
        key: &'static str,
    },
    InvalidDrainTimeoutSeconds {
        value: String,
    },
    InvalidEnvironment {
        value: String,
    },
    DrainTimeoutMustBePositive,
    ProductionDrainTimeoutOutOfRange {
        seconds: u64,
        minimum_seconds: u64,
        maximum_seconds: u64,
    },
    DrainTimeoutExceedsMaximum {
        seconds: u64,
        maximum_seconds: u64,
    },
    InvalidMaxConnections {
        value: String,
    },
    MaxConnectionsOutOfRange {
        connections: usize,
        minimum_connections: usize,
        maximum_connections: usize,
    },
    InvalidHeaderReadTimeoutSeconds {
        value: String,
    },
    HeaderReadTimeoutOutOfRange {
        seconds: u64,
        minimum_seconds: u64,
        maximum_seconds: u64,
    },
}

impl std::fmt::Display for GatewayConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnvironment => {
                write!(formatter, "{ENVIRONMENT_ENV} must be set explicitly")
            }
            Self::NonUnicodeEnvironmentVariable { key } => {
                write!(formatter, "environment variable {key} is not valid Unicode")
            }
            Self::InvalidDrainTimeoutSeconds { value } => write!(
                formatter,
                "{DRAIN_TIMEOUT_ENV} must be an unsigned integer number of seconds, got {value:?}"
            ),
            Self::InvalidEnvironment { value } => write!(
                formatter,
                "{ENVIRONMENT_ENV} must be one of development, test, staging, or production, got {value:?}"
            ),
            Self::DrainTimeoutMustBePositive => {
                write!(formatter, "{DRAIN_TIMEOUT_ENV} must be greater than zero")
            }
            Self::ProductionDrainTimeoutOutOfRange {
                seconds,
                minimum_seconds,
                maximum_seconds,
            } => write!(
                formatter,
                "{DRAIN_TIMEOUT_ENV} must be between {minimum_seconds} and {maximum_seconds} seconds in production, got {seconds}"
            ),
            Self::DrainTimeoutExceedsMaximum {
                seconds,
                maximum_seconds,
            } => write!(
                formatter,
                "{DRAIN_TIMEOUT_ENV} must not exceed {maximum_seconds} seconds, got {seconds}"
            ),
            Self::InvalidMaxConnections { value } => write!(
                formatter,
                "{MAX_CONNECTIONS_ENV} must be an unsigned integer, got {value:?}"
            ),
            Self::MaxConnectionsOutOfRange {
                connections,
                minimum_connections,
                maximum_connections,
            } => write!(
                formatter,
                "{MAX_CONNECTIONS_ENV} must be between {minimum_connections} and {maximum_connections}, got {connections}"
            ),
            Self::InvalidHeaderReadTimeoutSeconds { value } => write!(
                formatter,
                "{HEADER_READ_TIMEOUT_ENV} must be an unsigned integer number of seconds, got {value:?}"
            ),
            Self::HeaderReadTimeoutOutOfRange {
                seconds,
                minimum_seconds,
                maximum_seconds,
            } => write!(
                formatter,
                "{HEADER_READ_TIMEOUT_ENV} must be between {minimum_seconds} and {maximum_seconds} seconds, got {seconds}"
            ),
        }
    }
}

impl std::error::Error for GatewayConfigError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GatewayServerConfig {
    pub(crate) drain_timeout: Duration,
    pub(crate) max_connections: usize,
    pub(crate) header_read_timeout: Duration,
}

impl GatewayServerConfig {
    pub(crate) fn from_env() -> Result<Self, GatewayConfigError> {
        let drain_timeout = read_optional_env(DRAIN_TIMEOUT_ENV)?;
        let max_connections = read_optional_env(MAX_CONNECTIONS_ENV)?;
        let header_read_timeout = read_optional_env(HEADER_READ_TIMEOUT_ENV)?;
        let environment = read_optional_env(ENVIRONMENT_ENV)?;
        Ok(Self {
            drain_timeout: resolve_gateway_drain_timeout(
                drain_timeout.as_deref(),
                environment.as_deref(),
            )?,
            max_connections: resolve_gateway_max_connections(max_connections.as_deref())?,
            header_read_timeout: resolve_gateway_header_read_timeout(
                header_read_timeout.as_deref(),
            )?,
        })
    }
}

fn read_optional_env(key: &'static str) -> Result<Option<String>, GatewayConfigError> {
    match std::env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(GatewayConfigError::NonUnicodeEnvironmentVariable { key })
        }
    }
}

pub(crate) fn resolve_gateway_drain_timeout(
    value: Option<&str>,
    environment: Option<&str>,
) -> Result<Duration, GatewayConfigError> {
    let production = match environment {
        None => return Err(GatewayConfigError::MissingEnvironment),
        Some("development" | "test" | "staging") => false,
        Some("production") => true,
        Some(value) => {
            return Err(GatewayConfigError::InvalidEnvironment {
                value: value.to_string(),
            })
        }
    };
    let seconds =
        match value {
            Some(value) => value.parse::<u64>().map_err(|_| {
                GatewayConfigError::InvalidDrainTimeoutSeconds {
                    value: value.to_string(),
                }
            })?,
            None => DEFAULT_DRAIN_TIMEOUT_SECONDS,
        };

    if seconds == 0 {
        return Err(GatewayConfigError::DrainTimeoutMustBePositive);
    }

    if production
        && !(PRODUCTION_MINIMUM_DRAIN_TIMEOUT_SECONDS..=PRODUCTION_MAXIMUM_DRAIN_TIMEOUT_SECONDS)
            .contains(&seconds)
    {
        return Err(GatewayConfigError::ProductionDrainTimeoutOutOfRange {
            seconds,
            minimum_seconds: PRODUCTION_MINIMUM_DRAIN_TIMEOUT_SECONDS,
            maximum_seconds: PRODUCTION_MAXIMUM_DRAIN_TIMEOUT_SECONDS,
        });
    }
    if seconds > PRODUCTION_MAXIMUM_DRAIN_TIMEOUT_SECONDS {
        return Err(GatewayConfigError::DrainTimeoutExceedsMaximum {
            seconds,
            maximum_seconds: PRODUCTION_MAXIMUM_DRAIN_TIMEOUT_SECONDS,
        });
    }

    Ok(Duration::from_secs(seconds))
}

pub(crate) fn resolve_gateway_max_connections(
    value: Option<&str>,
) -> Result<usize, GatewayConfigError> {
    let connections = match value {
        Some(value) => {
            value
                .parse::<usize>()
                .map_err(|_| GatewayConfigError::InvalidMaxConnections {
                    value: value.to_string(),
                })?
        }
        None => DEFAULT_MAX_CONNECTIONS,
    };
    if !(MINIMUM_MAX_CONNECTIONS..=MAXIMUM_MAX_CONNECTIONS).contains(&connections) {
        return Err(GatewayConfigError::MaxConnectionsOutOfRange {
            connections,
            minimum_connections: MINIMUM_MAX_CONNECTIONS,
            maximum_connections: MAXIMUM_MAX_CONNECTIONS,
        });
    }
    Ok(connections)
}

pub(crate) fn resolve_gateway_header_read_timeout(
    value: Option<&str>,
) -> Result<Duration, GatewayConfigError> {
    let seconds = match value {
        Some(value) => value.parse::<u64>().map_err(|_| {
            GatewayConfigError::InvalidHeaderReadTimeoutSeconds {
                value: value.to_string(),
            }
        })?,
        None => DEFAULT_HEADER_READ_TIMEOUT.as_secs(),
    };
    if !(MINIMUM_HEADER_READ_TIMEOUT_SECONDS..=MAXIMUM_HEADER_READ_TIMEOUT_SECONDS)
        .contains(&seconds)
    {
        return Err(GatewayConfigError::HeaderReadTimeoutOutOfRange {
            seconds,
            minimum_seconds: MINIMUM_HEADER_READ_TIMEOUT_SECONDS,
            maximum_seconds: MAXIMUM_HEADER_READ_TIMEOUT_SECONDS,
        });
    }
    Ok(Duration::from_secs(seconds))
}
