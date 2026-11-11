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
        DriveObjectStoreErrorKind::Conflict => {
            KnowledgeWikiDriveSourceError::Conflict(error.message)
        }
        DriveObjectStoreErrorKind::IntegrityFailed => {
            KnowledgeWikiDriveSourceError::IntegrityFailed(error.message)
        }
        _ => KnowledgeWikiDriveSourceError::Upstream(
            "embedded Drive content provider is unavailable".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_drive_storage_contract::{DriveObjectStoreError, DriveObjectStoreErrorKind};
    use sdkwork_drive_workspace_service::domain::resource_resolution::DriveResourceContentLocator;
    use sdkwork_drive_workspace_service::DriveServiceError;

    fn resolved_resource() -> ResolvedDriveResource {
        ResolvedDriveResource {
            scope_kind: DriveResourceScopeKind::RootScopeSubscription,
            scope_uuid: "scope-uuid".to_string(),
            scope_generation: 3,
            relative_path: "sources/raw/page.md".to_string(),
            resource_type: "file".to_string(),
            node_id: "node-id".to_string(),
            node_version_id: "node-version-id".to_string(),
            version_no: 2,
            checksum_sha256_hex: format!("sha256:{}", "a".repeat(64)),
            content_type: "text/markdown".to_string(),
            content_length: 4,
            last_modified: "2026-11-11T00:00:00Z".to_string(),
            scope_status: "ACTIVE".to_string(),
            node_status: "ACTIVE".to_string(),
            eligibility: "ELIGIBLE".to_string(),
            content_locator: DriveResourceContentLocator {
                storage_provider_id: "local".to_string(),
                bucket: "knowledgebase".to_string(),
                object_key: "object-key".to_string(),
            },
        }
    }

    fn pinned_resource() -> KnowledgeWikiSourceResource {
        map_resource(resolved_resource()).expect("canonical resolved resource maps")
    }

    fn read_request(
        resource: KnowledgeWikiSourceResource,
        maximum_bytes: u64,
    ) -> ReadKnowledgeWikiSourceRequest {
        ReadKnowledgeWikiSourceRequest {
            resource,
            maximum_bytes,
        }
    }

    /// The deleted Internal API transport turned Drive's HTTP 409 into `Conflict`.
    /// The embedded port must keep that kind so a permanent conflict is neither
    /// reported as a retryable upstream outage nor as a new failure class.
    #[test]
    fn object_store_errors_keep_the_http_transport_error_kinds() {
        use DriveObjectStoreErrorKind::{
            Conflict, IntegrityFailed, InvalidRequest, NotFound, PermissionDenied, RateLimited,
            Timeout, Unavailable,
        };
        for (kind, expected_code) in [
            (NotFound, "wiki_drive_source_not_found"),
            (InvalidRequest, "wiki_drive_source_request_invalid"),
            (Conflict, "wiki_drive_source_conflict"),
            (IntegrityFailed, "wiki_drive_source_integrity_failed"),
            (PermissionDenied, "wiki_drive_source_upstream_failed"),
            (RateLimited, "wiki_drive_source_upstream_failed"),
            (Timeout, "wiki_drive_source_upstream_failed"),
            (Unavailable, "wiki_drive_source_upstream_failed"),
        ] {
            let mapped = map_object_error(DriveObjectStoreError::new(kind, "provider detail"));
            assert_eq!(mapped.code(), expected_code, "unexpected kind for {kind:?}");
        }
    }

    /// Drive's typed service still answers over the same status contract
    /// (400/422, 404, 409, 403, 500) the deleted transport mapped, so the kinds
    /// and public HTTP statuses must not drift.
    #[test]
    fn drive_service_errors_keep_the_http_transport_error_kinds() {
        for (error, expected_code) in [
            (
                DriveServiceError::Validation("detail".to_string()),
                "wiki_drive_source_request_invalid",
            ),
            (
                DriveServiceError::NotFound("detail".to_string()),
                "wiki_drive_source_not_found",
            ),
            (
                DriveServiceError::Conflict("detail".to_string()),
                "wiki_drive_source_conflict",
            ),
            (
                DriveServiceError::PermissionDenied("detail".to_string()),
                "wiki_drive_source_upstream_failed",
            ),
            (
                DriveServiceError::Internal("detail".to_string()),
                "wiki_drive_source_upstream_failed",
            ),
        ] {
            assert_eq!(map_drive_error(error).code(), expected_code);
        }
    }

    #[test]
    fn read_request_keeps_the_bounded_size_and_checksum_contract() {
        assert!(validate_read_request(&read_request(pinned_resource(), 4)).is_ok());
        for maximum_bytes in [0, MAX_WIKI_SOURCE_READ_BYTES + 1] {
            let error = validate_read_request(&read_request(pinned_resource(), maximum_bytes))
                .expect_err("maximum_bytes outside the bounded read limit must fail");
            assert_eq!(error.code(), "wiki_drive_source_request_invalid");
        }

        let oversized = read_request(pinned_resource(), 3);
        assert_eq!(
            validate_read_request(&oversized)
                .expect_err("content above the read limit must fail")
                .code(),
            "wiki_drive_source_request_invalid"
        );

        let mut foreign_scope = pinned_resource();
        foreign_scope.scope_type = "DRIVE_SPACE".to_string();
        assert_eq!(
            validate_read_request(&read_request(foreign_scope, 4))
                .expect_err("a non root-scope resource must fail")
                .code(),
            "wiki_drive_source_request_invalid"
        );

        let mut bad_checksum = pinned_resource();
        bad_checksum.checksum_sha256_hex = "sha256:NOT-HEX".to_string();
        assert_eq!(
            validate_read_request(&read_request(bad_checksum, 4))
                .expect_err("a non canonical checksum must fail")
                .code(),
            "wiki_drive_source_integrity_failed"
        );
    }

    fn assert_integrity_failure(mutate: impl FnOnce(&mut ResolvedDriveResource)) {
        let mut resource = resolved_resource();
        mutate(&mut resource);
        assert_eq!(
            map_resource(resource)
                .expect_err("an ineligible Drive resource must fail integrity")
                .code(),
            "wiki_drive_source_integrity_failed"
        );
    }

    #[test]
    fn resolved_resource_must_be_an_active_eligible_root_scope() {
        assert_eq!(
            map_resource(resolved_resource())
                .expect("canonical")
                .version_no,
            "2"
        );
        assert_integrity_failure(|resource| resource.scope_status = "REVOKED".to_string());
        assert_integrity_failure(|resource| resource.node_status = "DELETED".to_string());
        assert_integrity_failure(|resource| resource.eligibility = "QUARANTINED".to_string());
        assert_integrity_failure(|resource| resource.content_length = -1);
    }
}
