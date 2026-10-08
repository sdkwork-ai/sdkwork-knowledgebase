//! Weaviate connector configuration from runtime environment.

use sdkwork_knowledgebase_provider_runtime::env_flag;
use zeroize::Zeroizing;

pub const WEAVIATE_BASE_URL_ENV: &str = "SDKWORK_KNOWLEDGEBASE_WEAVIATE_BASE_URL";
pub const WEAVIATE_ALLOW_PRIVATE_NETWORK_ENV: &str =
    "SDKWORK_KNOWLEDGEBASE_WEAVIATE_ALLOW_PRIVATE_NETWORK";
pub const WEAVIATE_CLASS_NAME_ENV: &str = "SDKWORK_KNOWLEDGEBASE_WEAVIATE_CLASS_NAME";
pub const WEAVIATE_TITLE_PROPERTY_ENV: &str = "SDKWORK_KNOWLEDGEBASE_WEAVIATE_TITLE_PROPERTY";
pub const WEAVIATE_CONTENT_PROPERTY_ENV: &str = "SDKWORK_KNOWLEDGEBASE_WEAVIATE_CONTENT_PROPERTY";

pub const DEFAULT_WEAVIATE_TITLE_PROPERTY: &str = "title";
pub const DEFAULT_WEAVIATE_CONTENT_PROPERTY: &str = "content";

#[derive(Clone, PartialEq, Eq)]
pub struct WeaviateConnectorConfig {
    pub base_url: String,
    pub api_key: Option<Zeroizing<String>>,
    pub default_class_name: Option<String>,
    pub title_property: String,
    pub content_property: String,
    /// Fail-closed opt-in for self-hosted engine targets on a private network
    /// segment (loopback, RFC1918): set `WEAVIATE_ALLOW_PRIVATE_NETWORK=1|true`
    /// when the deployment explicitly trusts the private segment the engine
    /// runs on (operator responsibility). DNS socket pinning still applies.
    pub allow_private_network: bool,
}

impl WeaviateConnectorConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var(WEAVIATE_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())?;
        let api_key = None;
        let default_class_name = std::env::var(WEAVIATE_CLASS_NAME_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        let title_property = std::env::var(WEAVIATE_TITLE_PROPERTY_ENV)
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_WEAVIATE_TITLE_PROPERTY.to_string());
        let content_property = std::env::var(WEAVIATE_CONTENT_PROPERTY_ENV)
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_WEAVIATE_CONTENT_PROPERTY.to_string());

        let allow_private_network = env_flag(WEAVIATE_ALLOW_PRIVATE_NETWORK_ENV);

        Some(Self {
            base_url,
            api_key,
            default_class_name,
            title_property,
            content_property,
            allow_private_network,
        })
    }
}
