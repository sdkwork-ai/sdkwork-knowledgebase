//! Onyx connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const ONYX_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ONYX_BASE_URL";
pub const ONYX_ALLOW_PRIVATE_NETWORK_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ONYX_ALLOW_PRIVATE_NETWORK";

#[derive(Clone, PartialEq, Eq)]
pub struct OnyxConnectorConfig {
    pub base_url: String,
    pub api_key: Zeroizing<String>,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `ONYX_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl OnyxConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(ONYX_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = Zeroizing::new(String::new());

        let allow_private_network = env_flag(ONYX_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            allow_private_network,
        })
    }
}
