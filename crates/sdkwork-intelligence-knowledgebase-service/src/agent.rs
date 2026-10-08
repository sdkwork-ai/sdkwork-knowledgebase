use crate::ports::knowledge_agent_profile_store::{
    KnowledgeAgentProfileStore, KnowledgeAgentProfileStoreError,
};
use crate::retrieval::{KnowledgeRetrievalExecutor, KnowledgeRetrievalServiceError};
use sdkwork_knowledgebase_agent_provider::{
    validate_rag_profile_requirements, validate_registered_agent_implementation,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentBindingList, KnowledgeAgentProfile,
    KnowledgeAgentProfileRequest, KnowledgeRetrievalBinding, KnowledgeRetrievalMethod,
    KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
};
use sdkwork_utils_rust::is_blank;
use thiserror::Error;

pub struct KnowledgeAgentService<'a> {
    profiles: &'a dyn KnowledgeAgentProfileStore,
    retrieval: &'a dyn KnowledgeRetrievalExecutor,
}

impl<'a> KnowledgeAgentService<'a> {
    pub fn new(
        profiles: &'a dyn KnowledgeAgentProfileStore,
        retrieval: &'a dyn KnowledgeRetrievalExecutor,
    ) -> Self {
        Self {
            profiles,
            retrieval,
        }
    }

    pub async fn create_profile(
        &self,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentServiceError> {
        validate_profile_request(&request)?;
        self.profiles
            .create_profile(request)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn retrieve_profile(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentServiceError> {
        validate_id("profile_id", profile_id)?;
        self.profiles
            .retrieve_profile(profile_id)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn update_profile(
        &self,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentServiceError> {
        validate_id("profile_id", profile_id)?;
        validate_profile_request(&request)?;
        self.profiles
            .update_profile(profile_id, request)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn delete_profile(&self, profile_id: u64) -> Result<(), KnowledgeAgentServiceError> {
        validate_id("profile_id", profile_id)?;
        self.profiles
            .delete_profile(profile_id)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn list_bindings(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeAgentBindingList, KnowledgeAgentServiceError> {
        validate_id("profile_id", profile_id)?;
        self.profiles
            .list_bindings(profile_id)
            .await
            .map(|items| KnowledgeAgentBindingList { items })
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn create_binding(
        &self,
        profile_id: u64,
        request: sdkwork_knowledgebase_contract::rag::KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentServiceError> {
        validate_binding_request(profile_id, &request)?;
        self.profiles
            .create_binding(request)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn update_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
        request: sdkwork_knowledgebase_contract::rag::KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentServiceError> {
        validate_binding_request(profile_id, &request)?;
        validate_id("binding_id", binding_id)?;
        self.profiles
            .update_binding(profile_id, binding_id, request)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn delete_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
    ) -> Result<(), KnowledgeAgentServiceError> {
        validate_id("profile_id", profile_id)?;
        validate_id("binding_id", binding_id)?;
        self.profiles
            .delete_binding(profile_id, binding_id)
            .await
            .map_err(KnowledgeAgentServiceError::Store)
    }

    pub async fn preview_retrieval(
        &self,
        profile_id: u64,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeAgentServiceError> {
        validate_id("profile_id", profile_id)?;
        let profile = self.retrieve_profile(profile_id).await?;
        if profile.tenant_id != request.tenant_id {
            return Err(KnowledgeAgentServiceError::InvalidRequest(
                "profile tenant_id must match retrieval request tenant_id".to_string(),
            ));
        }

        let bindings = profile
            .bindings
            .iter()
            .filter(|binding| binding.enabled)
            .map(binding_to_retrieval_binding)
            .collect::<Vec<_>>();
        if bindings.is_empty() {
            return Err(KnowledgeAgentServiceError::InvalidRequest(
                "agent profile requires at least one enabled knowledge binding".to_string(),
            ));
        }

        self.retrieval
            .retrieve(KnowledgeRetrievalRequest {
                retrieval_profile_id: request
                    .retrieval_profile_id
                    .or(profile.retrieval_profile_id),
                bindings,
                methods: if request.methods.is_empty() {
                    vec![KnowledgeRetrievalMethod::Hybrid]
                } else {
                    request.methods
                },
                ..request
            })
            .await
            .map_err(KnowledgeAgentServiceError::Retrieval)
    }
}

fn binding_to_retrieval_binding(binding: &KnowledgeAgentBinding) -> KnowledgeRetrievalBinding {
    KnowledgeRetrievalBinding {
        space_id: binding.space_id,
        source_filter: binding.source_filter.clone(),
        document_filter: binding.document_filter.clone(),
        priority: binding.priority,
        top_k: binding.top_k,
        min_score: binding.min_score,
    }
}

fn validate_profile_request(
    request: &KnowledgeAgentProfileRequest,
) -> Result<(), KnowledgeAgentServiceError> {
    if request.tenant_id == 0 {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "tenant_id is required".to_string(),
        ));
    }
    if is_blank(Some(request.name.as_str())) {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "name is required".to_string(),
        ));
    }
    if is_blank(Some(request.system_instruction.as_str())) {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "system_instruction is required".to_string(),
        ));
    }
    if is_blank(Some(request.model_provider_id.as_str())) {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "model_provider_id is required".to_string(),
        ));
    }
    if is_blank(Some(request.model_id.as_str())) {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "model_id is required".to_string(),
        ));
    }
    validate_rag_profile_requirements(request.knowledge_mode, request.retrieval_profile_id)
        .map_err(KnowledgeAgentServiceError::InvalidRequest)?;
    validate_registered_agent_implementation(&request.agent_implementation_id)
        .map_err(KnowledgeAgentServiceError::InvalidRequest)?;
    Ok(())
}

fn validate_binding_request(
    profile_id: u64,
    request: &sdkwork_knowledgebase_contract::rag::KnowledgeAgentBindingRequest,
) -> Result<(), KnowledgeAgentServiceError> {
    validate_id("profile_id", profile_id)?;
    if request.profile_id != profile_id {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "profile_id in request must match profile_id in path".to_string(),
        ));
    }
    if request.tenant_id == 0 {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "tenant_id is required".to_string(),
        ));
    }
    if request.space_id == 0 {
        return Err(KnowledgeAgentServiceError::InvalidRequest(
            "space_id is required".to_string(),
        ));
    }
    Ok(())
}

fn validate_id(field_name: &str, value: u64) -> Result<(), KnowledgeAgentServiceError> {
    if value == 0 {
        return Err(KnowledgeAgentServiceError::InvalidRequest(format!(
            "{field_name} is required"
        )));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum KnowledgeAgentServiceError {
    #[error("invalid knowledge agent request: {0}")]
    InvalidRequest(String),
    #[error(transparent)]
    Store(#[from] KnowledgeAgentProfileStoreError),
    #[error(transparent)]
    Retrieval(#[from] KnowledgeRetrievalServiceError),
}
