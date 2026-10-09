use sdkwork_utils_rust::is_blank;
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

pub fn resolve_cloud_router_client_from_env() -> Result<SdkworkAiClient, String> {
    let base_url = std::env::var(CLOUDROUTER_OPEN_HTTP_URL_ENV).map_err(|_| {
        format!("{CLOUDROUTER_OPEN_HTTP_URL_ENV} must be set for Rig cloud-router LLM access")
    })?;
    let client = SdkworkAiClient::new_with_base_url(base_url).map_err(map_sdk_error)?;

    if let Ok(api_key) = std::env::var("SDKWORK_CLOUDROUTER_API_KEY") {
        if !is_blank(Some(api_key.as_str())) {
            client.set_api_key(api_key);
        }
    }

    Ok(client)
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
