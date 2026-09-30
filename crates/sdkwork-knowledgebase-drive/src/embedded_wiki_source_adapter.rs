use async_trait::async_trait;
use sdkwork_drive_object_runtime::DriveObjectStoreRuntime;
use sdkwork_drive_storage_contract::{DriveByteRange, DriveObjectLocator, ReadObjectRangeRequest};
use sdkwork_drive_workspace_service::{
    application::resource_resolution_service::{
        ResolveDriveResourceCommand, SqlDriveResourceResolutionService,
    },
    domain::resource_resolution::{DriveResourceScopeKind, ResolvedDriveResource},
    infrastructure::sql::resource_resolution_store::SqlResourceResolutionStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_wiki_drive_source::{
    EnsureKnowledgebaseRawScopeRequest, KnowledgeWikiDriveScope, KnowledgeWikiDriveSource,
    KnowledgeWikiDriveSourceError, KnowledgeWikiSourceResource, KnowledgebaseRawScope,
    KnowledgebaseRawScopeEventDelivery, ReadKnowledgeWikiSourceRequest,
    RenewKnowledgebaseRawScopeEventDeliveryRequest, ResolveKnowledgeWikiSourceRequest,
    MAX_WIKI_SOURCE_READ_BYTES, ROOT_SCOPE_SUBSCRIPTION_TYPE,
};
use sdkwork_utils_rust::sha256_hash;
use sqlx::PgPool;

use crate::KnowledgebaseDriveRootScopeAdapter;

const MAX_SCOPE_GENERATION: i64 = i64::MAX;

/// Embedded standalone adapter for the same Drive root-scoped resource contract used by cloud.
/// Resolution remains owned by Drive's typed service and bytes are read through Drive's provider
/// runtime, so the Knowledgebase service never derives object keys or calls storage providers.
#[derive(Clone)]
pub struct KnowledgebaseDriveEmbeddedWikiSourceAdapter {
    scope: KnowledgebaseDriveRootScopeAdapter,
    resolver: SqlDriveResourceResolutionService,
    object_runtime: DriveObjectStoreRuntime,
    tenant_id: String,
}

impl KnowledgebaseDriveEmbeddedWikiSourceAdapter {
    pub fn new(pool: PgPool, tenant_id: impl Into<String>, operator_id: impl Into<String>) -> Self {
        let tenant_id = tenant_id.into();
        Self {
            scope: KnowledgebaseDriveRootScopeAdapter::new(
                pool.clone(),
                tenant_id.clone(),
                operator_id,
            ),
            resolver: SqlDriveResourceResolutionService::new(SqlResourceResolutionStore::new(
                pool.clone(),
            )),
            object_runtime: DriveObjectStoreRuntime::new(pool),
            tenant_id,
        }
    }

    async fn resolve_drive_resource(
        &self,
        request: ResolveKnowledgeWikiSourceRequest,
    ) -> Result<ResolvedDriveResource, KnowledgeWikiDriveSourceError> {
        self.resolver
            .resolve(ResolveDriveResourceCommand {
                tenant_id: self.tenant_id.clone(),
                scope_kind: DriveResourceScopeKind::RootScopeSubscription,
                scope_uuid: request.subscription_uuid,
                relative_path: request.relative_path,
                pinned_generation: request
                    .pinned_generation
                    .map(|value| parse_positive_i64(&value, "pinned_generation"))
                    .transpose()?,
                pinned_node_version_id: request.pinned_node_version_id,
            })
            .await
            .map_err(map_drive_error)
    }
}

#[async_trait]
impl KnowledgeWikiDriveScope for KnowledgebaseDriveEmbeddedWikiSourceAdapter {
    async fn ensure_raw_scope(
        &self,
        request: EnsureKnowledgebaseRawScopeRequest,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        self.scope.ensure_raw_scope(request).await
    }

    async fn retrieve_raw_scope(
        &self,
        subscription_uuid: &str,
    ) -> Result<KnowledgebaseRawScope, KnowledgeWikiDriveSourceError> {
        self.scope.retrieve_raw_scope(subscription_uuid).await
    }

    async fn renew_raw_scope_event_delivery(
        &self,
        request: RenewKnowledgebaseRawScopeEventDeliveryRequest,
    ) -> Result<KnowledgebaseRawScopeEventDelivery, KnowledgeWikiDriveSourceError> {
        self.scope.renew_raw_scope_event_delivery(request).await
    }
}

#[async_trait]
impl KnowledgeWikiDriveSource for KnowledgebaseDriveEmbeddedWikiSourceAdapter {
    async fn resolve_source(
        &self,
        request: ResolveKnowledgeWikiSourceRequest,
    ) -> Result<KnowledgeWikiSourceResource, KnowledgeWikiDriveSourceError> {
        map_resource(self.resolve_drive_resource(request).await?)
    }

    async fn read_pinned_source(
        &self,
        request: ReadKnowledgeWikiSourceRequest,
    ) -> Result<Vec<u8>, KnowledgeWikiDriveSourceError> {
        validate_read_request(&request)?;
        if request.resource.content_length == 0 {
            return Ok(Vec::new());
        }

        let resolved = self
            .resolve_drive_resource(ResolveKnowledgeWikiSourceRequest {
                subscription_uuid: request.resource.subscription_uuid.clone(),
                relative_path: request.resource.normalized_relative_path.clone(),
                pinned_generation: Some(request.resource.scope_generation.clone()),
                pinned_node_version_id: Some(request.resource.drive_node_version_id.clone()),
            })
            .await?;
        let public_resource = map_resource(resolved.clone())?;
        if public_resource != request.resource {
            return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
                "pinned Drive resource changed while reading".to_string(),
            ));
        }

        let object_store = self
            .object_runtime
            .resolve(
                &resolved.content_locator.storage_provider_id,
                sdkwork_drive_object_runtime::ProviderAccessIntent::Read,
            )
            .await
            .map_err(map_object_error)?;
        let (_, mut chunks) = object_store
            .read_object_range(ReadObjectRangeRequest {
                locator: DriveObjectLocator {
                    bucket: resolved.content_locator.bucket,
                    object_key: resolved.content_locator.object_key,
                },
                range: DriveByteRange {
                    start_inclusive: 0,
                    end_inclusive: request.resource.content_length - 1,
                },
            })
            .await
            .map_err(map_object_error)?;
        let mut body = Vec::with_capacity(request.resource.content_length as usize);
        while let Some(chunk) = chunks.next_chunk().await.map_err(map_object_error)? {
            let next_length = body.len().saturating_add(chunk.len()) as u64;
            if next_length > request.maximum_bytes {
                return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
                    "pinned Drive content exceeds the read limit".to_string(),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        if body.len() as u64 != request.resource.content_length
            || format!("sha256:{}", sha256_hash(&body)) != request.resource.checksum_sha256_hex
        {
            return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
                "pinned Drive content does not match its resolved checksum or length".to_string(),
            ));
        }
        Ok(body)
    }
}

fn map_resource(
    resource: ResolvedDriveResource,
) -> Result<KnowledgeWikiSourceResource, KnowledgeWikiDriveSourceError> {
    let scope_generation = positive_i64_string(resource.scope_generation, "scope_generation")?;
    let content_length = u64::try_from(resource.content_length).map_err(|_| {
        KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive resource content length must be nonnegative".to_string(),
        )
    })?;
    validate_checksum(&resource.checksum_sha256_hex)?;
    if resource.scope_kind != DriveResourceScopeKind::RootScopeSubscription
        || resource.scope_status != "ACTIVE"
        || resource.node_status != "ACTIVE"
        || resource.eligibility != "ELIGIBLE"
    {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive resource is not an active eligible root-scoped file".to_string(),
        ));
    }
    let checksum = resource.checksum_sha256_hex.clone();
    Ok(KnowledgeWikiSourceResource {
        scope_type: ROOT_SCOPE_SUBSCRIPTION_TYPE.to_string(),
        subscription_uuid: resource.scope_uuid,
        scope_generation,
        normalized_relative_path: resource.relative_path,
        resource_type: resource.resource_type,
        drive_node_id: resource.node_id,
        drive_node_version_id: resource.node_version_id,
        version_no: positive_i64_string(resource.version_no, "version_no")?,
        checksum_sha256_hex: checksum.clone(),
        etag: format!("\"{checksum}\""),
        content_type: resource.content_type,
        content_length,
        last_modified: resource.last_modified,
        scope_status: resource.scope_status,
        node_status: resource.node_status,
        eligibility: resource.eligibility,
    })
}

fn validate_read_request(
    request: &ReadKnowledgeWikiSourceRequest,
) -> Result<(), KnowledgeWikiDriveSourceError> {
    if request.maximum_bytes == 0 || request.maximum_bytes > MAX_WIKI_SOURCE_READ_BYTES {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "maximum_bytes is outside the bounded Wiki source read limit".to_string(),
        ));
    }
    if request.resource.content_length > request.maximum_bytes
        || request.resource.scope_type != ROOT_SCOPE_SUBSCRIPTION_TYPE
    {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(
            "pinned Drive resource is outside the bounded root scope read contract".to_string(),
        ));
    }
    validate_checksum(&request.resource.checksum_sha256_hex)
}

fn parse_positive_i64(value: &str, field_name: &str) -> Result<i64, KnowledgeWikiDriveSourceError> {
    let parsed = value.parse::<i64>().map_err(|_| {
        KnowledgeWikiDriveSourceError::InvalidRequest(format!(
            "{field_name} must be a canonical positive signed BIGINT"
        ))
    })?;
    if !(1..=MAX_SCOPE_GENERATION).contains(&parsed) {
        return Err(KnowledgeWikiDriveSourceError::InvalidRequest(format!(
            "{field_name} must be a canonical positive signed BIGINT"
        )));
    }
    Ok(parsed)
}

fn positive_i64_string(
    value: i64,
    field_name: &str,
) -> Result<String, KnowledgeWikiDriveSourceError> {
    if value < 1 {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(format!(
            "Drive {field_name} must be positive"
        )));
    }
    Ok(value.to_string())
}

fn validate_checksum(value: &str) -> Result<(), KnowledgeWikiDriveSourceError> {
    let Some(digest) = value.strip_prefix("sha256:") else {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive checksum must use the canonical sha256:<lowercase-hex> format".to_string(),
        ));
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(KnowledgeWikiDriveSourceError::IntegrityFailed(
            "Drive checksum must use the canonical sha256:<lowercase-hex> format".to_string(),
        ));
    }
    Ok(())
}

fn map_drive_error(
    error: sdkwork_drive_workspace_service::DriveServiceError,
) -> KnowledgeWikiDriveSourceError {
    use sdkwork_drive_workspace_service::DriveServiceError;
    match error {
        DriveServiceError::Validation(message) => {
            KnowledgeWikiDriveSourceError::InvalidRequest(message)
        }
        DriveServiceError::NotFound(message) => KnowledgeWikiDriveSourceError::NotFound(message),
        DriveServiceError::Conflict(message) => KnowledgeWikiDriveSourceError::Conflict(message),
        DriveServiceError::PermissionDenied(_) | DriveServiceError::Internal(_) => {
            KnowledgeWikiDriveSourceError::Upstream(
                "embedded Drive resource resolution failed".to_string(),
            )
        }
    }
}

fn map_object_error(
    error: sdkwork_drive_storage_contract::DriveObjectStoreError,
) -> KnowledgeWikiDriveSourceError {
    use sdkwork_drive_storage_contract::DriveObjectStoreErrorKind;
    match error.kind {
        DriveObjectStoreErrorKind::NotFound => KnowledgeWikiDriveSourceError::NotFound(
            "pinned Drive content was not found".to_string(),
        ),
        DriveObjectStoreErrorKind::InvalidRequest => {
            KnowledgeWikiDriveSourceError::InvalidRequest(error.message)
        }
        DriveObjectStoreErrorKind::IntegrityFailed => {
            KnowledgeWikiDriveSourceError::IntegrityFailed(error.message)
        }
        _ => KnowledgeWikiDriveSourceError::Upstream(
            "embedded Drive content provider is unavailable".to_string(),
        ),
    }
}
