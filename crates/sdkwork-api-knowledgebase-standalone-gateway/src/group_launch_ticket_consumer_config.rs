use std::{collections::BTreeMap, time::Duration};

use sdkwork_knowledgebase_im_rpc_adapter::{
    KnowledgebaseImGroupLaunchTicketConsumer, KnowledgebaseImGroupLaunchTicketConsumerConfig,
    KnowledgebaseImRpcAdapterError,
};
use sdkwork_rpc_framework_core::RpcCallerContextSigningKey;
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

const ENVIRONMENT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ENVIRONMENT";
const IM_RPC_ENDPOINT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_IM_RPC_ENDPOINT";
const IM_RPC_CA_CERT_PATH_ENV: &str = "SDKWORK_KNOWLEDGEBASE_IM_RPC_CA_CERT_PATH";
const IM_RPC_CLIENT_CERT_PATH_ENV: &str = "SDKWORK_KNOWLEDGEBASE_IM_RPC_CLIENT_CERT_PATH";
const IM_RPC_CLIENT_KEY_PATH_ENV: &str = "SDKWORK_KNOWLEDGEBASE_IM_RPC_CLIENT_KEY_PATH";
const IM_RPC_TLS_DOMAIN_ENV: &str = "SDKWORK_KNOWLEDGEBASE_IM_RPC_TLS_DOMAIN";
const IM_RPC_CALLER_CONTEXT_SIGNING_KEY_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_IM_RPC_CALLER_CONTEXT_SIGNING_KEY";
const IM_RPC_CREDENTIAL_TTL_SECONDS_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_IM_RPC_CREDENTIAL_TTL_SECONDS";
const IM_RPC_TIMEOUT_MS_ENV: &str = "SDKWORK_KNOWLEDGEBASE_IM_RPC_TIMEOUT_MS";

const REQUIRED_IM_RPC_KEYS: [&str; 9] = [
    IM_RPC_ENDPOINT_ENV,
    IM_RPC_CA_CERT_PATH_ENV,
    IM_RPC_CLIENT_CERT_PATH_ENV,
    IM_RPC_CLIENT_KEY_PATH_ENV,
    IM_RPC_TLS_DOMAIN_ENV,
    IM_RPC_CALLER_CONTEXT_SIGNING_KEY_ENV,
    IM_RPC_CREDENTIAL_TTL_SECONDS_ENV,
    IM_RPC_TIMEOUT_MS_ENV,
    ENVIRONMENT_ENV,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeploymentEnvironment {
    Development,
    Test,
    Staging,
    Production,
}

impl DeploymentEnvironment {
    fn parse(value: &str) -> Result<Self, GroupLaunchTicketConsumerConfigError> {
        match value {
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            _ => Err(GroupLaunchTicketConsumerConfigError::InvalidEnvironment(
                value.to_string(),
            )),
        }
    }

    fn requires_ticket_consumer(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }
}

pub async fn resolve_group_launch_ticket_consumer_from_env(
) -> Result<Option<KnowledgebaseImGroupLaunchTicketConsumer>, GroupLaunchTicketConsumerConfigError>
{
    let values = read_environment_values()?;
    let Some(config) = build_group_launch_ticket_consumer_config(&values)? else {
        return Ok(None);
    };
    KnowledgebaseImGroupLaunchTicketConsumer::connect(config)
        .await
        .map(Some)
        .map_err(GroupLaunchTicketConsumerConfigError::Adapter)
}

fn build_group_launch_ticket_consumer_config(
    values: &BTreeMap<&'static str, Option<String>>,
) -> Result<
    Option<KnowledgebaseImGroupLaunchTicketConsumerConfig>,
    GroupLaunchTicketConsumerConfigError,
> {
    let environment = required_value(values, ENVIRONMENT_ENV)?;
    let environment = DeploymentEnvironment::parse(environment)?;
    let configured_key_count = REQUIRED_IM_RPC_KEYS[0..8]
        .iter()
        .filter(|key| {
            values
                .get(**key)
                .and_then(Option::as_deref)
                .is_some_and(|value| !is_blank(Some(value)))
        })
        .count();

    if configured_key_count == 0 && !environment.requires_ticket_consumer() {
        return Ok(None);
    }
    if configured_key_count != 8 {
        return Err(GroupLaunchTicketConsumerConfigError::IncompleteConfiguration);
    }

    let signing_key = RpcCallerContextSigningKey::from_base64url(required_value(
        values,
        IM_RPC_CALLER_CONTEXT_SIGNING_KEY_ENV,
    )?)
    .map_err(|_| GroupLaunchTicketConsumerConfigError::InvalidSigningKey)?;
    let credential_ttl_seconds = parse_positive_u64(
        IM_RPC_CREDENTIAL_TTL_SECONDS_ENV,
        required_value(values, IM_RPC_CREDENTIAL_TTL_SECONDS_ENV)?,
    )?;
    let request_timeout_ms = parse_positive_u64(
        IM_RPC_TIMEOUT_MS_ENV,
        required_value(values, IM_RPC_TIMEOUT_MS_ENV)?,
    )?;

    KnowledgebaseImGroupLaunchTicketConsumerConfig::new(
        required_value(values, IM_RPC_ENDPOINT_ENV)?,
        required_value(values, IM_RPC_CA_CERT_PATH_ENV)?,
        required_value(values, IM_RPC_CLIENT_CERT_PATH_ENV)?,
        required_value(values, IM_RPC_CLIENT_KEY_PATH_ENV)?,
        required_value(values, IM_RPC_TLS_DOMAIN_ENV)?,
        signing_key,
        Duration::from_secs(credential_ttl_seconds),
        Duration::from_millis(request_timeout_ms),
    )
    .map(Some)
    .map_err(GroupLaunchTicketConsumerConfigError::Adapter)
}

fn read_environment_values(
) -> Result<BTreeMap<&'static str, Option<String>>, GroupLaunchTicketConsumerConfigError> {
    REQUIRED_IM_RPC_KEYS
        .iter()
        .map(|key| {
            let value = match std::env::var(key) {
                Ok(value) if !is_blank(Some(&value)) => Some(value),
                Ok(_) | Err(std::env::VarError::NotPresent) => None,
                Err(std::env::VarError::NotUnicode(_)) => {
                    return Err(GroupLaunchTicketConsumerConfigError::NonUnicodeEnvironment(
                        key,
                    ));
                }
            };
            Ok((*key, value))
        })
        .collect()
}

fn required_value<'a>(
    values: &'a BTreeMap<&'static str, Option<String>>,
    key: &'static str,
) -> Result<&'a str, GroupLaunchTicketConsumerConfigError> {
    values
        .get(key)
        .and_then(Option::as_deref)
        .filter(|value| !is_blank(Some(value)))
        .ok_or(GroupLaunchTicketConsumerConfigError::MissingEnvironment(
            key,
        ))
}

fn parse_positive_u64(
    key: &'static str,
    value: &str,
) -> Result<u64, GroupLaunchTicketConsumerConfigError> {
    let parsed = value.parse::<u64>().map_err(|_| {
        GroupLaunchTicketConsumerConfigError::InvalidUnsignedInteger {
            key,
            value: value.to_string(),
        }
    })?;
    if parsed == 0 {
        return Err(
            GroupLaunchTicketConsumerConfigError::InvalidUnsignedInteger {
                key,
                value: value.to_string(),
            },
        );
    }
    Ok(parsed)
}

#[derive(Debug, Error)]
pub enum GroupLaunchTicketConsumerConfigError {
    #[error("{0} must be set to development, test, staging, or production")]
    InvalidEnvironment(String),
    #[error("required environment variable {0} is missing")]
    MissingEnvironment(&'static str),
    #[error("environment variable {0} is not valid Unicode")]
    NonUnicodeEnvironment(&'static str),
    #[error(
        "IM RPC configuration must either be fully configured or fully absent in development/test"
    )]
    IncompleteConfiguration,
    #[error("IM RPC caller-context signing key must be an unpadded base64url 32-byte key")]
    InvalidSigningKey,
    #[error("{key} must be a positive unsigned integer, got {value:?}")]
    InvalidUnsignedInteger { key: &'static str, value: String },
    #[error(transparent)]
    Adapter(#[from] KnowledgebaseImRpcAdapterError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(environment: &str) -> BTreeMap<&'static str, Option<String>> {
        let mut values = BTreeMap::new();
        for key in REQUIRED_IM_RPC_KEYS {
            values.insert(key, None);
        }
        values.insert(ENVIRONMENT_ENV, Some(environment.to_string()));
        values
    }

    #[test]
    fn development_allows_an_explicitly_absent_ticket_consumer() {
        assert!(
            build_group_launch_ticket_consumer_config(&values("development"))
                .expect("development config")
                .is_none()
        );
    }

    #[test]
    fn production_requires_the_ticket_consumer_configuration() {
        assert!(matches!(
            build_group_launch_ticket_consumer_config(&values("production")),
            Err(GroupLaunchTicketConsumerConfigError::IncompleteConfiguration)
        ));
    }

    #[test]
    fn partial_development_configuration_fails_closed() {
        let mut values = values("development");
        values.insert(
            IM_RPC_ENDPOINT_ENV,
            Some("grpcs://im.internal:7443".to_string()),
        );
        assert!(matches!(
            build_group_launch_ticket_consumer_config(&values),
            Err(GroupLaunchTicketConsumerConfigError::IncompleteConfiguration)
        ));
    }
}
