use sdkwork_utils_rust::caller_identity::CallerCredentials;
use sdkwork_utils_rust::is_blank;
use sdkwork_utils_rust::service_base_url::registry_base_url_or;
use std::sync::Arc;

use crate::async_bridge::block_on_async;

use cloudrouter_open_sdk::{
    OpenAiChatCompletionRequest, OpenAiChatMessage, SdkworkAiClient, SdkworkError,
};
use sdkwork_agent_kernel::{
    KernelError, KernelResult, ModelDescriptor, ModelProvider, ModelRequest, ModelResponse,
    ModelResponseFormat, PolicyCategory, ProviderHealth, ProviderManifest,
};

/// Rig registers this provider id; cloud-router SDK is the live LLM backend behind it.
pub const RIG_MODEL_PROVIDER_ID: &str = "provider.model.rig-rust";
pub const RIG_DEFAULT_MODEL_ID: &str = "rig.default-chat";
pub const DEFAULT_CLOUDROUTER_UPSTREAM_MODEL_ID: &str = "openai/gpt-4o-mini";
pub const CLOUDROUTER_OPEN_SDK_CRATE: &str = "cloudrouter_open_sdk";
pub const CLOUDROUTER_CHAT_COMPLETION_METHOD: &str = "chat.create";
pub const CLOUDROUTER_OPEN_HTTP_URL_ENV: &str = "SDKWORK_CLOUDROUTER_APPLICATION_OPEN_HTTP_URL";

#[derive(Clone)]
pub struct CloudRouterChatModelProvider {
    client: Arc<SdkworkAiClient>,
    provider_id: String,
    rig_default_model_id: String,
    upstream_default_model_id: String,
}

impl CloudRouterChatModelProvider {
    pub fn for_rig(client: Arc<SdkworkAiClient>) -> Self {
        Self {
            client,
            provider_id: RIG_MODEL_PROVIDER_ID.to_string(),
            rig_default_model_id: RIG_DEFAULT_MODEL_ID.to_string(),
            upstream_default_model_id: DEFAULT_CLOUDROUTER_UPSTREAM_MODEL_ID.to_string(),
        }
    }
}

pub fn is_rig_model_provider(model_provider_id: &str) -> bool {
    model_provider_id == RIG_MODEL_PROVIDER_ID
        || model_provider_id == "provider.model.sdkwork-cloudrouter-router"
        || model_provider_id == "provider.model.openai"
}

/// The credentials one cloud-router chat call authenticates with.
///
/// APP_SDK_INTEGRATION_SPEC.md §5.2.1: a user turn must carry the caller's
/// verified dual tokens and never the deployment API key; a caller-less
/// background/worker turn uses the declared service identity
/// (SDKWORK_CLOUDROUTER_API_KEY). The two never mix.
fn credential_headers(
    api_key: Option<&str>,
    caller: &CallerCredentials,
) -> Vec<(&'static str, String)> {
    if !caller.is_empty() {
        return caller.wire_headers();
    }
    api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| vec![("X-Api-Key", value.to_owned())])
        .unwrap_or_default()
}

/// Builds the cloud-router client for one caller scope.
pub fn resolve_cloud_router_client(caller: &CallerCredentials) -> Result<SdkworkAiClient, String> {
    let configured = std::env::var(CLOUDROUTER_OPEN_HTTP_URL_ENV).unwrap_or_default();
    // DISCOVERY_SPEC §12.1: registry-first, configured topology as fallback.
    let base_url = registry_base_url_or("cloudrouter", &configured);
    if base_url.is_empty() {
        return Err(format!(
            "{CLOUDROUTER_OPEN_HTTP_URL_ENV} must be set for Rig cloud-router LLM access, \
             or the HTTP service registry must have a healthy cloudrouter instance"
        ));
    }
    let client = SdkworkAiClient::new_with_base_url(base_url).map_err(map_sdk_error)?;
    let api_key = std::env::var("SDKWORK_CLOUDROUTER_API_KEY").ok();
    for (name, value) in credential_headers(api_key.as_deref(), caller) {
        client.set_header(name, value);
    }
    Ok(client)
}

/// Builds the declared-service-identity client used by background work.
pub fn resolve_cloud_router_client_from_env() -> Result<SdkworkAiClient, String> {
    resolve_cloud_router_client(&CallerCredentials::default())
}

impl ModelProvider for CloudRouterChatModelProvider {
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            self.provider_id.clone(),
            "model",
            "rig-rust-cloud-router",
            "0.1.0",
            vec!["model.chat".to_string()],
        )
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }

    fn list_models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            self.rig_default_model_id.clone(),
            self.provider_id.clone(),
            "Rig Default Chat (Cloud Router)",
            "rig",
        )
        .with_capability("model.chat")
        .with_response_format(ModelResponseFormat::Text)
        .with_input_mode("text")
        .with_output_mode("text")
        .with_policy_category(PolicyCategory::ModelInvoke.as_str())
        .with_metadata("sdkwork.llm.backend", CLOUDROUTER_OPEN_SDK_CRATE)
        .with_metadata(
            "sdkwork.llm.backend_method",
            CLOUDROUTER_CHAT_COMPLETION_METHOD,
        )
        .with_metadata(
            "sdkwork.llm.upstream_default_model",
            self.upstream_default_model_id.clone(),
        )]
    }

    fn invoke(&self, request: ModelRequest) -> KernelResult<ModelResponse> {
        let upstream_model_id = resolve_upstream_model_id(
            request.model_id.as_deref(),
            &self.rig_default_model_id,
            &self.upstream_default_model_id,
        );
        let messages = build_chat_messages(&request);
        let completion_request = OpenAiChatCompletionRequest {
            model: upstream_model_id,
            messages,
            ..Default::default()
        };

        let client = Arc::clone(&self.client);
        let completion =
            block_on_async(async move { client.chat().create(&completion_request).await })
                .map_err(|error| {
                    KernelError::provider_error("cloud_router.chat.create", error.to_string())
                })?
                .map_err(|error| {
                    KernelError::provider_error("cloud_router.chat.create", map_sdk_error(error))
                })?;

        let answer = completion
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default();

        Ok(ModelResponse::text(
            request.model_request_id,
            self.provider_id.clone(),
            answer,
        ))
    }
}

fn resolve_upstream_model_id(
    model_id: Option<&str>,
    rig_default_model_id: &str,
    upstream_default_model_id: &str,
) -> String {
    match model_id.filter(|value| !is_blank(Some(value))) {
        Some(model_id) if model_id == rig_default_model_id => upstream_default_model_id.to_string(),
        Some(model_id) => model_id.to_string(),
        None => upstream_default_model_id.to_string(),
    }
}

fn build_chat_messages(request: &ModelRequest) -> Vec<OpenAiChatMessage> {
    let mut messages = Vec::new();

    if let Some(instruction) = request
        .metadata
        .iter()
        .find_map(|(key, value)| (key == "sdkwork.knowledge.system_instruction").then_some(value))
    {
        messages.push(OpenAiChatMessage {
            role: "system".to_string(),
            content: Some(instruction.clone()),
            ..Default::default()
        });
    }

    if !request.context_frames.is_empty() {
        let context = request
            .context_frames
            .iter()
            .map(|frame| {
                let title = frame
                    .metadata_value("sdkwork.knowledge.title")
                    .unwrap_or(frame.source.as_str());
                format!("## {title}\n{}", frame.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        messages.push(OpenAiChatMessage {
            role: "system".to_string(),
            content: Some(format!(
                "Use the following knowledge context when answering. Cite relevant sources in your answer.\n\n{context}"
            )),
            ..Default::default()
        });
    }

    for user_message in &request.messages {
        messages.push(OpenAiChatMessage {
            role: "user".to_string(),
            content: Some(user_message.clone()),
            ..Default::default()
        });
    }

    messages
}

fn map_sdk_error(error: SdkworkError) -> String {
    // Raw SDK error text can carry upstream URLs, request ids, or response
    // fragments; it is diagnostic-only and must not reach API error surfaces
    // (same policy as the engine adapters' `engine_provider_error`).
    tracing::warn!(error = %error, "cloud router sdk call failed");
    "cloud router request failed".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rig_default_model_id_maps_to_cloud_router_catalog_model() {
        assert_eq!(
            resolve_upstream_model_id(
                Some(RIG_DEFAULT_MODEL_ID),
                RIG_DEFAULT_MODEL_ID,
                DEFAULT_CLOUDROUTER_UPSTREAM_MODEL_ID
            ),
            DEFAULT_CLOUDROUTER_UPSTREAM_MODEL_ID
        );
    }

    #[test]
    fn explicit_profile_model_id_is_forwarded_to_cloud_router() {
        assert_eq!(
            resolve_upstream_model_id(
                Some("openai/gpt-4.1"),
                RIG_DEFAULT_MODEL_ID,
                DEFAULT_CLOUDROUTER_UPSTREAM_MODEL_ID
            ),
            "openai/gpt-4.1"
        );
    }
}

#[cfg(test)]
mod caller_identity_tests {
    use super::*;

    #[test]
    fn a_user_turn_sends_only_the_caller_pair() {
        let caller = CallerCredentials::new(
            Some("auth-token".to_owned()),
            Some("access-token".to_owned()),
        );
        let headers = credential_headers(Some("service-key"), &caller);
        assert_eq!(
            headers,
            vec![
                ("Authorization", "Bearer auth-token".to_owned()),
                ("Access-Token", "access-token".to_owned()),
            ]
        );
        assert!(!headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("x-api-key")));
    }

    #[test]
    fn a_caller_less_turn_uses_the_declared_service_identity() {
        let headers = credential_headers(Some("service-key"), &CallerCredentials::default());
        assert_eq!(headers, vec![("X-Api-Key", "service-key".to_owned())]);
        assert!(credential_headers(None, &CallerCredentials::default()).is_empty());
    }
}
