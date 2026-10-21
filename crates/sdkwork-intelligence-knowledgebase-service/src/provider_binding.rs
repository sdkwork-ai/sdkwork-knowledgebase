use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::{stream, StreamExt};
use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineCapability, KnowledgeEngineError, KnowledgeEngineHealthStatus,
    KnowledgeEngineProviderErrorCategory,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    CreateKnowledgeEngineProviderBindingRequest,
    CreateKnowledgeEngineProviderCredentialReferenceRequest, KnowledgeEngineExecutionContext,
    KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingList,
    KnowledgeEngineProviderBindingState, KnowledgeEngineProviderCredentialReference,
    KnowledgeEngineProviderCredentialReferenceList, ListKnowledgeEngineProviderBindingsRequest,
    ListKnowledgeEngineProviderCredentialReferencesRequest,
    RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    RotateKnowledgeEngineProviderCredentialReferenceRequest,
    UpdateKnowledgeEngineProviderBindingRequest,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

use crate::ports::knowledge_engine::KnowledgeEngineRegistry;
use crate::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderBindingStoreError,
    KnowledgeEngineProviderScope, RecordKnowledgeEngineProviderTestResult,
};
use crate::ports::knowledge_provider_credential_resolver::{
    KnowledgeEngineProviderCredentialAccessContext, KnowledgeEngineProviderCredentialError,
    KnowledgeEngineProviderCredentialResolver,
};

pub const KNOWLEDGE_PLATFORM_MANAGE_PERMISSION: &str = "knowledge.platform.manage";
const MAX_CONCURRENT_PROVIDER_HEALTH_PROBES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeEngineProviderHealthSummary {
    pub implementation_ids: Vec<String>,
    pub degraded: bool,
}

pub struct KnowledgeEngineProviderBindingService<R> {
    store: Arc<dyn KnowledgeEngineProviderBindingStore>,
    registry: Arc<R>,
    credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
}

impl<R> KnowledgeEngineProviderBindingService<R>
where
    R: KnowledgeEngineRegistry + 'static,
{
    pub fn new(
        store: Arc<dyn KnowledgeEngineProviderBindingStore>,
        registry: Arc<R>,
        credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
    ) -> Self {
        Self {
            store,
            registry,
            credential_resolver,
        }
    }

    pub async fn create_credential_reference(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: CreateKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<
        KnowledgeEngineProviderCredentialReference,
        KnowledgeEngineProviderBindingServiceError,
    > {
        validate_management_context(context, None)?;
        self.require_executable_external_implementation(&request.implementation_id)?;
        self.credential_resolver
            .validate_reference_locator(&request.implementation_id, &request.reference_locator)?;
        self.store
            .create_credential_reference(provider_scope(context), &context.actor_id, request)
            .await
            .map_err(Into::into)
    }

    pub async fn create_binding(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: CreateKnowledgeEngineProviderBindingRequest,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingServiceError> {
        validate_management_context(context, Some(request.space_id))?;
        self.require_executable_external_implementation(&request.implementation_id)?;
        self.store
            .create_binding(provider_scope(context), &context.actor_id, request)
            .await
            .map_err(Into::into)
    }

    pub async fn get_credential_reference(
        &self,
        context: &KnowledgeEngineExecutionContext,
        credential_reference_id: u64,
    ) -> Result<
        KnowledgeEngineProviderCredentialReference,
        KnowledgeEngineProviderBindingServiceError,
    > {
        validate_management_context(context, None)?;
        self.store
            .get_credential_reference(provider_scope(context), credential_reference_id)
            .await
            .map_err(Into::into)
    }

    pub async fn list_credential_references(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: ListKnowledgeEngineProviderCredentialReferencesRequest,
    ) -> Result<
        KnowledgeEngineProviderCredentialReferenceList,
        KnowledgeEngineProviderBindingServiceError,
    > {
        validate_management_context(context, None)?;
        if let Some(implementation_id) = request.implementation_id.as_deref() {
            self.require_executable_external_implementation(implementation_id)?;
        }
        self.store
            .list_credential_references(provider_scope(context), request)
            .await
            .map_err(Into::into)
    }

    pub async fn rotate_credential_reference(
        &self,
        context: &KnowledgeEngineExecutionContext,
        credential_reference_id: u64,
        request: RotateKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<
        KnowledgeEngineProviderCredentialReference,
        KnowledgeEngineProviderBindingServiceError,
    > {
        validate_management_context(context, None)?;
        let credential = self
            .store
            .get_credential_reference(provider_scope(context), credential_reference_id)
            .await?;
        self.require_executable_external_implementation(&credential.implementation_id)?;
        self.credential_resolver.validate_reference_locator(
            &credential.implementation_id,
            &request.reference_locator,
        )?;
        self.store
            .rotate_credential_reference(
                provider_scope(context),
                credential_reference_id,
                &context.actor_id,
                request,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn revoke_credential_reference(
        &self,
        context: &KnowledgeEngineExecutionContext,
        credential_reference_id: u64,
        request: RevokeKnowledgeEngineProviderCredentialReferenceRequest,
    ) -> Result<
        KnowledgeEngineProviderCredentialReference,
        KnowledgeEngineProviderBindingServiceError,
    > {
        validate_management_context(context, None)?;
        self.store
            .revoke_credential_reference(
                provider_scope(context),
                credential_reference_id,
                &context.actor_id,
                request,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn get_binding(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding_id: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingServiceError> {
        validate_management_context(context, None)?;
        let binding = self
            .store
            .get_binding(provider_scope(context), binding_id)
            .await?;
        require_space_scope(context, binding.space_id)?;
        Ok(binding)
    }

    pub async fn list_bindings(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: ListKnowledgeEngineProviderBindingsRequest,
    ) -> Result<KnowledgeEngineProviderBindingList, KnowledgeEngineProviderBindingServiceError>
    {
        let space_id = request.space_id.ok_or_else(|| {
            KnowledgeEngineProviderBindingServiceError::InvalidRequest(
                "space_id is required for paginated Provider binding lists".to_string(),
            )
        })?;
        validate_management_context(context, Some(space_id))?;
        self.store
            .list_bindings(provider_scope(context), request)
            .await
            .map_err(Into::into)
    }

    pub async fn update_binding(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding_id: u64,
        request: UpdateKnowledgeEngineProviderBindingRequest,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingServiceError> {
        let binding = self.get_binding(context, binding_id).await?;
        validate_management_context(context, Some(binding.space_id))?;
        self.store
            .update_draft_binding(
                provider_scope(context),
                binding_id,
                &context.actor_id,
                request,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn test_binding(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding_id: u64,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingServiceError> {
        let current = self.get_binding(context, binding_id).await?;
        validate_management_context(context, Some(current.space_id))?;
        let testing = self
            .store
            .begin_binding_test(
                provider_scope(context),
                binding_id,
                &context.actor_id,
                expected_version,
            )
            .await?;
        let credential = self
            .resolve_binding_credential(
                context,
                &testing,
                sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderOperation::Health,
            )
            .await?;
        let engine = self
            .registry
            .resolve_by_id(&testing.implementation_id)
            .map_err(KnowledgeEngineProviderBindingServiceError::Engine)?
            .bind_provider(&testing, credential)
            .map_err(KnowledgeEngineProviderBindingServiceError::Engine)?;
        let descriptor = engine.descriptor();
        let health = engine
            .health()
            .await
            .map_err(KnowledgeEngineProviderBindingServiceError::Engine)?;
        let (capabilities, error_category) = match health.status {
            KnowledgeEngineHealthStatus::Available => (descriptor.capabilities, None),
            KnowledgeEngineHealthStatus::Degraded | KnowledgeEngineHealthStatus::Unavailable => (
                Vec::new(),
                Some(KnowledgeEngineProviderErrorCategory::Unavailable),
            ),
        };
        self.store
            .record_binding_test_result(
                provider_scope(context),
                binding_id,
                RecordKnowledgeEngineProviderTestResult {
                    expected_version: testing.version,
                    capabilities,
                    error_category,
                    updated_by: context.actor_id.clone(),
                },
            )
            .await
            .map_err(Into::into)
    }

    pub async fn probe_active_bindings_health(
        &self,
        context: &KnowledgeEngineExecutionContext,
    ) -> Result<KnowledgeEngineProviderHealthSummary, KnowledgeEngineProviderBindingServiceError>
    {
        let mut implementation_ids = BTreeSet::new();
        let mut degraded = false;
        let mut cursor = None;

        loop {
            validate_management_context(context, None)?;
            let page = self
                .store
                .list_bindings(
                    provider_scope(context),
                    ListKnowledgeEngineProviderBindingsRequest {
                        space_id: None,
                        lifecycle_state: Some(KnowledgeEngineProviderBindingState::Active),
                        cursor: cursor.clone(),
                        page_size: Some(200),
                    },
                )
                .await?;

            for binding in &page.items {
                implementation_ids.insert(binding.implementation_id.clone());
            }
            let probe_results = stream::iter(page.items.into_iter().map(|binding| async move {
                validate_management_context(context, None)?;
                if !binding
                    .capability_snapshot
                    .contains(&KnowledgeEngineCapability::Health)
                {
                    return Ok(false);
                }
                let remaining = remaining_deadline(context)?;
                Ok(
                    tokio::time::timeout(remaining, self.probe_binding_health(context, &binding))
                        .await
                        .unwrap_or(false),
                )
            }))
            .buffer_unordered(MAX_CONCURRENT_PROVIDER_HEALTH_PROBES)
            .collect::<Vec<Result<bool, KnowledgeEngineProviderBindingServiceError>>>()
            .await;
            for result in probe_results {
                degraded |= !result?;
            }

            match page.next_cursor {
                Some(next_cursor) if cursor.as_deref() != Some(next_cursor.as_str()) => {
                    cursor = Some(next_cursor);
                }
                Some(_) => {
                    return Err(KnowledgeEngineProviderBindingServiceError::Internal(
                        "Provider binding pagination returned a repeated cursor".to_string(),
                    ));
                }
                None => break,
            }
        }

        Ok(KnowledgeEngineProviderHealthSummary {
            implementation_ids: implementation_ids.into_iter().collect(),
            degraded,
        })
    }

    async fn probe_binding_health(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding: &KnowledgeEngineProviderBinding,
    ) -> bool {
        let Ok(engine) = self.registry.resolve_by_id(&binding.implementation_id) else {
            return false;
        };
        let Ok(credential) = self
            .resolve_binding_credential(
                context,
                binding,
                sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderOperation::Health,
            )
            .await
        else {
            return false;
        };
        let Ok(engine) = engine.bind_provider(binding, credential) else {
            return false;
        };
        if !engine
            .descriptor()
            .supports(KnowledgeEngineCapability::Health)
        {
            return false;
        }
        matches!(
            engine.health().await,
            Ok(health) if health.status == KnowledgeEngineHealthStatus::Available
        )
    }

    async fn resolve_binding_credential(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding: &KnowledgeEngineProviderBinding,
        operation: sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderOperation,
    ) -> Result<
        Option<
            crate::ports::knowledge_provider_credential_resolver::KnowledgeEngineProviderCredential,
        >,
        KnowledgeEngineProviderBindingServiceError,
    > {
        let Some(credential_reference_id) = binding.credential_reference_id else {
            return Ok(None);
        };
        let reference = self
            .store
            .resolve_credential_reference(
                provider_scope(context),
                credential_reference_id,
                &binding.implementation_id,
            )
            .await?;
        self.credential_resolver
            .resolve(
                &KnowledgeEngineProviderCredentialAccessContext::for_binding(
                    context, binding, &reference, operation,
                ),
                &reference,
            )
            .await
            .map(Some)
            .map_err(Into::into)
    }

    pub async fn activate_binding(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding_id: u64,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingServiceError> {
        let binding = self.get_binding(context, binding_id).await?;
        validate_management_context(context, Some(binding.space_id))?;
        if !binding
            .capability_snapshot
            .contains(&KnowledgeEngineCapability::Health)
            || !binding
                .capability_snapshot
                .contains(&KnowledgeEngineCapability::Search)
        {
            return Err(
                KnowledgeEngineProviderBindingServiceError::InvalidLifecycle(
                    "Provider activation requires tested health and search capabilities"
                        .to_string(),
                ),
            );
        }
        self.store
            .activate_binding(
                provider_scope(context),
                binding_id,
                &context.actor_id,
                expected_version,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn disable_binding(
        &self,
        context: &KnowledgeEngineExecutionContext,
        binding_id: u64,
        expected_version: u64,
    ) -> Result<KnowledgeEngineProviderBinding, KnowledgeEngineProviderBindingServiceError> {
        let binding = self.get_binding(context, binding_id).await?;
        validate_management_context(context, Some(binding.space_id))?;
        self.store
            .disable_binding(
                provider_scope(context),
                binding_id,
                &context.actor_id,
                expected_version,
            )
            .await
            .map_err(Into::into)
    }

    fn require_executable_external_implementation(
        &self,
        implementation_id: &str,
    ) -> Result<(), KnowledgeEngineProviderBindingServiceError> {
        let engine = self
            .registry
            .resolve_by_id(implementation_id)
            .map_err(KnowledgeEngineProviderBindingServiceError::Engine)?;
        let descriptor = engine.descriptor();
        if descriptor.native
            || !descriptor.supports(KnowledgeEngineCapability::Health)
            || !descriptor.supports(KnowledgeEngineCapability::Search)
        {
            return Err(KnowledgeEngineProviderBindingServiceError::InvalidRequest(
                format!(
                    "implementation_id={implementation_id} is not an executable external Provider"
                ),
            ));
        }
        Ok(())
    }
}

fn validate_management_context(
    context: &KnowledgeEngineExecutionContext,
    space_id: Option<u64>,
) -> Result<(), KnowledgeEngineProviderBindingServiceError> {
    if context.tenant_id == 0 {
        return Err(
            KnowledgeEngineProviderBindingServiceError::PermissionDenied(
                "tenant scope is required".to_string(),
            ),
        );
    }
    if is_blank(Some(context.actor_id.as_str())) {
        return Err(
            KnowledgeEngineProviderBindingServiceError::PermissionDenied(
                "authenticated actor is required".to_string(),
            ),
        );
    }
    // Exact permission only. A wildcard grant such as `knowledge.*` must not reach Provider
    // credential rotation/revocation or binding cutover through any caller that forwards
    // request scopes directly; the HTTP guard already enforces the exact scope.
    let may_manage = context
        .permission_scope
        .iter()
        .any(|permission| permission == KNOWLEDGE_PLATFORM_MANAGE_PERMISSION);
    if !may_manage {
        return Err(
            KnowledgeEngineProviderBindingServiceError::PermissionDenied(format!(
                "{KNOWLEDGE_PLATFORM_MANAGE_PERMISSION} is required"
            )),
        );
    }
    if is_blank(Some(context.trace_id.as_str())) {
        return Err(KnowledgeEngineProviderBindingServiceError::InvalidRequest(
            "trace_id is required".to_string(),
        ));
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| KnowledgeEngineProviderBindingServiceError::Internal(error.to_string()))?
        .as_millis();
    if u128::from(context.deadline_unix_ms) <= now_ms {
        return Err(KnowledgeEngineProviderBindingServiceError::DeadlineExceeded);
    }
    if context.space_id != 0 {
        require_space_scope(context, context.space_id)?;
    }
    if let Some(space_id) = space_id {
        require_space_scope(context, space_id)?;
    }
    Ok(())
}

fn require_space_scope(
    context: &KnowledgeEngineExecutionContext,
    space_id: u64,
) -> Result<(), KnowledgeEngineProviderBindingServiceError> {
    if context.data_scope.allowed_space_ids.is_empty()
        || !context.data_scope.allowed_space_ids.contains(&space_id)
    {
        return Err(
            KnowledgeEngineProviderBindingServiceError::PermissionDenied(format!(
                "space_id={space_id} is outside the authenticated data scope"
            )),
        );
    }
    Ok(())
}

fn provider_scope(context: &KnowledgeEngineExecutionContext) -> KnowledgeEngineProviderScope {
    KnowledgeEngineProviderScope {
        tenant_id: context.tenant_id,
        organization_id: context.organization_id,
    }
}

fn remaining_deadline(
    context: &KnowledgeEngineExecutionContext,
) -> Result<Duration, KnowledgeEngineProviderBindingServiceError> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| KnowledgeEngineProviderBindingServiceError::Internal(error.to_string()))?
        .as_millis();
    let remaining_ms = u128::from(context.deadline_unix_ms)
        .checked_sub(now_ms)
        .filter(|value| *value > 0)
        .ok_or(KnowledgeEngineProviderBindingServiceError::DeadlineExceeded)?;
    Ok(Duration::from_millis(
        u64::try_from(remaining_ms).unwrap_or(u64::MAX),
    ))
}

#[derive(Debug, Error)]
pub enum KnowledgeEngineProviderBindingServiceError {
    #[error("invalid Provider binding request: {0}")]
    InvalidRequest(String),
    #[error("Provider binding permission denied: {0}")]
    PermissionDenied(String),
    #[error("Provider binding request deadline exceeded")]
    DeadlineExceeded,
    #[error("invalid Provider binding lifecycle: {0}")]
    InvalidLifecycle(String),
    #[error(transparent)]
    Store(#[from] KnowledgeEngineProviderBindingStoreError),
    #[error(transparent)]
    Engine(#[from] KnowledgeEngineError),
    #[error(transparent)]
    Credential(#[from] KnowledgeEngineProviderCredentialError),
    #[error("Provider binding internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_knowledgebase_contract::provider_binding::KnowledgeEngineDataScope;

    #[test]
    fn management_context_requires_permission_and_exact_space_scope() {
        let mut context = valid_context();
        context.permission_scope.clear();
        assert!(matches!(
            validate_management_context(&context, Some(42)),
            Err(KnowledgeEngineProviderBindingServiceError::PermissionDenied(_))
        ));

        let mut context = valid_context();
        context.data_scope.allowed_space_ids = vec![7];
        assert!(matches!(
            validate_management_context(&context, Some(42)),
            Err(KnowledgeEngineProviderBindingServiceError::PermissionDenied(_))
        ));
    }

    #[test]
    fn management_context_rejects_expired_deadline_before_store_access() {
        let mut context = valid_context();
        context.deadline_unix_ms = 1;
        assert!(matches!(
            validate_management_context(&context, Some(42)),
            Err(KnowledgeEngineProviderBindingServiceError::DeadlineExceeded)
        ));
    }

    fn valid_context() -> KnowledgeEngineExecutionContext {
        let deadline_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis() as u64
            + 60_000;
        KnowledgeEngineExecutionContext {
            tenant_id: 1,
            organization_id: 7,
            actor_id: "tenant-admin".to_string(),
            permission_scope: vec![KNOWLEDGE_PLATFORM_MANAGE_PERMISSION.to_string()],
            data_scope: KnowledgeEngineDataScope {
                allowed_space_ids: vec![42],
                allowed_source_ids: Vec::new(),
                allowed_document_ids: Vec::new(),
            },
            space_id: 42,
            binding_id: None,
            trace_id: "trace-provider-test".to_string(),
            deadline_unix_ms,
        }
    }
}
