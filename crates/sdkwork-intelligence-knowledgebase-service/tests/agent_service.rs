use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::agent::KnowledgeAgentService;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_agent_profile_store::{
    KnowledgeAgentProfileStore, KnowledgeAgentProfileStoreError,
};
use sdkwork_intelligence_knowledgebase_service::retrieval::{
    KnowledgeRetrievalExecutor, KnowledgeRetrievalServiceError,
};
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBinding, KnowledgeAgentBindingRequest, KnowledgeAgentKnowledgeMode,
    KnowledgeAgentProfile, KnowledgeAgentProfileRequest, KnowledgeAgentStatus,
    KnowledgeContextFragment, KnowledgeRetrievalBinding, KnowledgeRetrievalMethod,
    KnowledgeRetrievalRequest, KnowledgeRetrievalResult, KnowledgeRetrievalTrace,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[tokio::test]
async fn agent_service_creates_profile_and_multiple_knowledge_bindings() {
    let store = RecordingAgentProfileStore::default();
    let retrieval = RecordingRetrievalExecutor::default();
    let service = KnowledgeAgentService::new(&store, &retrieval);

    let profile = service
        .create_profile(profile_request("Support Agent"))
        .await
        .unwrap();
    let first_binding = service
        .create_binding(
            profile.profile_id,
            binding_request(profile.profile_id, 7, 20, true),
        )
        .await
        .unwrap();
    let second_binding = service
        .create_binding(
            profile.profile_id,
            binding_request(profile.profile_id, 9, 10, true),
        )
        .await
        .unwrap();

    let loaded = service.retrieve_profile(profile.profile_id).await.unwrap();

    assert_eq!(loaded.profile_id, 501);
    assert_eq!(loaded.name, "Support Agent");
    assert_eq!(loaded.model_provider_id, "provider.model.openai");
    assert_eq!(loaded.model_id, "gpt-4.1");
    assert_eq!(loaded.bindings, vec![first_binding, second_binding]);
}

#[tokio::test]
async fn retrieval_preview_uses_enabled_agent_bindings_and_profile_model_policy() {
    let store = RecordingAgentProfileStore::with_profile(profile(
        501,
        "Support Agent",
        vec![
            binding(601, 501, 7, 20, true),
            binding(602, 501, 9, 10, false),
        ],
    ));
    let retrieval = RecordingRetrievalExecutor::default();
    let service = KnowledgeAgentService::new(&store, &retrieval);

    let result = service
        .preview_retrieval(
            501,
            KnowledgeRetrievalRequest {
                tenant_id: 100001,
                actor_id: Some(30001),
                query: "enterprise renewal support".to_string(),
                retrieval_profile_id: None,
                bindings: vec![],
                methods: vec![],
                top_k: Some(4),
                include_citations: true,
                include_trace: true,
                context_budget_tokens: Some(1200),
                metadata: vec![],
            },
        )
        .await
        .unwrap();

    assert_eq!(result.retrieval_id, 701);
    assert_eq!(
        retrieval.requests(),
        vec![KnowledgeRetrievalRequest {
            tenant_id: 100001,
            actor_id: Some(30001),
            query: "enterprise renewal support".to_string(),
            retrieval_profile_id: Some(31),
            bindings: vec![KnowledgeRetrievalBinding {
                space_id: 7,
                source_filter: None,
                document_filter: None,
                priority: 20,
                top_k: Some(3),
                min_score: Some(0.75),
            }],
            methods: vec![KnowledgeRetrievalMethod::Hybrid],
            top_k: Some(4),
            include_citations: true,
            include_trace: true,
            context_budget_tokens: Some(1200),
            metadata: vec![],
        }]
    );
}

#[tokio::test]
async fn agent_service_rejects_binding_profile_mismatch() {
    let store = RecordingAgentProfileStore::default();
    let retrieval = RecordingRetrievalExecutor::default();
    let service = KnowledgeAgentService::new(&store, &retrieval);

    let error = service
        .create_binding(501, binding_request(999, 7, 20, true))
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("profile_id in request must match"));
}

#[derive(Default)]
struct RecordingAgentProfileStore {
    profile: Mutex<Option<KnowledgeAgentProfile>>,
    bindings: Mutex<HashMap<u64, Vec<KnowledgeAgentBinding>>>,
}

impl RecordingAgentProfileStore {
    fn with_profile(profile: KnowledgeAgentProfile) -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(profile.profile_id, profile.bindings.clone());
        Self {
            profile: Mutex::new(Some(profile)),
            bindings: Mutex::new(bindings),
        }
    }
}

#[async_trait]
impl KnowledgeAgentProfileStore for RecordingAgentProfileStore {
    async fn create_profile(
        &self,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        let profile = KnowledgeAgentProfile {
            profile_id: 501,
            tenant_id: request.tenant_id,
            name: request.name,
            description: request.description,
            system_instruction: request.system_instruction,
            model_provider_id: request.model_provider_id,
            model_id: request.model_id,
            model_parameters: request.model_parameters,
            retrieval_profile_id: request.retrieval_profile_id,
            citation_policy: request.citation_policy,
            memory_policy_ref: request.memory_policy_ref,
            tool_policy_ref: request.tool_policy_ref,
            answer_policy: request.answer_policy,
            knowledge_mode: request.knowledge_mode,
            agent_implementation_id: request.agent_implementation_id,
            status: request.status,
            bindings: vec![],
        };
        *self.profile.lock().unwrap() = Some(profile.clone());
        Ok(profile)
    }

    async fn retrieve_profile(
        &self,
        profile_id: u64,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        let mut profile = self
            .profile
            .lock()
            .unwrap()
            .clone()
            .ok_or(KnowledgeAgentProfileStoreError::NotFound(profile_id))?;
        profile.bindings = self.list_bindings(profile_id).await?;
        Ok(profile)
    }

    async fn update_profile(
        &self,
        profile_id: u64,
        request: KnowledgeAgentProfileRequest,
    ) -> Result<KnowledgeAgentProfile, KnowledgeAgentProfileStoreError> {
        let bindings = self.list_bindings(profile_id).await?;
        let profile = KnowledgeAgentProfile {
            profile_id,
            tenant_id: request.tenant_id,
            name: request.name,
            description: request.description,
            system_instruction: request.system_instruction,
            model_provider_id: request.model_provider_id,
            model_id: request.model_id,
            model_parameters: request.model_parameters,
            retrieval_profile_id: request.retrieval_profile_id,
            citation_policy: request.citation_policy,
            memory_policy_ref: request.memory_policy_ref,
            tool_policy_ref: request.tool_policy_ref,
            answer_policy: request.answer_policy,
            knowledge_mode: request.knowledge_mode,
            agent_implementation_id: request.agent_implementation_id,
            status: request.status,
            bindings,
        };
        *self.profile.lock().unwrap() = Some(profile.clone());
        Ok(profile)
    }

    async fn delete_profile(&self, profile_id: u64) -> Result<(), KnowledgeAgentProfileStoreError> {
        if self
            .profile
            .lock()
            .unwrap()
            .as_ref()
            .map(|profile| profile.profile_id == profile_id)
            .unwrap_or(false)
        {
            *self.profile.lock().unwrap() = None;
            self.bindings.lock().unwrap().remove(&profile_id);
            return Ok(());
        }
        Err(KnowledgeAgentProfileStoreError::NotFound(profile_id))
    }

    async fn list_bindings(
        &self,
        profile_id: u64,
    ) -> Result<Vec<KnowledgeAgentBinding>, KnowledgeAgentProfileStoreError> {
        Ok(self
            .bindings
            .lock()
            .unwrap()
            .get(&profile_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn create_binding(
        &self,
        request: KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
        let binding_id = if request.space_id == 7 { 601 } else { 602 };
        let binding = KnowledgeAgentBinding {
            binding_id,
            profile_id: request.profile_id,
            tenant_id: request.tenant_id,
            space_id: request.space_id,
            source_filter: request.source_filter,
            document_filter: request.document_filter,
            priority: request.priority,
            top_k: request.top_k,
            min_score: request.min_score,
            enabled: request.enabled,
        };
        self.bindings
            .lock()
            .unwrap()
            .entry(request.profile_id)
            .or_default()
            .push(binding.clone());
        Ok(binding)
    }

    async fn update_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
        request: KnowledgeAgentBindingRequest,
    ) -> Result<KnowledgeAgentBinding, KnowledgeAgentProfileStoreError> {
        let mut bindings = self.bindings.lock().unwrap();
        let profile_bindings = bindings.entry(profile_id).or_default();
        let binding = profile_bindings
            .iter_mut()
            .find(|binding| binding.binding_id == binding_id)
            .ok_or(KnowledgeAgentProfileStoreError::NotFound(binding_id))?;
        binding.space_id = request.space_id;
        binding.enabled = request.enabled;
        Ok(binding.clone())
    }

    async fn delete_binding(
        &self,
        profile_id: u64,
        binding_id: u64,
    ) -> Result<(), KnowledgeAgentProfileStoreError> {
        let mut bindings = self.bindings.lock().unwrap();
        let profile_bindings = bindings.entry(profile_id).or_default();
        profile_bindings.retain(|binding| binding.binding_id != binding_id);
        Ok(())
    }
}

#[derive(Default)]
struct RecordingRetrievalExecutor {
    requests: Mutex<Vec<KnowledgeRetrievalRequest>>,
}

impl RecordingRetrievalExecutor {
    fn requests(&self) -> Vec<KnowledgeRetrievalRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl KnowledgeRetrievalExecutor for RecordingRetrievalExecutor {
    async fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, KnowledgeRetrievalServiceError> {
        self.requests.lock().unwrap().push(request);
        Ok(KnowledgeRetrievalResult {
            retrieval_id: 701,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 701,
                status: "succeeded".to_string(),
                latency_ms: Some(9),
                result_count: 1,
            }),
            hits: vec![KnowledgeContextFragment {
                chunk_id: 11,
                document_id: 101,
                document_version_id: Some(201),
                space_id: 7,
                title: "Support Playbook".to_string(),
                content: "enterprise renewal support answer".to_string(),
                score: Some(0.91),
                rank: 1,
                token_count: Some(8),
                retrieval_method: KnowledgeRetrievalMethod::Hybrid,
                citation: None,
            }],
        })
    }
}

fn profile_request(name: &str) -> KnowledgeAgentProfileRequest {
    KnowledgeAgentProfileRequest {
        tenant_id: 100001,
        name: name.to_string(),
        description: Some("Answers from support knowledge bases.".to_string()),
        system_instruction: "Answer with citations.".to_string(),
        model_provider_id: "provider.model.openai".to_string(),
        model_id: "gpt-4.1".to_string(),
        model_parameters: Some(r#"{"temperature":0.2}"#.to_string()),
        retrieval_profile_id: Some(31),
        citation_policy: Some(r#"{"required":true}"#.to_string()),
        memory_policy_ref: Some("memory.short_term".to_string()),
        tool_policy_ref: Some("tools.read_only".to_string()),
        answer_policy: Some(r#"{"style":"concise"}"#.to_string()),
        knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        agent_implementation_id: sdkwork_knowledgebase_contract::default_agent_implementation_id(),
        status: KnowledgeAgentStatus::Active,
    }
}

fn binding_request(
    profile_id: u64,
    space_id: u64,
    priority: i32,
    enabled: bool,
) -> KnowledgeAgentBindingRequest {
    KnowledgeAgentBindingRequest {
        tenant_id: 100001,
        profile_id,
        space_id,
        source_filter: None,
        document_filter: None,
        priority,
        top_k: Some(3),
        min_score: Some(0.75),
        enabled,
    }
}

fn profile(
    profile_id: u64,
    name: &str,
    bindings: Vec<KnowledgeAgentBinding>,
) -> KnowledgeAgentProfile {
    KnowledgeAgentProfile {
        profile_id,
        tenant_id: 100001,
        name: name.to_string(),
        description: Some("Answers from support knowledge bases.".to_string()),
        system_instruction: "Answer with citations.".to_string(),
        model_provider_id: "provider.model.openai".to_string(),
        model_id: "gpt-4.1".to_string(),
        model_parameters: Some(r#"{"temperature":0.2}"#.to_string()),
        retrieval_profile_id: Some(31),
        citation_policy: Some(r#"{"required":true}"#.to_string()),
        memory_policy_ref: Some("memory.short_term".to_string()),
        tool_policy_ref: Some("tools.read_only".to_string()),
        answer_policy: Some(r#"{"style":"concise"}"#.to_string()),
        knowledge_mode: KnowledgeAgentKnowledgeMode::default(),
        agent_implementation_id: sdkwork_knowledgebase_contract::default_agent_implementation_id(),
        status: KnowledgeAgentStatus::Active,
        bindings,
    }
}

fn binding(
    binding_id: u64,
    profile_id: u64,
    space_id: u64,
    priority: i32,
    enabled: bool,
) -> KnowledgeAgentBinding {
    KnowledgeAgentBinding {
        binding_id,
        profile_id,
        tenant_id: 100001,
        space_id,
        source_filter: None,
        document_filter: None,
        priority,
        top_k: Some(3),
        min_score: Some(0.75),
        enabled,
    }
}
