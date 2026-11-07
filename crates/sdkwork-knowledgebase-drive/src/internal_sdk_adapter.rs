use async_trait::async_trait;
use sdkwork_drive_internal_sdk_generated_rust::{
    CreateRootScopeSubscriptionRequest, DriveResourceResolution,
    EnsureRootScopeEventDeliveryRequest, ResolveDriveResourceRequest, RootScopeEventDelivery,
    RootScopeSubscription, SdkworkCustomClient, SdkworkError,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_drive_source::{
    EnsureKnowledgebaseRawScopeRequest, KnowledgeWikiDriveEventDeliveryMode,
    KnowledgeWikiDriveScope, KnowledgeWikiDriveSource, KnowledgeWikiDriveSourceError,
    KnowledgeWikiSourceResource, KnowledgebaseRawScope, KnowledgebaseRawScopeEventDelivery,
    ReadKnowledgeWikiSourceRequest, RenewKnowledgebaseRawScopeEventDeliveryRequest,
    ResolveKnowledgeWikiSourceRequest, KNOWLEDGEBASE_RAW_CONSUMER_KIND, MAX_WIKI_SOURCE_READ_BYTES,
    ROOT_SCOPE_SUBSCRIPTION_TYPE,
};
use sdkwork_utils_rust::service_base_url::registry_base_url_or;
use sdkwork_utils_rust::{hmac_sha256, is_blank, sha256_hash};

const MIN_EVENT_CHANNEL_TTL_SECONDS: u64 = 3_600;
const MAX_EVENT_CHANNEL_TTL_SECONDS: u64 = 2_592_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgebaseDriveEventDeliveryConfig {
    pub callback_url: String,
    pub signing_master_secret: String,
    pub channel_ttl_seconds: u64,
}

#[derive(Clone)]
pub struct KnowledgebaseDriveInternalSdkAdapter {
    client: SdkworkCustomClient,
    event_delivery: Option<KnowledgebaseDriveEventDeliveryConfig>,
}

impl KnowledgebaseDriveInternalSdkAdapter {
    pub fn new(client: SdkworkCustomClient) -> Self {
        Self {
            client,
            event_delivery: None,
        }
    }

    pub fn with_event_delivery(
        mut self,
        config: KnowledgebaseDriveEventDeliveryConfig,
    ) -> Result<Self, KnowledgeWikiDriveSourceError> {
        validate_event_delivery_config(&config)?;
        self.event_delivery = Some(config);
        Ok(self)
    }

    pub fn from_ingress_token(
        base_url: impl Into<String>,
        ingress_token: impl Into<String>,
    ) -> Result<Self, KnowledgeWikiDriveSourceError> {
        let base_url = base_url.into();
        let ingress_token = ingress_token.into();
        if is_blank(Some(&base_url)) {
            return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
                "Drive Internal API base URL is required".to_string(),
            ));
        }
        if ingress_token.len() < 16
            || ingress_token.len() > 4_096
            || ingress_token.chars().any(char::is_whitespace)
        {
            return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
                "Drive Internal API ingress token is invalid".to_string(),
            ));
        }
        // DISCOVERY_SPEC §12.1: registry-first base URL, configured topology as
        // the degradation fallback.
        let base_url = registry_base_url_or("sdkwork-drive", &base_url);
        let client = SdkworkCustomClient::new_with_base_url(base_url).map_err(map_sdk_error)?;
        client.set_api_key(&ingress_token);
        Ok(Self::new(client))
    }
}

#[async_trait]
impl KnowledgeWikiDriveScope for KnowledgebaseDriveInternalSdkAdapter {
    async fn ensure_raw_scope(
        &self,
        request: EnsureKnowledgebaseRawScopeRequest,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        let drive_space_id = require_identifier(&request.drive_space_id, "drive_space_id")?;
        let knowledgebase_uuid =
            require_identifier(&request.knowledgebase_uuid, "knowledgebase_uuid")?;
        let response = self
            .client
            .drive_internal_publishing()
            .root_scope_subscriptions_create(&CreateRootScopeSubscriptionRequest {
                space_id: drive_space_id,
                knowledge_base_id: knowledgebase_uuid,
            })
            .await
            .map_err(map_sdk_error)?;
        let scope = map_subscription(response)?;
        if let Some(config) = self.event_delivery.as_ref() {
            self.ensure_event_delivery(&scope.subscription_uuid, config)
                .await?;
        }
        Ok(scope)
    }

    async fn retrieve_raw_scope(
        &self,
        subscription_uuid: &str,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        let subscription_uuid = require_identifier(subscription_uuid, "subscription_uuid")?;
        let response = self
            .client
            .drive_internal_publishing()
            .root_scope_subscriptions_retrieve(&subscription_uuid)
            .await
            .map_err(map_sdk_error)?;
        map_subscription(response)
    }

    async fn renew_raw_scope_event_delivery(
        &self,
        request: RenewKnowledgebaseRawScopeEventDeliveryRequest,
    ) -> Result<KnowledgebaseRawScopeEventDelivery, KnowledgeWikiDriveSourceError> {
        let subscription_uuid =
            require_identifier(&request.subscription_uuid, "subscription_uuid")?;
        let config = self.event_delivery.as_ref().ok_or_else(|| {
            KnowledgeWikiDriveSourceError::Upstream(
                "Drive event delivery renewal is not configured for cloud deployment".to_string(),
            )
        })?;
        self.ensure_event_delivery(&subscription_uuid, config).await
    }
}

impl KnowledgebaseDriveInternalSdkAdapter {
    async fn ensure_event_delivery(
        &self,
        subscription_uuid: &str,
        config: &KnowledgebaseDriveEventDeliveryConfig,
    ) -> Result<KnowledgebaseRawScopeEventDelivery, KnowledgeWikiDriveSourceError> {
        let expiration_epoch_ms = (time::OffsetDateTime::now_utc().unix_timestamp_nanos()
            / 1_000_000)
            .checked_add((config.channel_ttl_seconds as i128) * 1_000)
            .ok_or_else(|| {
                KnowledgeWikiDriveSourceError::InvalidRequest(
                    "Drive event channel expiration exceeds int64".to_string(),
                )
            })?;
        if expiration_epoch_ms > i64::MAX as i128 {
            return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
                "Drive event channel expiration exceeds int64".to_string(),
            ));
        }
        let verification_token = hmac_sha256(
            subscription_uuid.as_bytes(),
            config.signing_master_secret.as_bytes(),
        );
        let delivery = self
            .client
            .drive_internal_publishing()
            .root_scope_event_deliveries_replace(
                subscription_uuid,
                &EnsureRootScopeEventDeliveryRequest {
                    address: config.callback_url.clone(),
                    verification_token,
                    expiration_epoch_ms: expiration_epoch_ms.to_string(),
                },
            )
            .await
            .map_err(map_sdk_error)?;
        validate_event_delivery(subscription_uuid, config, &delivery)?;
        Ok(KnowledgebaseRawScopeEventDelivery {
            subscription_uuid: subscription_uuid.to_string(),
            channel_id: delivery.channel_id,
            expiration_epoch_ms: delivery.expiration_epoch_ms.parse::<i64>().ok(),
            mode: KnowledgeWikiDriveEventDeliveryMode::CloudWebhook,
        })
    }
}

fn validate_event_delivery_config(
    config: &KnowledgebaseDriveEventDeliveryConfig,
) -> Result<(), KnowledgeWikiDriveSourceError> {
    let callback = url::Url::parse(&config.callback_url).map_err(|_| {
        KnowledgeWikiDriveSourceError::InvalidRequest(
            "Knowledgebase Drive event callback URL is invalid".to_string(),
        )
    })?;
    if callback.scheme() != "https"
        || callback.host_str().is_none()
        || !callback.username().is_empty()
        || callback.password().is_some()
        || callback.query().is_some()
        || callback.fragment().is_some()
        || callback.path() != "/internal/v3/api/knowledgebase/drive_events"
    {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "Knowledgebase Drive event callback must be a credential-free HTTPS internal-api URL"
                .to_string(),
        ));
    }
    if config.signing_master_secret.len() < 32
        || config.signing_master_secret.len() > 1_024
        || !config
            .signing_master_secret
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "Knowledgebase Drive event signing secret is invalid".to_string(),
        ));
    }
    if !(MIN_EVENT_CHANNEL_TTL_SECONDS..=MAX_EVENT_CHANNEL_TTL_SECONDS)
        .contains(&config.channel_ttl_seconds)
    {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(format!(
            "Drive event channel TTL must be between {MIN_EVENT_CHANNEL_TTL_SECONDS} and {MAX_EVENT_CHANNEL_TTL_SECONDS} seconds"
        )));
    }
    Ok(())
}

fn validate_event_delivery(
    subscription_uuid: &str,
    config: &KnowledgebaseDriveEventDeliveryConfig,
    delivery: &RootScopeEventDelivery,
) -> Result<(), KnowledgeWikiDriveSourceError> {
    if delivery.channel_id != format!("kbraw:{subscription_uuid}")
        || delivery.subscription_uuid != subscription_uuid
        || delivery.address != config.callback_url
        || !delivery.lifecycle_status.eq_ignore_ascii_case("ACTIVE")
        || delivery.expiration_epoch_ms.parse::<i64>().ok().is_none()
    {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive event delivery response does not match the requested root scope".to_string(),
        ));
    }
    Ok(())
}

#[async_trait]
impl KnowledgeWikiDriveSource for KnowledgebaseDriveInternalSdkAdapter {
    async fn resolve_source(
        &self,
        request: ResolveKnowledgeWikiSourceRequest,
    ) -> Result<KnowledgeWikiSourceResource, KnowledgeWikiDriveSourceError> {
        let subscription_uuid =
            require_identifier(&request.subscription_uuid, "subscription_uuid")?;
        let relative_path = require_relative_path(&request.relative_path)?;
        let response = self
            .client
            .drive_internal_publishing()
            .drive_resources_resolve(&ResolveDriveResourceRequest {
                scope_type: ROOT_SCOPE_SUBSCRIPTION_TYPE.to_string(),
                scope_uuid: subscription_uuid,
                relative_path,
                pinned_generation: normalize_optional(request.pinned_generation),
                pinned_node_version_id: normalize_optional(request.pinned_node_version_id),
            })
            .await
            .map_err(map_sdk_error)?;
        map_resource(response)
    }

    async fn read_pinned_source(
        &self,
        request: ReadKnowledgeWikiSourceRequest,
    ) -> Result<Vec<u8>, KnowledgeWikiDriveSourceError> {
        validate_read_request(&request)?;
        if request.resource.content_length == 0 {
            return Ok(Vec::new());
        }

        let range = format!("bytes=0-{}", request.resource.content_length - 1);
        let etag =
            (!is_blank(Some(&request.resource.etag))).then_some(request.resource.etag.as_str());
        let bytes = self
            .client
            .drive_internal_publishing()
            .drive_resource_content_retrieve(
                &request.resource.drive_node_version_id,
                ROOT_SCOPE_SUBSCRIPTION_TYPE,
                &request.resource.subscription_uuid,
                &request.resource.normalized_relative_path,
                Some(&request.resource.scope_generation),
                Some(&range),
                etag,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(map_sdk_error)?;

        let actual_length = bytes.len() as u64;
        if actual_length != request.resource.content_length {
            return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(format!(
                "pinned Drive content length {actual_length} does not match resolved length {}",
                request.resource.content_length
            )));
        }
        let actual_checksum = format!("sha256:{}", sha256_hash(&bytes));
        if !actual_checksum.eq_ignore_ascii_case(&request.resource.checksum_sha256_hex) {
            return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
                "pinned Drive content checksum does not match resource resolution".to_string(),
            ));
        }
        Ok(bytes)
    }
}

fn map_subscription(
    subscription: RootScopeSubscription,
) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
    if !subscription
        .consumer_kind
        .eq_ignore_ascii_case(KNOWLEDGEBASE_RAW_CONSUMER_KIND)
    {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(format!(
            "Drive root scope consumer kind must be {KNOWLEDGEBASE_RAW_CONSUMER_KIND}"
        )));
    }
    Ok(KnowledgebaseRawScope {
        subscription_uuid: require_identifier(&subscription.uuid, "subscription_uuid")?,
        drive_space_id: require_identifier(&subscription.space_id, "drive_space_id")?,
        consumer_kind: KNOWLEDGEBASE_RAW_CONSUMER_KIND.to_string(),
        knowledgebase_uuid: require_identifier(
            &subscription.consumer_resource_id,
            "knowledgebase_uuid",
        )?,
        raw_folder_node_id: require_identifier(&subscription.root_node_id, "raw_folder_node_id")?,
        scope_status: require_non_blank(subscription.scope_status, "scope_status")?,
        version: require_non_blank(subscription.version, "scope_version")?,
        created_at: require_non_blank(subscription.created_at, "created_at")?,
        updated_at: require_non_blank(subscription.updated_at, "updated_at")?,
    })
}

fn map_resource(
    resource: DriveResourceResolution,
) -> Result<KnowledgeWikiSourceResource, KnowledgeWikiDriveSourceError> {
    if resource.scope_type != ROOT_SCOPE_SUBSCRIPTION_TYPE {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(format!(
            "Drive resource scope type must be {ROOT_SCOPE_SUBSCRIPTION_TYPE}"
        )));
    }
    let content_length = resource.content_length.parse::<u64>().map_err(|_| {
        KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive resource content_length must be an unsigned integer".to_string(),
        )
    })?;
    let checksum_sha256_hex = normalize_checksum(&resource.checksum_sha256_hex)?;
    Ok(KnowledgeWikiSourceResource {
        scope_type: resource.scope_type,
        subscription_uuid: require_identifier(&resource.scope_uuid, "subscription_uuid")?,
        scope_generation: require_non_blank(resource.scope_generation, "scope_generation")?,
        normalized_relative_path: require_relative_path(&resource.normalized_relative_path)?,
        resource_type: require_non_blank(resource.resource_type, "resource_type")?,
        drive_node_id: require_identifier(&resource.node_id, "drive_node_id")?,
        drive_node_version_id: require_identifier(
            &resource.logical_node_version_id,
            "drive_node_version_id",
        )?,
        version_no: require_non_blank(resource.version_no, "version_no")?,
        checksum_sha256_hex,
        etag: resource.etag.trim().to_string(),
        content_type: require_non_blank(resource.content_type, "content_type")?,
        content_length,
        last_modified: require_non_blank(resource.last_modified, "last_modified")?,
        scope_status: require_non_blank(resource.scope_status, "scope_status")?,
        node_status: require_non_blank(resource.node_status, "node_status")?,
        eligibility: require_non_blank(resource.eligibility, "eligibility")?,
    })
}

fn validate_read_request(
    request: &ReadKnowledgeWikiSourceRequest,
) -> Result<(), KnowledgeWikiDriveSourceError> {
    if request.maximum_bytes == 0 || request.maximum_bytes > MAX_WIKI_SOURCE_READ_BYTES {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(format!(
            "maximum_bytes must be between 1 and {MAX_WIKI_SOURCE_READ_BYTES}"
        )));
    }
    if request.resource.content_length > request.maximum_bytes {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(format!(
            "Drive resource size {} exceeds read limit {}",
            request.resource.content_length, request.maximum_bytes
        )));
    }
    if request.resource.scope_type != ROOT_SCOPE_SUBSCRIPTION_TYPE {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "only ROOT_SCOPE_SUBSCRIPTION resources may be read".to_string(),
        ));
    }
    require_identifier(&request.resource.subscription_uuid, "subscription_uuid")?;
    require_identifier(
        &request.resource.drive_node_version_id,
        "drive_node_version_id",
    )?;
    require_non_blank(
        request.resource.scope_generation.clone(),
        "scope_generation",
    )?;
    require_relative_path(&request.resource.normalized_relative_path)?;
    normalize_checksum(&request.resource.checksum_sha256_hex)?;
    Ok(())
}

fn require_identifier(
    value: &str,
    field_name: &str,
) -> Result<String, KnowledgeWikiDriveSourceError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 160
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(format!(
            "invalid {field_name}"
        )));
    }
    Ok(value.to_string())
}

fn require_relative_path(value: &str) -> Result<String, KnowledgeWikiDriveSourceError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 2048
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "relative_path must be a normalized path below sources/raw".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn normalize_checksum(value: &str) -> Result<String, KnowledgeWikiDriveSourceError> {
    let value = value.trim();
    let Some(digest) = value.strip_prefix("sha256:") else {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive resource checksum must use the canonical sha256:<lowercase-hex> format"
                .to_string(),
        ));
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive resource checksum must use the canonical sha256:<lowercase-hex> format"
                .to_string(),
        ));
    }
    Ok(value.to_string())
}

fn require_non_blank(
    value: String,
    field_name: &str,
) -> Result<String, KnowledgeWikiDriveSourceError> {
    if is_blank(Some(&value)) {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(format!(
            "Drive response {field_name} is required"
        )));
    }
    Ok(value.trim().to_string())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn map_sdk_error(error: SdkworkError) -> KnowledgeWikiDriveSourceError {
    match error {
        SdkworkError::HttpStatus {
            status: 400 | 422, ..
        } => KnowledgeWikiDriveSourceError::InvalidRequest(
            "Drive Internal API rejected the request".to_string(),
        ),
        SdkworkError::HttpStatus { status: 404, .. } => KnowledgeWikiDriveSourceError::NotFound(
            "Drive root scope or resource was not found".to_string(),
        ),
        SdkworkError::HttpStatus { status: 409, .. } => KnowledgeWikiDriveSourceError::Conflict(
            "Drive root scope or resource changed concurrently".to_string(),
        ),
        SdkworkError::HttpStatus { status, .. } => KnowledgeWikiDriveSourceError::Upstream(
            format!("Drive Internal API returned HTTP status {status}"),
        ),
        SdkworkError::ApiStatus { code, trace_id } => KnowledgeWikiDriveSourceError::Upstream(
            format!("Drive Internal API returned code {code} (traceId={trace_id})"),
        ),
        SdkworkError::ResponseBodyTooLarge { maximum_bytes } => {
            KnowledgeWikiDriveSourceError::Upstream(format!(
                "Drive Internal API response exceeds {maximum_bytes} bytes"
            ))
        }
        _ => KnowledgeWikiDriveSourceError::Upstream(
            "Drive Internal API transport or response validation failed".to_string(),
        ),
    }
}
