use async_trait::async_trait;
use cloudrouter_open_sdk::{
    OpenAiAudioTranscriptionRequest, OpenAiFileReferenceInput, OpenAiImageGenerationRequest,
};
use sdkwork_intelligence_knowledgebase_service::ports::commerce_store::{
    KnowledgeMarketStore, KnowledgeMarketStoreError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_access_control::KnowledgeAccessRole;
use sdkwork_knowledgebase_contract::market::{
    KnowledgeMarketCatalogItem, KnowledgeMarketSubscriptionRequest,
    KnowledgeMarketSubscriptionResult,
};
use sdkwork_knowledgebase_contract::media_task::{
    KnowledgeMediaTaskRequest, KnowledgeMediaTaskResult, KnowledgeMediaTaskType,
};
use sdkwork_utils_rust::{is_blank, SdkWorkPageData};

use crate::{
    hosted_access::{ensure_runtime_tenant, require_actor_id, require_space_access_with_role},
    runtime::KnowledgebaseRuntime,
    ApiError, ApiResult, KnowledgeAppRequestContext, KnowledgeCommerceAppService,
};

#[derive(Clone)]
pub(crate) struct HostedCommerceService {
    runtime: KnowledgebaseRuntime,
}

impl HostedCommerceService {
    pub fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl KnowledgeCommerceAppService for HostedCommerceService {
    async fn list_market_listings(
        &self,
        context: KnowledgeAppRequestContext,
        cursor: Option<String>,
        page_size: Option<u32>,
    ) -> ApiResult<SdkWorkPageData<KnowledgeMarketCatalogItem>> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let normalized_page_size = crate::pagination::normalize_api_page_size(page_size)?;
        let cursor_id =
            crate::pagination::parse_opaque_u64_cursor(cursor.as_deref()).map_err(|_| {
                ApiError::invalid_request("invalid_parameter", "cursor must be a valid listing id")
            })?;
        let (items, next_cursor, has_more) = self
            .runtime
            .commerce_store()
            .list_catalog_page(
                context.tenant_id,
                context.actor_id,
                cursor_id,
                normalized_page_size,
            )
            .await
            .map_err(map_market_error)?;
        Ok(crate::pagination::cursor_page_data(
            items,
            next_cursor,
            has_more,
            normalized_page_size,
        ))
    }

    async fn create_market_subscription(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeMarketSubscriptionRequest,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let actor_id = require_actor_id(&context)?.parse::<u64>().map_err(|_| {
            ApiError::invalid_request("invalid_actor_id", "actor_id must be numeric")
        })?;
        if request.listing_id == 0 {
            return Err(ApiError::invalid_request(
                "invalid_market_subscription_request",
                "listing_id is required",
            ));
        }
        self.runtime
            .commerce_store()
            .subscribe(context.tenant_id, actor_id, request.listing_id)
            .await
            .map_err(map_market_error)?;
        Ok(KnowledgeMarketSubscriptionResult {
            accepted: true,
            status: "completed".to_string(),
        })
    }

    async fn delete_market_subscription(
        &self,
        context: KnowledgeAppRequestContext,
        listing_id: u64,
    ) -> ApiResult<KnowledgeMarketSubscriptionResult> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let actor_id = require_actor_id(&context)?.parse::<u64>().map_err(|_| {
            ApiError::invalid_request("invalid_actor_id", "actor_id must be numeric")
        })?;
        if listing_id == 0 {
            return Err(ApiError::invalid_request(
                "invalid_market_subscription_request",
                "listing_id is required",
            ));
        }
        self.runtime
            .commerce_store()
            .unsubscribe(context.tenant_id, actor_id, listing_id)
            .await
            .map_err(map_market_error)?;
        Ok(KnowledgeMarketSubscriptionResult {
            accepted: true,
            status: "completed".to_string(),
        })
    }

    async fn create_media_task(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeMediaTaskRequest,
    ) -> ApiResult<KnowledgeMediaTaskResult> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        if request.space_id == 0 {
            return Err(ApiError::invalid_request(
                "invalid_media_task_request",
                "space_id is required",
            ));
        }
        require_space_access_with_role(
            &self.runtime,
            &context,
            request.space_id,
            KnowledgeAccessRole::Writer,
        )
        .await?;
        match request.task_type {
            KnowledgeMediaTaskType::SpeechToText => {
                let source_url = require_https_media_url(request.source_url.as_deref())?;
                let client = resolve_media_client("media_transcription_provider_unavailable")?;
                let mut file = OpenAiFileReferenceInput::default();
                file.additional_properties
                    .insert("url".to_string(), serde_json::Value::String(source_url));
                let transcription = client
                    .audio()
                    .create_transcription(&OpenAiAudioTranscriptionRequest {
                        file,
                        language: None,
                        model: media_model(
                            "SDKWORK_KNOWLEDGEBASE_TRANSCRIPTION_MODEL",
                            "openai/whisper-1",
                        ),
                        prompt: normalize_optional_media_text(request.prompt.as_deref(), 4_000)?,
                        response_format: Some("json".to_string()),
                    })
                    .await
                    .map_err(|error| media_provider_error("media_transcription_failed", &error))?;
                if is_blank(Some(transcription.text.as_str())) {
                    return Err(ApiError::new(
                        axum::http::StatusCode::BAD_GATEWAY,
                        "media_transcription_invalid_response",
                        "transcription provider returned no text",
                    ));
                }
                Ok(KnowledgeMediaTaskResult {
                    accepted: true,
                    status: "completed".to_string(),
                    url: request.source_url,
                    resolution: None,
                    text: Some(transcription.text),
                    suggestions: Vec::new(),
                    similars: Vec::new(),
                })
            }
            KnowledgeMediaTaskType::GenerateImage => {
                let prompt = normalize_optional_media_text(request.prompt.as_deref(), 4_000)?
                    .ok_or_else(|| {
                        ApiError::invalid_request(
                            "invalid_media_task_request",
                            "prompt is required",
                        )
                    })?;
                let resolution = image_resolution(request.aspect_mode.as_deref())?;
                let quality = image_quality(request.style_mode.as_deref())?;
                let client = resolve_media_client("media_image_provider_unavailable")?;
                let result = client
                    .images()
                    .create_generation(&OpenAiImageGenerationRequest {
                        model: media_model(
                            "SDKWORK_KNOWLEDGEBASE_IMAGE_GENERATION_MODEL",
                            "openai/gpt-image-1",
                        ),
                        n: Some(1),
                        prompt,
                        quality,
                        response_format: Some("url".to_string()),
                        size: Some(resolution.to_string()),
                    })
                    .await
                    .map_err(|error| {
                        media_provider_error("media_image_generation_failed", &error)
                    })?;
                let image = result.data.into_iter().next().ok_or_else(|| {
                    ApiError::new(
                        axum::http::StatusCode::BAD_GATEWAY,
                        "media_image_invalid_response",
                        "image provider returned no output",
                    )
                })?;
                let image_url = require_https_provider_url(image.url.as_deref())?;
                Ok(KnowledgeMediaTaskResult {
                    accepted: true,
                    status: "completed".to_string(),
                    url: Some(image_url),
                    resolution: Some(resolution.to_string()),
                    text: None,
                    suggestions: image.revised_prompt.into_iter().collect(),
                    similars: Vec::new(),
                })
            }
        }
    }
}

fn resolve_media_client(
    unavailable_code: &'static str,
) -> ApiResult<cloudrouter_open_sdk::SdkworkAiClient> {
    sdkwork_knowledgebase_agent_provider::resolve_cloud_router_client_from_env().map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            unavailable_code,
            "the configured media provider is unavailable",
        )
    })
}

fn media_model(environment_key: &str, default_model: &str) -> String {
    std::env::var(environment_key)
        .ok()
        .filter(|value| !is_blank(Some(value.as_str())))
        .unwrap_or_else(|| default_model.to_string())
}

fn normalize_optional_media_text(
    value: Option<&str>,
    max_chars: usize,
) -> ApiResult<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max_chars {
        return Err(ApiError::invalid_request(
            "invalid_media_task_request",
            format!("media text exceeds {max_chars} characters"),
        ));
    }
    Ok(Some(value.to_string()))
}

fn require_https_media_url(value: Option<&str>) -> ApiResult<String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 2_048)
        .ok_or_else(|| {
            ApiError::invalid_request(
                "invalid_media_task_request",
                "source_url is required and must not exceed 2048 bytes",
            )
        })?;
    let url = url::Url::parse(value).map_err(|_| {
        ApiError::invalid_request(
            "invalid_media_task_request",
            "source_url must be a valid URL",
        )
    })?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() {
        return Err(ApiError::invalid_request(
            "invalid_media_task_request",
            "source_url must be an absolute HTTPS URL without user information",
        ));
    }
    let blocked_host = match url.host() {
        Some(url::Host::Domain(domain)) => {
            domain.eq_ignore_ascii_case("localhost")
                || domain.to_ascii_lowercase().ends_with(".localhost")
        }
        Some(url::Host::Ipv4(address)) => {
            address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_broadcast()
                || address.is_documentation()
                || address.is_unspecified()
        }
        Some(url::Host::Ipv6(address)) => {
            address.is_loopback() || address.is_unspecified() || address.is_unique_local()
        }
        None => true,
    };
    if blocked_host {
        return Err(ApiError::invalid_request(
            "invalid_media_task_request",
            "source_url host is not allowed",
        ));
    }
    Ok(url.to_string())
}

fn require_https_provider_url(value: Option<&str>) -> ApiResult<String> {
    require_https_media_url(value).map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "media_image_invalid_response",
            "image provider returned an invalid URL",
        )
    })
}

fn image_resolution(aspect_mode: Option<&str>) -> ApiResult<&'static str> {
    match aspect_mode.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("square") | Some("1:1") => Ok("1024x1024"),
        Some("landscape") | Some("3:2") | Some("16:9") => Ok("1536x1024"),
        Some("portrait") | Some("2:3") | Some("9:16") => Ok("1024x1536"),
        Some(_) => Err(ApiError::invalid_request(
            "invalid_media_task_request",
            "aspect_mode must be square, landscape, portrait, 1:1, 3:2, 2:3, 16:9, or 9:16",
        )),
    }
}

fn image_quality(style_mode: Option<&str>) -> ApiResult<Option<String>> {
    match style_mode.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("auto") => Ok(Some("auto".to_string())),
        Some(value @ ("low" | "medium" | "high")) => Ok(Some(value.to_string())),
        Some(_) => Err(ApiError::invalid_request(
            "invalid_media_task_request",
            "style_mode must be auto, low, medium, or high",
        )),
    }
}

fn media_provider_error(
    code: &'static str,
    error: &cloudrouter_open_sdk::SdkworkError,
) -> ApiError {
    tracing::warn!(error = %error, "Cloud Router media request failed");
    ApiError::new(
        axum::http::StatusCode::BAD_GATEWAY,
        code,
        "the media provider request failed",
    )
}

fn map_market_error(error: KnowledgeMarketStoreError) -> ApiError {
    match error {
        KnowledgeMarketStoreError::InvalidRequest(detail) => {
            ApiError::invalid_request("invalid_market_request", detail)
        }
        KnowledgeMarketStoreError::NotFound => ApiError::new(
            axum::http::StatusCode::NOT_FOUND,
            "market_listing_not_found",
            "market listing was not found",
        ),
        KnowledgeMarketStoreError::Internal(detail) => {
            ApiError::internal("market_store_failed", detail)
        }
    }
}
