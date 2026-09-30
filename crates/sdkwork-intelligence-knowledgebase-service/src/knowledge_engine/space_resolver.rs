use sdkwork_knowledgebase_contract::knowledge_engine::{
    KnowledgeEngineCapability, KnowledgeEngineError,
};
use sdkwork_knowledgebase_contract::rag::KnowledgeAgentKnowledgeMode;
use std::sync::Arc;

use crate::knowledge_engine::KnowledgeEngineExecutionHandle;
use crate::knowledge_engine::cached_binding_engine::ProviderBindingEngineCache;
use crate::ports::knowledge_engine::{KnowledgeEngineRegistry, KnowledgeEngineSpaceRegistry};
use crate::ports::knowledge_provider_binding_store::{
    KnowledgeEngineProviderBindingStore, KnowledgeEngineProviderScope,
};
use crate::ports::knowledge_provider_credential_resolver::KnowledgeEngineProviderCredentialResolver;
use crate::ports::knowledge_space_store::KnowledgeSpaceStore;

pub struct KnowledgeEngineSpaceResolver<R> {
    registry: Arc<R>,
    space_store: Arc<dyn KnowledgeSpaceStore>,
    provider_binding_store: Arc<dyn KnowledgeEngineProviderBindingStore>,
    provider_scope: KnowledgeEngineProviderScope,
    credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
    /// Process-lifetime cache of bound external engines; shared by every
    /// per-request execution handle created by this resolver so adapter
    /// ProviderRuntimes (circuit breaker, bulkhead, pinned client) persist.
    bind_cache: Arc<ProviderBindingEngineCache>,
}

impl<R> KnowledgeEngineSpaceResolver<R>
where
    R: KnowledgeEngineRegistry + 'static,
{
    pub fn new(
        registry: Arc<R>,
        space_store: Arc<dyn KnowledgeSpaceStore>,
        provider_binding_store: Arc<dyn KnowledgeEngineProviderBindingStore>,
        provider_scope: KnowledgeEngineProviderScope,
        credential_resolver: Arc<dyn KnowledgeEngineProviderCredentialResolver>,
    ) -> Self {
        Self {
            registry,
            space_store,
            provider_binding_store,
            provider_scope,
            credential_resolver,
            bind_cache: Arc::new(ProviderBindingEngineCache::with_default_capacity()),
        }
    }

    pub async fn resolve_for_space(
        &self,
        space_id: u64,
        mode_override: Option<KnowledgeAgentKnowledgeMode>,
    ) -> Result<KnowledgeEngineExecutionHandle, KnowledgeEngineError> {
        let space = self
            .space_store
            .get_space(space_id)
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?;

        let mode = mode_override.unwrap_or(space.knowledge_mode);
        if mode == KnowledgeAgentKnowledgeMode::External {
            return self.resolve_external_for_space(space_id).await;
        }

        self.registry.resolve_for_mode(mode).map(|engine| {
            KnowledgeEngineExecutionHandle::native(engine, self.provider_scope, space_id)
        })
    }

    async fn resolve_external_for_space(
        &self,
        space_id: u64,
    ) -> Result<KnowledgeEngineExecutionHandle, KnowledgeEngineError> {
        let binding = self
            .provider_binding_store
            .get_active_binding_for_space(self.provider_scope, space_id)
            .await
            .map_err(|error| KnowledgeEngineError::Internal(error.to_string()))?
            .ok_or_else(|| {
                KnowledgeEngineError::NotFound(format!(
                    "no active external Provider binding for space_id={space_id}"
                ))
            })?;
        if !binding
            .capability_snapshot
            .contains(&KnowledgeEngineCapability::Search)
        {
            return Err(KnowledgeEngineError::Unsupported(format!(
                "active Provider binding_id={} has no tested search capability",
                binding.id
            )));
        }

        let engine = self.registry.resolve_by_id(&binding.implementation_id)?;
        Ok(KnowledgeEngineExecutionHandle::external(
            engine,
            binding,
            self.provider_scope,
            space_id,
            Some(self.provider_binding_store.clone()),
            Some(self.credential_resolver.clone()),
        )?
        .with_bind_cache(self.bind_cache.clone()))
    }
}

#[async_trait::async_trait]
impl<R> KnowledgeEngineSpaceRegistry for KnowledgeEngineSpaceResolver<R>
where
    R: KnowledgeEngineRegistry + 'static,
{
    async fn resolve_for_space(
        &self,
        space_id: u64,
        mode_override: Option<KnowledgeAgentKnowledgeMode>,
    ) -> Result<KnowledgeEngineExecutionHandle, KnowledgeEngineError> {
        KnowledgeEngineSpaceResolver::resolve_for_space(self, space_id, mode_override).await
    }
}
