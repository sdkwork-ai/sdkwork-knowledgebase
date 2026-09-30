use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineCapability, KnowledgeEngineDescriptor, KnowledgeEngineDocument,
    KnowledgeEngineDocumentList, KnowledgeEngineError, KnowledgeEngineListRequest,
    KnowledgeEngineProviderErrorCategory, KnowledgeEngineProviderFailure,
    KnowledgeEngineProviderOperation, KnowledgeEngineReadRequest, KnowledgeEngineSearchRequest,
    KnowledgeEngineSearchResult,
};
use sdkwork_knowledgebase_contract::provider_binding::{
    KnowledgeEngineExecutionContext, KnowledgeEngineProviderBinding,
    KnowledgeEngineProviderBindingState,
};
use sdkwork_utils_rust::is_blank;

use crate::ports::knowledge_engine::KnowledgeEngine;
use crate::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderBindingStoreError,
    KnowledgeEngineProviderScope,
};
use crate::ports::knowledge_provider_credential_resolver::{
    KnowledgeEngineProviderCredentialAccessContext, KnowledgeEngineProviderCredentialError,
    KnowledgeEngineProviderCredentialResolver,
};

#[derive(Clone)]
pub struct KnowledgeEngineExecutionHandle {
    engine: Arc<dyn KnowledgeEngine>,
    binding: Option<KnowledgeEngineProviderBinding>,
    provider_scope: KnowledgeEngineProviderScope,
    space_id: u64,
    binding_store: Option<Arc<dyn KnowledgeEngineProviderBindingStore>>,
    credential_resolver: Option<Arc<dyn KnowledgeEngineProviderCredentialResolver>>,
    /// Process-level cache for bound external engines. Sharing one cache
    /// across per-request handles is what keeps adapter ProviderRuntimes
    /// (circuit breaker, bulkhead, pinned client) alive between requests.
    bind_cache: Option<Arc<super::cached_binding_engine::ProviderBindingEngineCache>>,
}

impl KnowledgeEngineExecutionHandle {
    pub fn native(
        engine: Arc<dyn KnowledgeEngine>,
        provider_scope: KnowledgeEngineProviderScope,
        space_id: u64,
    ) -> Self {
        Self {
            engine,
            binding: None,
            provider_scope,
            space_id,
            binding_store: None,
            credential_resolver: None,
            bind_cache: None,
        }
    }

    pub fn external(
        engine: Arc<dyn KnowledgeEngine>,
        binding: KnowledgeEngineProviderBinding,
        provider_scope: KnowledgeEngineProviderScope,
        space_id: u64,
        binding_store: Option<Arc<dyn KnowledgeEngineProviderBindingStore>>,
        credential_resolver: Option<Arc<dyn KnowledgeEngineProviderCredentialResolver>>,
    ) -> Result<Self, KnowledgeEngineError> {
        if binding.lifecycle_state != KnowledgeEngineProviderBindingState::Active {
            return Err(KnowledgeEngineError::Validation(
                "external Provider execution requires an active binding".to_string(),
            ));
        }
        if binding.tenant_id != provider_scope.tenant_id
            || binding.organization_id != provider_scope.organization_id
            || binding.space_id != space_id
        {
            return Err(KnowledgeEngineError::PermissionDenied(
                "Provider binding is outside the resolver scope".to_string(),
            ));
        }
        Ok(Self {
            engine,
            binding: Some(binding),
            provider_scope,
            space_id,
            binding_store,
            credential_resolver,
            bind_cache: None,
        })
    }

    /// Attaches the shared process-level bound-engine cache. Call once on the
    /// per-request handle at construction time (see `KnowledgeEngineSpaceResolver`).
    pub fn with_bind_cache(
        mut self,
        bind_cache: Arc<super::cached_binding_engine::ProviderBindingEngineCache>,
    ) -> Self {
        self.bind_cache = Some(bind_cache);
        self
    }

    pub fn descriptor(&self) -> KnowledgeEngineDescriptor {
        self.engine.descriptor()
    }

    pub fn binding(&self) -> Option<&KnowledgeEngineProviderBinding> {
        self.binding.as_ref()
    }

    pub async fn search(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineSearchRequest,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        let context = self.scoped_context(
            context,
            request.tenant_id,
            request.space_id,
            KnowledgeEngineProviderOperation::Search,
        )?;
        let engine = self
            .engine_for_operation(&context, KnowledgeEngineProviderOperation::Search)
            .await?;
        engine.search(&context, request).await
    }

    pub async fn read_document(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineReadRequest,
    ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
        let context = self.scoped_context(
            context,
            request.tenant_id,
            request.space_id,
            KnowledgeEngineProviderOperation::Read,
        )?;
        let engine = self
            .engine_for_operation(&context, KnowledgeEngineProviderOperation::Read)
            .await?;
        engine.read_document(&context, request).await
    }

    pub async fn list_documents(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request: KnowledgeEngineListRequest,
    ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
        let context = self.scoped_context(
            context,
            request.tenant_id,
            request.space_id,
            KnowledgeEngineProviderOperation::List,
        )?;
        let engine = self
            .engine_for_operation(&context, KnowledgeEngineProviderOperation::List)
            .await?;
        engine.list_documents(&context, request).await
    }

    async fn engine_for_operation(
        &self,
        context: &KnowledgeEngineExecutionContext,
        operation: KnowledgeEngineProviderOperation,
    ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
        let Some(binding) = self.binding.as_ref() else {
            return Ok(self.engine.clone());
        };
        let capability = capability_for_operation(operation);
        if !binding.capability_snapshot.contains(&capability) {
            return Err(KnowledgeEngineError::Unsupported(format!(
                "active Provider binding_id={} has no tested {} capability",
                binding.id,
                capability_label(capability)
            )));
        }
        let credential_freshness = match binding.credential_reference_id {
            Some(credential_reference_id) => {
                let store = self.binding_store.as_ref().ok_or_else(|| {
                    credential_failure(
                        binding,
                        operation,
                        KnowledgeEngineProviderErrorCategory::Internal,
                        "Provider credential store is unavailable",
                    )
                })?;
                let reference = store
                    .resolve_credential_reference(
                        self.provider_scope,
                        credential_reference_id,
                        &binding.implementation_id,
                    )
                    .await
                    .map_err(|error| map_store_error(binding, operation, error))?;
                let resolver = self.credential_resolver.as_ref().ok_or_else(|| {
                    credential_failure(
                        binding,
                        operation,
                        KnowledgeEngineProviderErrorCategory::Internal,
                        "Provider credential resolver is unavailable",
                    )
                })?;
                let credential = resolver
                    .resolve(
                        &KnowledgeEngineProviderCredentialAccessContext::for_binding(
                            context, binding, &reference, operation,
                        ),
                        &reference,
                    )
                    .await
                    .map_err(|error| map_credential_error(binding, operation, error))?;
                Ok::<_, KnowledgeEngineError>((
                    Some(credential),
                    (
                        Some(reference.credential_reference_id),
                        Some(reference.version),
                    ),
                ))
            }
            None => Ok((None, (None, None))),
        }?;
        let (credential, (reference_id, reference_version)) = credential_freshness;
        // Reuse the bound adapter (and its ProviderRuntime) across requests via
        // the shared process-level cache; the credential reference version is
        // the freshness token, so rotation produces a fresh bind.
        let engine = match self.bind_cache.as_ref() {
            Some(cache) => {
                cache
                    .get_or_bind(
                        super::cached_binding_engine::ProviderBindingEngineKey {
                            implementation_id: binding.implementation_id.clone(),
                            binding_id: binding.id,
                            credential_reference_id: reference_id,
                            credential_reference_version: reference_version,
                            credential_bound: credential.is_some(),
                        },
                        || self.engine.bind_provider(binding, credential),
                    )?
            }
            None => self.engine.bind_provider(binding, credential)?,
        };
        if !engine.descriptor().supports(capability) {
            return Err(KnowledgeEngineError::Unsupported(format!(
                "Provider implementation_id={} no longer supports {}",
                binding.implementation_id,
                capability_label(capability)
            )));
        }
        Ok(engine)
    }

    fn scoped_context(
        &self,
        context: &KnowledgeEngineExecutionContext,
        request_tenant_id: u64,
        request_space_id: u64,
        operation: KnowledgeEngineProviderOperation,
    ) -> Result<KnowledgeEngineExecutionContext, KnowledgeEngineError> {
        if request_tenant_id == 0
            || request_tenant_id != context.tenant_id
            || request_tenant_id != self.provider_scope.tenant_id
        {
            return Err(KnowledgeEngineError::PermissionDenied(
                "Provider request tenant scope does not match the execution context".to_string(),
            ));
        }
        if context.organization_id != self.provider_scope.organization_id {
            return Err(KnowledgeEngineError::PermissionDenied(
                "Provider request organization scope does not match the resolver".to_string(),
            ));
        }
        if request_space_id == 0
            || request_space_id != context.space_id
            || request_space_id != self.space_id
            || !context
                .data_scope
                .allowed_space_ids
                .contains(&request_space_id)
        {
            return Err(KnowledgeEngineError::PermissionDenied(
                "Provider request space is outside the authenticated data scope".to_string(),
            ));
        }
        if is_blank(Some(context.actor_id.as_str())) {
            return Err(KnowledgeEngineError::PermissionDenied(
                "authenticated actor is required for Provider execution".to_string(),
            ));
        }
        let may_read = context.permission_scope.iter().any(|permission| {
            matches!(
                permission.as_str(),
                "knowledge.read"
                    | "knowledge.retrieval.read"
                    | "knowledge.platform.manage"
                    | "knowledge.*"
            )
        });
        if !may_read {
            return Err(KnowledgeEngineError::PermissionDenied(
                "knowledge read permission is required for Provider execution".to_string(),
            ));
        }
        if is_blank(Some(context.trace_id.as_str())) {
            return Err(KnowledgeEngineError::Validation(
                "Provider execution trace_id is required".to_string(),
            ));
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?
            .as_millis();
        if u128::from(context.deadline_unix_ms) <= now_ms {
            return Err(KnowledgeEngineError::Provider(
                sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderFailure {
                    category: sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory::Timeout,
                    operation,
                    implementation_id: self.descriptor().implementation_id,
                    binding_id: self.binding.as_ref().map(|binding| binding.id.to_string()),
                    status_code: None,
                    retryable: false,
                    retry_after_ms: None,
                    safe_message: "Provider request deadline has expired".to_string(),
                },
            ));
        }

        let mut scoped = context.clone();
        match &self.binding {
            Some(binding) => {
                if let Some(binding_id) = context.binding_id {
                    if binding_id != binding.id {
                        return Err(KnowledgeEngineError::PermissionDenied(
                            "Provider binding does not match the execution handle".to_string(),
                        ));
                    }
                }
                scoped.binding_id = Some(binding.id);
            }
            None => {
                if context.binding_id.is_some() {
                    return Err(KnowledgeEngineError::Validation(
                        "native knowledge execution must not carry a Provider binding".to_string(),
                    ));
                }
            }
        }
        Ok(scoped)
    }
}

fn capability_for_operation(
    operation: KnowledgeEngineProviderOperation,
) -> KnowledgeEngineCapability {
    match operation {
        KnowledgeEngineProviderOperation::Search => KnowledgeEngineCapability::Search,
        KnowledgeEngineProviderOperation::Read => KnowledgeEngineCapability::ReadDocument,
        KnowledgeEngineProviderOperation::List => KnowledgeEngineCapability::ListDocuments,
        KnowledgeEngineProviderOperation::Health => KnowledgeEngineCapability::Health,
        KnowledgeEngineProviderOperation::Ingest => KnowledgeEngineCapability::Ingest,
        KnowledgeEngineProviderOperation::Sync => KnowledgeEngineCapability::SyncSources,
    }
}

fn capability_label(capability: KnowledgeEngineCapability) -> &'static str {
    match capability {
        KnowledgeEngineCapability::Health => "health",
        KnowledgeEngineCapability::Search => "search",
        KnowledgeEngineCapability::ReadDocument => "read_document",
        KnowledgeEngineCapability::ListDocuments => "list_documents",
        KnowledgeEngineCapability::Ingest => "ingest",
        KnowledgeEngineCapability::SyncSources => "sync_sources",
    }
}

fn map_store_error(
    binding: &KnowledgeEngineProviderBinding,
    operation: KnowledgeEngineProviderOperation,
    error: KnowledgeEngineProviderBindingStoreError,
) -> KnowledgeEngineError {
    let (category, message) = match error {
        KnowledgeEngineProviderBindingStoreError::CredentialUnavailable(_) => (
            KnowledgeEngineProviderErrorCategory::Authentication,
            "Provider credential reference is unavailable",
        ),
        _ => (
            KnowledgeEngineProviderErrorCategory::Internal,
            "Provider credential reference lookup failed",
        ),
    };
    credential_failure(binding, operation, category, message)
}

fn map_credential_error(
    binding: &KnowledgeEngineProviderBinding,
    operation: KnowledgeEngineProviderOperation,
    error: KnowledgeEngineProviderCredentialError,
) -> KnowledgeEngineError {
    let (category, message) = match error {
        KnowledgeEngineProviderCredentialError::InvalidReference => (
            KnowledgeEngineProviderErrorCategory::Validation,
            "Provider credential reference is invalid",
        ),
        KnowledgeEngineProviderCredentialError::AccessDenied => (
            KnowledgeEngineProviderErrorCategory::PermissionDenied,
            "Provider credential access is denied",
        ),
        KnowledgeEngineProviderCredentialError::ResponseTooLarge => (
            KnowledgeEngineProviderErrorCategory::ResponseTooLarge,
            "Provider credential exceeds the permitted size",
        ),
        KnowledgeEngineProviderCredentialError::Unavailable
        | KnowledgeEngineProviderCredentialError::Internal => (
            KnowledgeEngineProviderErrorCategory::Authentication,
            "Provider credential is unavailable",
        ),
    };
    credential_failure(binding, operation, category, message)
}

fn credential_failure(
    binding: &KnowledgeEngineProviderBinding,
    operation: KnowledgeEngineProviderOperation,
    category: KnowledgeEngineProviderErrorCategory,
    safe_message: &str,
) -> KnowledgeEngineError {
    KnowledgeEngineError::Provider(KnowledgeEngineProviderFailure {
        category,
        operation,
        implementation_id: binding.implementation_id.clone(),
        binding_id: Some(binding.id.to_string()),
        status_code: None,
        retryable: false,
        retry_after_ms: None,
        safe_message: safe_message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use async_trait::async_trait;
    use sdkwork_knowledgebase_contract::knowledge_engine::{
        descriptor_for_external_search_read, KnowledgeEngineDocumentList, KnowledgeEngineHealth,
        KnowledgeEngineHealthStatus, KnowledgeEngineListRequest,
    };
    use sdkwork_knowledgebase_contract::provider_binding::{
        KnowledgeEngineDataScope, KnowledgeEngineProviderBindingState,
    };

    use super::*;

    #[derive(Clone, Default)]
    struct RecordingEngine {
        bind_count: Arc<AtomicUsize>,
        search_contexts: Arc<Mutex<Vec<KnowledgeEngineExecutionContext>>>,
        list_contexts: Arc<Mutex<Vec<KnowledgeEngineExecutionContext>>>,
    }

    #[async_trait]
    impl KnowledgeEngine for RecordingEngine {
        fn descriptor(&self) -> KnowledgeEngineDescriptor {
            let mut descriptor = descriptor_for_external_search_read("test", "Test Provider");
            descriptor
                .capabilities
                .push(KnowledgeEngineCapability::ListDocuments);
            descriptor
        }

        fn bind_provider(
            &self,
            _binding: &KnowledgeEngineProviderBinding,
            _credential: Option<crate::ports::knowledge_provider_credential_resolver::KnowledgeEngineProviderCredential>,
        ) -> Result<Arc<dyn KnowledgeEngine>, KnowledgeEngineError> {
            self.bind_count.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(self.clone()))
        }

        async fn health(&self) -> Result<KnowledgeEngineHealth, KnowledgeEngineError> {
            Ok(KnowledgeEngineHealth {
                implementation_id: "engine.knowledge.external.test".to_string(),
                status: KnowledgeEngineHealthStatus::Available,
                detail: None,
            })
        }

        async fn search(
            &self,
            context: &KnowledgeEngineExecutionContext,
            _request: KnowledgeEngineSearchRequest,
        ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
            self.search_contexts
                .lock()
                .expect("recording lock")
                .push(context.clone());
            Ok(KnowledgeEngineSearchResult {
                implementation_id: "engine.knowledge.external.test".to_string(),
                hits: Vec::new(),
            })
        }

        async fn read_document(
            &self,
            _context: &KnowledgeEngineExecutionContext,
            _request: KnowledgeEngineReadRequest,
        ) -> Result<KnowledgeEngineDocument, KnowledgeEngineError> {
            Err(KnowledgeEngineError::Unsupported(
                "not needed by execution handle tests".to_string(),
            ))
        }

        async fn list_documents(
            &self,
            context: &KnowledgeEngineExecutionContext,
            _request: KnowledgeEngineListRequest,
        ) -> Result<KnowledgeEngineDocumentList, KnowledgeEngineError> {
            self.list_contexts
                .lock()
                .expect("recording lock")
                .push(context.clone());
            Ok(KnowledgeEngineDocumentList { items: Vec::new() })
        }
    }

    #[tokio::test]
    async fn handle_injects_binding_and_preserves_trace() {
        let engine = Arc::new(RecordingEngine::default());
        let handle = external_handle(engine.clone());
        let context = execution_context();

        handle
            .search(
                &context,
                KnowledgeEngineSearchRequest {
                    tenant_id: 11,
                    space_id: 42,
                    query: "policy".to_string(),
                    top_k: 3,
                },
            )
            .await
            .expect("authorized search");

        let recorded = engine.search_contexts.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].binding_id, Some(73));
        assert_eq!(recorded[0].trace_id, "trace-execution-handle");
        assert_eq!(recorded[0].actor_id, "actor-execution-handle");
        assert_eq!(engine.bind_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn handle_rejects_scope_binding_and_deadline_before_engine() {
        let engine = Arc::new(RecordingEngine::default());
        let handle = external_handle(engine.clone());

        let mut wrong_tenant = execution_context();
        wrong_tenant.tenant_id = 99;
        assert!(matches!(
            search(&handle, &wrong_tenant).await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let mut wrong_organization = execution_context();
        wrong_organization.organization_id = 99;
        assert!(matches!(
            search(&handle, &wrong_organization).await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let mut wrong_space = execution_context();
        wrong_space.space_id = 99;
        assert!(matches!(
            search(&handle, &wrong_space).await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let mut missing_actor = execution_context();
        missing_actor.actor_id.clear();
        assert!(matches!(
            search(&handle, &missing_actor).await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let mut missing_permission = execution_context();
        missing_permission.permission_scope.clear();
        assert!(matches!(
            search(&handle, &missing_permission).await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let mut wrong_binding = execution_context();
        wrong_binding.binding_id = Some(999);
        assert!(matches!(
            search(&handle, &wrong_binding).await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let mut expired = execution_context();
        expired.deadline_unix_ms = 1;
        assert!(matches!(
            search(&handle, &expired).await,
            Err(KnowledgeEngineError::Provider(failure))
                if failure.category
                    == sdkwork_knowledgebase_contract::knowledge_engine::KnowledgeEngineProviderErrorCategory::Timeout
        ));

        let mut missing_trace = execution_context();
        missing_trace.trace_id.clear();
        assert!(matches!(
            search(&handle, &missing_trace).await,
            Err(KnowledgeEngineError::Validation(_))
        ));

        assert_eq!(engine.bind_count.load(Ordering::SeqCst), 0);
        assert!(engine
            .search_contexts
            .lock()
            .expect("recording lock")
            .is_empty());
    }

    #[tokio::test]
    async fn handle_validates_list_scope_and_preserves_binding_context() {
        let engine = Arc::new(RecordingEngine::default());
        let handle = external_handle(engine.clone());
        let context = execution_context();

        handle
            .list_documents(
                &context,
                KnowledgeEngineListRequest {
                    tenant_id: 11,
                    space_id: 42,
                    limit: 10,
                },
            )
            .await
            .expect("authorized list");

        let mut wrong_space = context;
        wrong_space.space_id = 99;
        assert!(matches!(
            handle
                .list_documents(
                    &wrong_space,
                    KnowledgeEngineListRequest {
                        tenant_id: 11,
                        space_id: 42,
                        limit: 10,
                    },
                )
                .await,
            Err(KnowledgeEngineError::PermissionDenied(_))
        ));

        let recorded = engine.list_contexts.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].binding_id, Some(73));
        assert_eq!(recorded[0].trace_id, "trace-execution-handle");
        assert_eq!(engine.bind_count.load(Ordering::SeqCst), 1);
    }

    async fn search(
        handle: &KnowledgeEngineExecutionHandle,
        context: &KnowledgeEngineExecutionContext,
    ) -> Result<KnowledgeEngineSearchResult, KnowledgeEngineError> {
        handle
            .search(
                context,
                KnowledgeEngineSearchRequest {
                    tenant_id: 11,
                    space_id: 42,
                    query: "policy".to_string(),
                    top_k: 3,
                },
            )
            .await
    }

    fn external_handle(engine: Arc<RecordingEngine>) -> KnowledgeEngineExecutionHandle {
        KnowledgeEngineExecutionHandle::external(
            engine,
            active_binding(),
            KnowledgeEngineProviderScope {
                tenant_id: 11,
                organization_id: 12,
            },
            42,
            None,
            None,
        )
        .expect("active binding handle")
    }

    fn execution_context() -> KnowledgeEngineExecutionContext {
        let now_ms = sdkwork_utils_rust::to_unix_millis(sdkwork_utils_rust::now());
        KnowledgeEngineExecutionContext {
            tenant_id: 11,
            organization_id: 12,
            actor_id: "actor-execution-handle".to_string(),
            permission_scope: vec!["knowledge.read".to_string()],
            data_scope: KnowledgeEngineDataScope {
                allowed_space_ids: vec![42],
                allowed_source_ids: Vec::new(),
                allowed_document_ids: Vec::new(),
            },
            space_id: 42,
            binding_id: None,
            trace_id: "trace-execution-handle".to_string(),
            deadline_unix_ms: u64::try_from(now_ms).expect("test clock") + 60_000,
        }
    }

    fn active_binding() -> KnowledgeEngineProviderBinding {
        KnowledgeEngineProviderBinding {
            id: 73,
            uuid: "binding-73".to_string(),
            tenant_id: 11,
            organization_id: 12,
            space_id: 42,
            implementation_id: "engine.knowledge.external.test".to_string(),
            remote_resource_type: "dataset".to_string(),
            remote_resource_id: "dataset-42".to_string(),
            credential_reference_id: None,
            lifecycle_state: KnowledgeEngineProviderBindingState::Active,
            capability_snapshot: vec![
                KnowledgeEngineCapability::Search,
                KnowledgeEngineCapability::ListDocuments,
            ],
            capability_snapshot_version: 1,
            last_tested_at: Some("2026-07-20T00:00:00Z".to_string()),
            activated_at: Some("2026-07-20T00:00:01Z".to_string()),
            disabled_at: None,
            last_error_category: None,
            created_by: "actor-execution-handle".to_string(),
            updated_by: "actor-execution-handle".to_string(),
            created_at: "2026-07-20T00:00:00Z".to_string(),
            updated_at: "2026-07-20T00:00:01Z".to_string(),
            version: 2,
        }
    }
}
