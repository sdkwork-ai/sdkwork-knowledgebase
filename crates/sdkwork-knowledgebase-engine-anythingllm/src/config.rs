//! AnythingLLM connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const ANYTHINGLLM_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ANYTHINGLLM_BASE_URL";
pub const ANYTHINGLLM_ALLOW_PRIVATE_NETWORK_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_ANYTHINGLLM_ALLOW_PRIVATE_NETWORK";
pub const ANYTHINGLLM_WORKSPACE_SLUG_ENV: &str = "SDKWORK_KNOWLEDGEBASE_ANYTHINGLLM_WORKSPACE_SLUG";

#[derive(Clone, PartialEq, Eq)]
pub struct AnythingLlmConnectorConfig {
    pub base_url: String,
    pub api_key: Zeroizing<String>,
    pub default_workspace_slug: Option<String>,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `ANYTHINGLLM_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl AnythingLlmConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(ANYTHINGLLM_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = Zeroizing::new(String::new());
        let default_workspace_slug = std::env::var(ANYTHINGLLM_WORKSPACE_SLUG_ENV)
            .ok()
            .filter(|value| !value.is_empty());

        let allow_private_network = env_flag(ANYTHINGLLM_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            default_workspace_slug,
            allow_private_network,
        })
    }
}
