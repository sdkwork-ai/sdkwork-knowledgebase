use async_trait::async_trait;
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentBindingRequest, KnowledgeAgentProfile,
    KnowledgeAgentProfileRequest,
};
use thiserror::Error;

/// One keyset page of agent-profile bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeAgentBindingPage {
    pub items: Vec<KnowledgeAgentBinding>,
    /// Binding id to pass as the next cursor, present only when `has_more` is true.
    pub next_cursor: Option<u64>,
    pub has_more: bool,
}

#[async_trait]
pub trait KnowledgeAgentProfileStore: Send + Sync {
    async fn create_profile(
        &self,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError>;

    async fn retrieve_profile(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError>;

    async fn update_profile(
        &self,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError>;

    async fn delete_profile(&self, profile_id: u64) -> Result<(), KnowledgeAgentProfileStoreError>;

    async fn list_bindings(
        &self,
        profile_id: u64,
    ) -> Result<Vec<KnowledgeAgentBinding>, KnowledgeAgentProfileStoreError>;

    /// Keyset page of bindings ordered by binding id. Stores that cannot page fail closed;
    /// callers must not silently fall back to an unbounded or fixed page.
    async fn list_bindings_page(
        &self,
        profile_id: u64,
        cursor: Option<u64>,
        page_size: u32,
    ) -> Result<KnowledgeAgentBindingPage, KnowledgeAgentProfileStoreError> {
        let _ = (profile_id, cursor, page_size);
        Err(KnowledgeAgentProfileStoreError::Unsupported(
            "paged agent binding listing is unsupported by this store".to_string(),
        ))
    }

    async fn create_binding(
        &self,
        request: KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError>;

    async fn update_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError>;

    async fn delete_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
    ) -> Result<(), KnowledgeAgentProfileStoreError>;

    /// Resolve the highest-priority active agent profile bound to a knowledge space.
    ///
    /// Route crates use this instead of issuing raw SQL against
    /// `kb_agent_knowledge_binding`, keeping persistence concerns inside the
    /// repository layer.
    async fn resolve_profile_id_for_space(
        &self,
        tenant_id: u64,
        space_id: u64,
    ) -> Result<Option<u64>, KnowledgeAgentProfileStoreError> {
        let _ = (tenant_id, space_id);
        Err(KnowledgeAgentProfileStoreError::Unsupported(
            "resolve_profile_id_for_space is unsupported by this store".to_string(),
        ))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgeAgentProfileStoreError {
    #[error("knowledge agent profile invalid request: {0}")]
    InvalidRequest(String),
    #[error("knowledge agent profile not found: {0}")]
    NotFound(u64),
    #[error("knowledge agent profile store conflict: {0}")]
    Conflict(String),
    #[error("knowledge agent profile store unsupported operation: {0}")]
    Unsupported(String),
    #[error("knowledge agent profile store internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalProfileStore;

    #[async_trait]
    impl KnowledgeAgentProfileStore for MinimalProfileStore {
        async fn create_profile(
            &self,
            _request: KnowledgeAgentProfileRequest,
        ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile creation is unsupported by this store".to_string(),
            ))
        }

        async fn retrieve_profile(
            &self,
            profile_id: u64,
        ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::NotFound(profile_id))
        }

        async fn update_profile(
            &self,
            _profile_id: u64,
            _request: KnowledgeAgentProfileRequest,
        ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile update is unsupported by this store".to_string(),
            ))
        }

        async fn delete_profile(
            &self,
            _profile_id: u64,
        ) -> Result<(), KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile deletion is unsupported by this store".to_string(),
            ))
        }

        async fn list_bindings(
            &self,
            _profile_id: u64,
        ) -> Result<Vec<KnowledgeAgentBinding>, KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile binding listing is unsupported by this store".to_string(),
            ))
        }

        async fn create_binding(
            &self,
            _request: KnowledgeAgentBindingRequest,
        ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile binding creation is unsupported by this store".to_string(),
            ))
        }

        async fn update_binding(
            &self,
            _profile_id: u64,
            _binding_id: u64,
            _request: KnowledgeAgentBindingRequest,
        ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile binding update is unsupported by this store".to_string(),
            ))
        }

        async fn delete_binding(
            &self,
            _profile_id: u64,
            _binding_id: u64,
        ) -> Result<(), KnowledgeAgentProfileStoreError> {
            Err(KnowledgeAgentProfileStoreError::Unsupported(
                "profile binding deletion is unsupported by this store".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn default_profile_resolution_reports_unsupported_instead_of_internal_placeholder() {
        let error = MinimalProfileStore
            .resolve_profile_id_for_space(100001, 42)
            .await
            .expect_err("default profile resolution should be unsupported");

        match error {
            KnowledgeAgentProfileStoreError::Unsupported(detail) => {
                assert!(detail.contains("resolve_profile_id_for_space"));
                assert!(detail.contains("unsupported"));
            }
            other => panic!("expected unsupported default profile resolution error, got {other:?}"),
        }
    }
}
