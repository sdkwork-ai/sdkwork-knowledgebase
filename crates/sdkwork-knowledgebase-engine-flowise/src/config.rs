//! Flowise connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const FLOWISE_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_FLOWISE_BASE_URL";
pub const FLOWISE_ALLOW_PRIVATE_NETWORK_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_FLOWISE_ALLOW_PRIVATE_NETWORK";
pub const FLOWISE_STORE_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_FLOWISE_STORE_ID";

#[derive(Clone, PartialEq, Eq)]
pub struct FlowiseConnectorConfig {
    pub base_url: String,
    pub api_key: Zeroizing<String>,
    pub default_store_id: Option<String>,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `FLOWISE_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl FlowiseConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(FLOWISE_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = Zeroizing::new(String::new());
        let default_store_id = std::env::var(FLOWISE_STORE_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty());

        let allow_private_network = env_flag(FLOWISE_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            default_store_id,
            allow_private_network,
        })
    }
}
