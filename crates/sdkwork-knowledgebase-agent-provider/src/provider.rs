use sdkwork_agent_kernel::{
    KernelError, KernelErrorSource, KernelResult, KnowledgeDocument, KnowledgeDocumentFilter,
    KnowledgeProvider, KnowledgeSearchRequest, KnowledgeSearchResult, ProviderHealth,
    ProviderManifest,
};
use sdkwork_utils_rust::is_blank;

use crate::client::KnowledgebaseRetrievalClient;
use crate::mapper::{hit_to_search_result, map_method, parse_namespace_space_id, parse_tenant_id};

pub const SDKWORK_KNOWLEDGEBASE_PROVIDER_ID: &str = "provider.knowledge.sdkwork-knowledgebase";

pub struct SdkworkKnowledgebaseProvider<C> {
    client: C,
    tenant_id: u64,
}

impl<C> SdkworkKnowledgebaseProvider<C> {
    pub fn new(client: C, tenant_id: u64) -> Self {
        Self { client, tenant_id }
    }
}

impl<C> KnowledgeProvider for SdkworkKnowledgebaseProvider<C>
where
    C: KnowledgebaseRetrievalClient,
{
    fn provider_manifest(&self) -> ProviderManifest {
        ProviderManifest::new(
            SDKWORK_KNOWLEDGEBASE_PROVIDER_ID,
            "knowledge",
            "sdkwork-knowledgebase-provider",
            env!("CARGO_PKG_VERSION"),
            vec![
                "knowledge.search".to_string(),
                "knowledge.read".to_string(),
                "knowledge.list".to_string(),
            ],
        )
    }

    fn search(&self, request: KnowledgeSearchRequest) -> KernelResult<Vec<KnowledgeSearchResult>> {
        let retrieval_request = self.to_retrieval_request(&request)?;
        let result = self.client.retrieve(retrieval_request).map_err(|message| {
            KernelError::provider_error("knowledgebase.retrieve_failed", message)
                .with_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
                .from_source(KernelErrorSource::Provider)
        })?;

        Ok(result.hits.into_iter().map(hit_to_search_result).collect())
    }

    fn read(&self, document_id: &str) -> KernelResult<KnowledgeDocument> {
        self.client.read_document(document_id).map_err(|message| {
            KernelError::provider_error("knowledgebase.read_failed", message)
                .with_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
                .from_source(KernelErrorSource::Provider)
        })
    }

    fn list(&self, filter: KnowledgeDocumentFilter) -> KernelResult<Vec<KnowledgeDocument>> {
        self.client.list_documents(filter).map_err(|message| {
            KernelError::provider_error("knowledgebase.list_failed", message)
                .with_provider(SDKWORK_KNOWLEDGEBASE_PROVIDER_ID)
                .from_source(KernelErrorSource::Provider)
        })
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::available()
    }
}

impl<C> SdkworkKnowledgebaseProvider<C>
where
    C: KnowledgebaseRetrievalClient,
{
    fn to_retrieval_request(
        &self,
        request: &KnowledgeSearchRequest,
    ) -> KernelResult<sdkwork_knowledgebase_contract::KnowledgeRetrievalRequest> {
        if is_blank(Some(request.query.as_str())) {
            return Err(KernelError::validation(
                "knowledge search query must not be blank",
            ));
        }

        Ok(sdkwork_knowledgebase_contract::KnowledgeRetrievalRequest {
            tenant_id: parse_tenant_id(request.tenant_id.as_deref()).unwrap_or(self.tenant_id),
            actor_id: None,
            query: request.query.clone(),
            retrieval_profile_id: request
                .metadata_value("sdkwork.knowledge.retrieval_profile_id")
                .and_then(|value| value.parse::<u64>().ok()),
            bindings: vec![sdkwork_knowledgebase_contract::KnowledgeRetrievalBinding {
                space_id: parse_namespace_space_id(request.namespace.as_deref())?,
                source_filter: None,
                document_filter: None,
                priority: 0,
                top_k: Some(request.top_k.min(u32::MAX as usize) as u32),
                min_score: request
                    .metadata_value("sdkwork.knowledge.min_score")
                    .and_then(|value| value.parse::<f64>().ok()),
            }],
            methods: request.methods.iter().copied().map(map_method).collect(),
            top_k: Some(request.top_k.min(u32::MAX as usize) as u32),
            include_citations: true,
            include_trace: true,
            context_budget_tokens: request
                .metadata_value("sdkwork.knowledge.context_budget_tokens")
                .and_then(|value| value.parse::<u32>().ok()),
            metadata: request
                .metadata
                .iter()
                .map(
                    |(key, value)| sdkwork_knowledgebase_contract::KnowledgeFilter {
                        key: key.clone(),
                        value: value.clone(),
                    },
                )
                .collect(),
        })
    }
}
