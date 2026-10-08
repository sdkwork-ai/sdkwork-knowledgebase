//! Qdrant connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const QDRANT_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_QDRANT_BASE_URL";
pub const QDRANT_ALLOW_PRIVATE_NETWORK_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_QDRANT_ALLOW_PRIVATE_NETWORK";
pub const QDRANT_COLLECTION_NAME_ENV: &str = "SDKWORK_KNOWLEDGEBASE_QDRANT_COLLECTION_NAME";
pub const QDRANT_QUERY_MODEL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_QDRANT_QUERY_MODEL";
pub const QDRANT_USING_VECTOR_ENV: &str = "SDKWORK_KNOWLEDGEBASE_QDRANT_USING_VECTOR";

#[derive(Clone, PartialEq, Eq)]
pub struct QdrantConnectorConfig {
    pub base_url: String,
    pub api_key: Option<Zeroizing<String>>,
    pub default_collection_name: Option<String>,
    pub query_model: Option<String>,
    pub using_vector: Option<String>,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `QDRANT_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl QdrantConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(QDRANT_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = None;
        let default_collection_name = std::env::var(QDRANT_COLLECTION_NAME_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        let query_model = std::env::var(QDRANT_QUERY_MODEL_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        let using_vector = std::env::var(QDRANT_USING_VECTOR_ENV)
            .ok()
            .filter(|value| !value.is_empty());

        let allow_private_network = env_flag(QDRANT_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            default_collection_name,
            query_model,
            using_vector,
            allow_private_network,
        })
    }
}
