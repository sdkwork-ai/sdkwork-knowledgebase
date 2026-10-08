//! Chroma connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const CHROMA_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_CHROMA_BASE_URL";
pub const CHROMA_ALLOW_PRIVATE_NETWORK_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_CHROMA_ALLOW_PRIVATE_NETWORK";
pub const CHROMA_COLLECTION_ID_ENV: &str = "SDKWORK_KNOWLEDGEBASE_CHROMA_COLLECTION_ID";
pub const CHROMA_TENANT_ENV: &str = "SDKWORK_KNOWLEDGEBASE_CHROMA_TENANT";
pub const CHROMA_DATABASE_ENV: &str = "SDKWORK_KNOWLEDGEBASE_CHROMA_DATABASE";

pub const DEFAULT_CHROMA_TENANT: &str = "default_tenant";
pub const DEFAULT_CHROMA_DATABASE: &str = "default_database";

#[derive(Clone, PartialEq, Eq)]
pub struct ChromaConnectorConfig {
    pub base_url: String,
    pub api_key: Option<Zeroizing<String>>,
    pub default_collection_id: Option<String>,
    pub tenant: String,
    pub database: String,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `CHROMA_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl ChromaConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(CHROMA_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = None;
        let default_collection_id = std::env::var(CHROMA_COLLECTION_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        let tenant = std::env::var(CHROMA_TENANT_ENV)
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_CHROMA_TENANT.to_string());
        let database = std::env::var(CHROMA_DATABASE_ENV)
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_CHROMA_DATABASE.to_string());

        let allow_private_network = env_flag(CHROMA_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            default_collection_id,
            tenant,
            database,
            allow_private_network,
        })
    }
}
