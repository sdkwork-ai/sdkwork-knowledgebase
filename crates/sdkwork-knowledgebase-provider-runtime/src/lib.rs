//! Shared outbound runtime for external Knowledgebase providers.

mod error;
mod path_segment;
mod policy;
mod runtime;
mod target_security;
mod telemetry;

pub use error::{engine_provider_error, ProviderError};
pub use path_segment::{encoded_path_segment, is_path_segment_id, MAX_PATH_SEGMENT_ID_LEN};
pub use policy::{ProviderOrigin, ProviderRuntimeConfig, ProviderTargetPolicy};
pub use runtime::{
    ProviderExecutionContext, ProviderHttpRequest, ProviderHttpResponse, ProviderRuntime,
};
pub use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineProviderErrorCategory as ProviderErrorCategory,
    KnowledgeEngineProviderOperation as ProviderOperation,
};

/// Parses an explicit boolean environment flag for engine configuration.
///
/// Recognized truthy values are `1`, `true`, `yes`, and `on` (case-insensitive);
/// an unset variable or any other value is `false` so configuration flags fail
/// closed. Used for opt-in flags such as `<ENGINE>_ALLOW_PRIVATE_NETWORK`.
pub fn env_flag(name: &str) -> bool {
    parse_bool_env_value(std::env::var(name).ok().as_deref())
}

/// Pure parser behind [`env_flag`]: `Some("1" | "true" | "yes" | "on")`
/// (case-insensitive) is `true`, anything else — including `None` — is `false`.
pub fn parse_bool_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Logs the degrade decision an adapter must take when its configuration fails
/// the Provider runtime target policy: the engine registers as
/// unconfigured/degraded instead of the process panicking at startup or at
/// Provider binding time.
pub fn log_engine_degraded(
    implementation_id: &str,
    error: &sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineError,
) {
    tracing::error!(
        implementation_id,
        error = %error,
        "provider adapter configuration failed; engine registered as unconfigured/degraded"
    );
}

/// Normalizes an adapter credential into the `optional_bearer_auth` input: a
/// blank secret (the pre-binding default for required-credential adapters) must
/// not become an empty `Authorization: Bearer` header.
pub fn optional_bearer_token(token: Option<&str>) -> Option<&str> {
    token.filter(|token| !sdkwork_utils_rust::is_blank(Some(token)))
}
pub use telemetry::{
    install_provider_telemetry, NoopProviderTelemetry, ProviderTelemetry, ProviderTelemetryEvent,
};

#[cfg(test)]
mod tests {
    use super::{env_flag, parse_bool_env_value};

    const TEST_FLAG_ENV: &str = "SDKWORK_KNOWLEDGEBASE_PROVIDER_RUNTIME_TEST_FLAG";

    #[test]
    fn bool_env_values_parse_fail_closed() {
        for truthy in ["1", "true", "YES", "On", " on "] {
            assert!(
                parse_bool_env_value(Some(truthy)),
                "{truthy:?} must parse as true"
            );
        }
        for falsy in [None, Some(""), Some("0"), Some("false"), Some("enabled")] {
            assert!(
                !parse_bool_env_value(falsy),
                "{falsy:?} must parse as false"
            );
        }
    }

    #[test]
    fn env_flag_reads_the_named_variable_fail_closed() {
        std::env::remove_var(TEST_FLAG_ENV);
        assert!(!env_flag(TEST_FLAG_ENV));
        std::env::set_var(TEST_FLAG_ENV, "1");
        assert!(env_flag(TEST_FLAG_ENV));
        std::env::remove_var(TEST_FLAG_ENV);
        assert!(!env_flag(TEST_FLAG_ENV));
    }
}
