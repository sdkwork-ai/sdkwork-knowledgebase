use std::{fs::File, io::Read, net::SocketAddr, path::PathBuf, str::FromStr, time::Duration};

use sdkwork_rpc_framework_core::{
    RpcCallerContextSigningKey, RpcCallerContextVerifier, RpcFrameworkError,
    RpcServiceIdentityPolicy,
};
use sdkwork_rpc_server::{RpcInternalServiceSecurity, RpcServerTlsConfig};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

const ENVIRONMENT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ENVIRONMENT";
const RPC_ENABLED_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_ENABLED";
const RPC_BIND_ADDR_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_BIND_ADDR";
const RPC_TLS_ENABLED_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_TLS_ENABLED";
const RPC_MTLS_ENABLED_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_MTLS_ENABLED";
const RPC_HEALTH_ENABLED_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_HEALTH_ENABLED";
const RPC_SERVER_CERT_PATH_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_SERVER_CERT_PATH";
const RPC_SERVER_KEY_PATH_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_SERVER_KEY_PATH";
const RPC_CLIENT_CA_CERTIFICATE_PATH_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_CLIENT_CA_CERTIFICATE_PATH";
const RPC_SPIFFE_TRUST_DOMAIN_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_SPIFFE_TRUST_DOMAIN";
const RPC_IM_CALLER_CONTEXT_SIGNING_KEY_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_IM_CALLER_CONTEXT_SIGNING_KEY";
const RPC_IM_CALLER_CONTEXT_SIGNING_KEY_FILE_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_IM_CALLER_CONTEXT_SIGNING_KEY_FILE";
const RPC_MAX_DECODING_MESSAGE_BYTES_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_MAX_DECODING_MESSAGE_BYTES";
const RPC_MAX_ENCODING_MESSAGE_BYTES_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_MAX_ENCODING_MESSAGE_BYTES";
const RPC_MAX_CONCURRENT_REQUESTS_PER_CONNECTION_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_MAX_CONCURRENT_REQUESTS_PER_CONNECTION";
const RPC_REQUEST_TIMEOUT_MS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_REQUEST_TIMEOUT_MS";
const RPC_HTTP2_KEEPALIVE_INTERVAL_MS_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_HTTP2_KEEPALIVE_INTERVAL_MS";
const RPC_HTTP2_KEEPALIVE_TIMEOUT_MS_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_HTTP2_KEEPALIVE_TIMEOUT_MS";
const RPC_MAX_CONNECTION_AGE_SECONDS_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_MAX_CONNECTION_AGE_SECONDS";
const RPC_MAX_CONNECTION_AGE_GRACE_SECONDS_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_RPC_MAX_CONNECTION_AGE_GRACE_SECONDS";
const RPC_TCP_KEEPALIVE_SECONDS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_TCP_KEEPALIVE_SECONDS";
const RPC_DRAIN_TIMEOUT_SECONDS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_RPC_DRAIN_TIMEOUT_SECONDS";
const DATABASE_URL_ENV: &str = "SDKWORK_DATABASE_URL";
const DATABASE_URL_FILE_ENV: &str = "SDKWORK_DATABASE_URL_FILE";
const DEPLOYMENT_PROFILE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE";
const DRIVE_STORAGE_ROOT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT";
const DRIVE_STORAGE_PROVIDER_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_PROVIDER_ID";
const OPERATOR_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_OPERATOR_ID";
const ACTOR_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ACTOR_ID";

const IM_SERVICE_ID: &str = "sdkwork-im";
const KNOWLEDGEBASE_SERVICE_ID: &str = "sdkwork-knowledgebase";
const DEFAULT_DRIVE_STORAGE_ROOT: &str = "data/drive-objects";
const MAXIMUM_SIGNING_KEY_FILE_BYTES: u64 = 4 * 1024;

/// Bootstrap-owned private configuration for the IM-facing Knowledgebase RPC listener.
///
/// It deliberately does not support a plaintext or unsigned-local fallback. The lifecycle
/// surface changes Drive ACLs and is always authenticated as `sdkwork-im` over strict mTLS.
#[derive(Clone, Debug)]
pub struct GroupKnowledgeSpaceLifecycleRpcHostConfig {
    pub environment: RpcHostEnvironment,
    pub bind_addr: SocketAddr,
    pub tls: RpcServerTlsConfig,
    pub database_url: String,
    pub drive_storage: DriveStorageRuntimeConfig,
    pub operator_id: String,
    pub system_actor_id: u64,
    pub transport: RpcServerTransportConfig,
    caller_context_signing_key: RpcCallerContextSigningKey,
    spiffe_trust_domain: String,
}

impl GroupKnowledgeSpaceLifecycleRpcHostConfig {
    pub fn from_env() -> Result<Self, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
        let environment = RpcHostEnvironment::parse(&required_env(ENVIRONMENT_ENV)?)?;
        require_enabled(RPC_ENABLED_ENV)?;
        require_enabled(RPC_TLS_ENABLED_ENV)?;
        require_enabled(RPC_MTLS_ENABLED_ENV)?;
        require_enabled(RPC_HEALTH_ENABLED_ENV)?;

        let bind_addr = SocketAddr::from_str(&required_env(RPC_BIND_ADDR_ENV)?).map_err(|_| {
            GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                key: RPC_BIND_ADDR_ENV,
            }
        })?;
        let tls = RpcServerTlsConfig {
            server_cert_path: PathBuf::from(required_env(RPC_SERVER_CERT_PATH_ENV)?),
            server_key_path: PathBuf::from(required_env(RPC_SERVER_KEY_PATH_ENV)?),
            client_ca_certificate_path: Some(PathBuf::from(required_env(
                RPC_CLIENT_CA_CERTIFICATE_PATH_ENV,
            )?)),
            client_auth_optional: false,
        };
        let database_url = read_secret_env_or_file(DATABASE_URL_ENV, DATABASE_URL_FILE_ENV)?;
        let drive_storage = resolve_drive_storage_from_env(environment)?;
        let operator_id =
            configured_nonblank_text_or_default(OPERATOR_ID_ENV, KNOWLEDGEBASE_SERVICE_ID)?;
        if operator_id.len() > 256 {
            return Err(
                GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                    key: OPERATOR_ID_ENV,
                },
            );
        }
        let system_actor_id = sdkwork_knowledgebase_contract::parse_canonical_positive_signed_i64(
            &required_env(ACTOR_ID_ENV)?,
        )
        .map_err(
            |_| GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key: ACTOR_ID_ENV },
        )?;
        let spiffe_trust_domain = required_env(RPC_SPIFFE_TRUST_DOMAIN_ENV)?;
        let caller_context_signing_key = RpcCallerContextSigningKey::from_base64url(
            read_secret_env_or_file(
                RPC_IM_CALLER_CONTEXT_SIGNING_KEY_ENV,
                RPC_IM_CALLER_CONTEXT_SIGNING_KEY_FILE_ENV,
            )?
            .as_str(),
        )
        .map_err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::RpcFramework)?;
        let transport = RpcServerTransportConfig::from_env()?;
        let config = Self {
            environment,
            bind_addr,
            tls,
            database_url,
            drive_storage,
            operator_id,
            system_actor_id,
            transport,
            caller_context_signing_key,
            spiffe_trust_domain,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn internal_service_security(
        &self,
    ) -> Result<RpcInternalServiceSecurity, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
        let identity_policy =
            RpcServiceIdentityPolicy::new(self.spiffe_trust_domain.as_str(), [IM_SERVICE_ID])
                .map_err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::RpcFramework)?;
        let caller_context_verifier = RpcCallerContextVerifier::new(
            KNOWLEDGEBASE_SERVICE_ID,
            [(IM_SERVICE_ID, self.caller_context_signing_key.clone())],
        )
        .map_err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::RpcFramework)?;
        Ok(RpcInternalServiceSecurity::new(
            identity_policy,
            Some(caller_context_verifier),
        ))
    }

    fn validate(&self) -> Result<(), GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
        if is_blank(Some(self.database_url.as_str()))
            || contains_control_character(&self.database_url)
        {
            return Err(
                GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                    key: DATABASE_URL_ENV,
                },
            );
        }
        let security = self.internal_service_security()?;
        security
            .validate_mtls_listener(&self.tls)
            .map_err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::RpcServer)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RpcServerTransportConfig {
    pub max_decoding_message_bytes: usize,
    pub max_encoding_message_bytes: usize,
    pub max_concurrent_requests_per_connection: usize,
    pub request_timeout: Duration,
    pub http2_keepalive_interval: Duration,
    pub http2_keepalive_timeout: Duration,
    pub max_connection_age: Duration,
    pub max_connection_age_grace: Duration,
    pub tcp_keepalive: Duration,
    pub drain_timeout: Duration,
}

impl RpcServerTransportConfig {
    fn from_env() -> Result<Self, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
        Ok(Self {
            max_decoding_message_bytes: bounded_usize_env(
                RPC_MAX_DECODING_MESSAGE_BYTES_ENV,
                2 * 1024 * 1024,
                64 * 1024,
                8 * 1024 * 1024,
            )?,
            max_encoding_message_bytes: bounded_usize_env(
                RPC_MAX_ENCODING_MESSAGE_BYTES_ENV,
                1024 * 1024,
                64 * 1024,
                8 * 1024 * 1024,
            )?,
            max_concurrent_requests_per_connection: bounded_usize_env(
                RPC_MAX_CONCURRENT_REQUESTS_PER_CONNECTION_ENV,
                64,
                1,
                1024,
            )?,
            request_timeout: Duration::from_millis(bounded_u64_env(
                RPC_REQUEST_TIMEOUT_MS_ENV,
                30_000,
                100,
                300_000,
            )?),
            http2_keepalive_interval: Duration::from_millis(bounded_u64_env(
                RPC_HTTP2_KEEPALIVE_INTERVAL_MS_ENV,
                30_000,
                1_000,
                300_000,
            )?),
            http2_keepalive_timeout: Duration::from_millis(bounded_u64_env(
                RPC_HTTP2_KEEPALIVE_TIMEOUT_MS_ENV,
                10_000,
                1_000,
                60_000,
            )?),
            max_connection_age: Duration::from_secs(bounded_u64_env(
                RPC_MAX_CONNECTION_AGE_SECONDS_ENV,
                3_600,
                60,
                86_400,
            )?),
            max_connection_age_grace: Duration::from_secs(bounded_u64_env(
                RPC_MAX_CONNECTION_AGE_GRACE_SECONDS_ENV,
                30,
                1,
                300,
            )?),
            tcp_keepalive: Duration::from_secs(bounded_u64_env(
                RPC_TCP_KEEPALIVE_SECONDS_ENV,
                60,
                10,
                600,
            )?),
            drain_timeout: Duration::from_secs(bounded_u64_env(
                RPC_DRAIN_TIMEOUT_SECONDS_ENV,
                30,
                1,
                300,
            )?),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriveStorageRuntimeConfig {
    StandaloneLocal(PathBuf),
    CloudProvider(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RpcHostEnvironment {
    Development,
    Test,
    Staging,
    Production,
    Demo,
}

impl RpcHostEnvironment {
    fn parse(value: &str) -> Result<Self, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
        match value {
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            "demo" => Ok(Self::Demo),
            _ => Err(
                GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                    key: ENVIRONMENT_ENV,
                },
            ),
        }
    }

    fn requires_explicit_persistent_storage(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }
}

fn required_env(
    key: &'static str,
) -> Result<String, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let value = std::env::var(key)
        .map_err(|_| GroupKnowledgeSpaceLifecycleRpcHostConfigError::Missing { key })?;
    if is_blank(Some(value.as_str())) || contains_control_character(&value) {
        return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key });
    }
    Ok(value)
}

fn configured_nonblank_text_or_default(
    key: &'static str,
    default_value: &'static str,
) -> Result<String, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let value = optional_env_text(key)?.unwrap_or_else(|| default_value.to_string());
    validate_configured_value(key, value)
}

fn resolve_drive_storage_from_env(
    environment: RpcHostEnvironment,
) -> Result<DriveStorageRuntimeConfig, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let profile = match optional_env_text(DEPLOYMENT_PROFILE_ENV)? {
        Some(profile) => profile,
        None if matches!(
            environment,
            RpcHostEnvironment::Development
                | RpcHostEnvironment::Test
                | RpcHostEnvironment::Demo
        ) =>
        {
            "standalone".to_string()
        }
        None => {
            return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::Missing {
                key: DEPLOYMENT_PROFILE_ENV,
            });
        }
    };
    match profile.trim().to_ascii_lowercase().as_str() {
        "standalone" => {
            resolve_drive_storage_root(environment, optional_env_text(DRIVE_STORAGE_ROOT_ENV)?)
                .map(DriveStorageRuntimeConfig::StandaloneLocal)
        }
        "cloud" => {
            if optional_env_text(DRIVE_STORAGE_ROOT_ENV)?.is_some() {
                return Err(
                    GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                        key: DRIVE_STORAGE_ROOT_ENV,
                    },
                );
            }
            let provider_id = required_env(DRIVE_STORAGE_PROVIDER_ID_ENV)?;
            if provider_id.len() > 255 {
                return Err(
                    GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                        key: DRIVE_STORAGE_PROVIDER_ID_ENV,
                    },
                );
            }
            Ok(DriveStorageRuntimeConfig::CloudProvider(provider_id))
        }
        _ => Err(
            GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                key: DEPLOYMENT_PROFILE_ENV,
            },
        ),
    }
}

fn resolve_drive_storage_root(
    environment: RpcHostEnvironment,
    configured_value: Option<String>,
) -> Result<PathBuf, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let value = match configured_value {
        Some(value) => validate_configured_value(DRIVE_STORAGE_ROOT_ENV, value)?,
        None if !environment.requires_explicit_persistent_storage() => {
            DEFAULT_DRIVE_STORAGE_ROOT.to_string()
        }
        None => {
            return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::Missing {
                key: DRIVE_STORAGE_ROOT_ENV,
            });
        }
    };
    let path = PathBuf::from(value);
    if environment.requires_explicit_persistent_storage() && !path.is_absolute() {
        return Err(
            GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                key: DRIVE_STORAGE_ROOT_ENV,
            },
        );
    }
    Ok(path)
}

fn optional_env_text(
    key: &'static str,
) -> Result<Option<String>, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    std::env::var_os(key)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key })
        })
        .transpose()
}

fn bounded_u64_env(
    key: &'static str,
    default_value: u64,
    minimum: u64,
    maximum: u64,
) -> Result<u64, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let value = match optional_env_text(key)? {
        Some(value) => value
            .parse::<u64>()
            .map_err(|_| GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key })?,
        None => default_value,
    };
    if !(minimum..=maximum).contains(&value) {
        return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key });
    }
    Ok(value)
}

fn bounded_usize_env(
    key: &'static str,
    default_value: usize,
    minimum: usize,
    maximum: usize,
) -> Result<usize, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let value = bounded_u64_env(key, default_value as u64, minimum as u64, maximum as u64)?;
    usize::try_from(value)
        .map_err(|_| GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key })
}

fn validate_configured_value(
    key: &'static str,
    value: String,
) -> Result<String, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    if is_blank(Some(value.as_str())) || contains_control_character(&value) {
        return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key });
    }
    Ok(value.trim().to_string())
}

fn require_enabled(
    key: &'static str,
) -> Result<(), GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let value = required_env(key)?;
    match value.as_str() {
        "true" | "1" => Ok(()),
        "false" | "0" => Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::Disabled { key }),
        _ => Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key }),
    }
}

fn read_secret_env_or_file(
    value_key: &'static str,
    file_key: &'static str,
) -> Result<String, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let direct = std::env::var(value_key)
        .ok()
        .filter(|value| !is_blank(Some(value)));
    let file = std::env::var(file_key)
        .ok()
        .filter(|value| !is_blank(Some(value)));
    match (direct, file) {
        (Some(_), Some(_)) => Err(
            GroupKnowledgeSpaceLifecycleRpcHostConfigError::ConflictingSecretSources {
                value_key,
                file_key,
            },
        ),
        (Some(value), None) if value.len() <= MAXIMUM_SIGNING_KEY_FILE_BYTES as usize => Ok(value),
        (Some(_), None) => {
            Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key: value_key })
        }
        (None, Some(path)) => read_bounded_secret_file(file_key, PathBuf::from(path)),
        (None, None) => {
            Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::Missing { key: value_key })
        }
    }
}

fn read_bounded_secret_file(
    key: &'static str,
    path: PathBuf,
) -> Result<String, GroupKnowledgeSpaceLifecycleRpcHostConfigError> {
    let file = File::open(path).map_err(|_| {
        GroupKnowledgeSpaceLifecycleRpcHostConfigError::UnreadableSecretFile { key }
    })?;
    let metadata = file.metadata().map_err(|_| {
        GroupKnowledgeSpaceLifecycleRpcHostConfigError::UnreadableSecretFile { key }
    })?;
    if !metadata.is_file() || metadata.len() > MAXIMUM_SIGNING_KEY_FILE_BYTES {
        return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key });
    }
    let mut value = String::with_capacity(metadata.len() as usize);
    file.take(MAXIMUM_SIGNING_KEY_FILE_BYTES + 1)
        .read_to_string(&mut value)
        .map_err(
            |_| GroupKnowledgeSpaceLifecycleRpcHostConfigError::UnreadableSecretFile { key },
        )?;
    if value.len() > MAXIMUM_SIGNING_KEY_FILE_BYTES as usize {
        return Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key });
    }
    Ok(value.trim_end_matches(['\r', '\n']).to_string())
}

fn contains_control_character(value: &str) -> bool {
    value.chars().any(char::is_control)
}

#[derive(Debug, Error)]
pub enum GroupKnowledgeSpaceLifecycleRpcHostConfigError {
    #[error("required private RPC configuration is missing: {key}")]
    Missing { key: &'static str },
    #[error("private RPC configuration has an invalid value: {key}")]
    InvalidValue { key: &'static str },
    #[error("the internal RPC listener requires enabled configuration: {key}")]
    Disabled { key: &'static str },
    #[error("private RPC value has conflicting direct and file sources")]
    ConflictingSecretSources {
        value_key: &'static str,
        file_key: &'static str,
    },
    #[error("private RPC signing-key file cannot be read: {key}")]
    UnreadableSecretFile { key: &'static str },
    #[error(transparent)]
    RpcFramework(#[from] RpcFrameworkError),
    #[error("internal RPC TLS configuration is invalid: {0}")]
    RpcServer(#[from] sdkwork_rpc_server::ServeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_disabled_and_invalid_server_values_before_bind() {
        assert!(matches!(
            validate_configured_value("TEST_ROOT", "default".to_string()),
            Ok(value) if value == "default"
        ));
        assert!(matches!(
            validate_configured_value("TEST_ROOT", "\n".to_string()),
            Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key: "TEST_ROOT" })
        ));
        assert!(matches!(
            validate_configured_value("TEST_ROOT", "  ".to_string()),
            Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue { key: "TEST_ROOT" })
        ));
        assert!(matches!(
            RpcHostEnvironment::parse("localhost"),
            Err(
                GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                    key: ENVIRONMENT_ENV
                }
            )
        ));
    }

    #[test]
    fn transport_limits_reject_values_outside_the_operational_bounds() {
        assert_eq!(
            bounded_u64_env("SDKWORK_TEST_ABSENT_LIMIT", 64, 1, 128)
                .expect("default within bounds"),
            64
        );
        assert!(matches!(
            bounded_u64_env("SDKWORK_TEST_ABSENT_LIMIT", 256, 1, 128),
            Err(
                GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                    key: "SDKWORK_TEST_ABSENT_LIMIT"
                }
            )
        ));
    }

    #[test]
    fn production_and_staging_require_an_explicit_absolute_drive_storage_root() {
        assert_eq!(
            resolve_drive_storage_root(RpcHostEnvironment::Development, None)
                .expect("development may use the local default"),
            PathBuf::from(DEFAULT_DRIVE_STORAGE_ROOT),
        );
        assert!(matches!(
            resolve_drive_storage_root(RpcHostEnvironment::Production, None),
            Err(GroupKnowledgeSpaceLifecycleRpcHostConfigError::Missing {
                key: DRIVE_STORAGE_ROOT_ENV
            })
        ));
        assert!(matches!(
            resolve_drive_storage_root(
                RpcHostEnvironment::Staging,
                Some("relative/drive-storage".to_string()),
            ),
            Err(
                GroupKnowledgeSpaceLifecycleRpcHostConfigError::InvalidValue {
                    key: DRIVE_STORAGE_ROOT_ENV
                }
            )
        ));
        let production_mount = std::env::temp_dir().join("sdkwork-knowledgebase-rpc-test");
        assert_eq!(
            resolve_drive_storage_root(
                RpcHostEnvironment::Production,
                Some(production_mount.to_string_lossy().into_owned()),
            )
            .expect("production accepts an explicitly configured persistent mount"),
            production_mount,
        );
    }
}
