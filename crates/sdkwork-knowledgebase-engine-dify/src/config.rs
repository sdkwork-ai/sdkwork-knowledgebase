//! Dify connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const DIFY_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DIFY_BASE_URL";
pub const DIFY_ALLOW_PRIVATE_NETWORK_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DIFY_ALLOW_PRIVATE_NETWORK";
pub const DIFY_DATASET_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_DIFY_DATASET_ID";

#[derive(Clone, PartialEq, Eq)]
pub struct DifyConnectorConfig {
    pub base_url: String,
    pub api_key: Zeroizing<String>,
    pub default_dataset_id: Option<String>,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `DIFY_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl DifyConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(DIFY_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = Zeroizing::new(String::new());
        let default_dataset_id = std::env::var(DIFY_DATASET_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty());

        let allow_private_network = env_flag(DIFY_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            default_dataset_id,
            allow_private_network,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DifyConnectorConfig, DIFY_ALLOW_PRIVATE_NETWORK_ENV, DIFY_BASE_URL_ENV};

    /// Representative parse test for the shared `<ENGINE>_ALLOW_PRIVATE_NETWORK`
    /// template: the nine other engine crates repeat this identical one-line
    /// `env_flag(...)` parse behind their own prefixed env constant.
    #[test]
    fn allow_private_network_flag_defaults_to_fail_closed_and_parses_explicit_opt_in() {
        std::env::set_var(DIFY_BASE_URL_ENV, "http://127.0.0.1:1");
        std::env::remove_var(DIFY_ALLOW_PRIVATE_NETWORK_ENV);
        let config = DifyConnectorConfig::from_env().expect("config from base url");
        assert!(!config.allow_private_network);

        for truthy in ["1", "true", "YES", "on"] {
            std::env::set_var(DIFY_ALLOW_PRIVATE_NETWORK_ENV, truthy);
            let config = DifyConnectorConfig::from_env().expect("config from base url");
            assert!(config.allow_private_network, "value {truthy} must opt in");
        }

        for falsy in ["false", "0", "", "sometimes"] {
            std::env::set_var(DIFY_ALLOW_PRIVATE_NETWORK_ENV, falsy);
            let config = DifyConnectorConfig::from_env().expect("config from base url");
            assert!(
                !config.allow_private_network,
                "value {falsy:?} must stay fail closed"
            );
        }

        std::env::remove_var(DIFY_BASE_URL_ENV);
        std::env::remove_var(DIFY_ALLOW_PRIVATE_NETWORK_ENV);
    }
}
