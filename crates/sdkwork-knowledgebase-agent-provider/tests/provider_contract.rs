use sdkwork_agent_kernel::{
    KnowledgeDocument, KnowledgeDocumentFilter, KnowledgeDocumentKind, KnowledgeProvider,
    KnowledgeRetrievalMethod, KnowledgeSearchRequest, RedactionClassification, TrustLevel,
};
use sdkwork_knowledgebase_agent_provider::{
    KnowledgebaseRetrievalClient, SdkworkKnowledgebaseProvider, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID,
};
use sdkwork_knowledgebase_contract::{
    KnowledgeContextFragment, KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
    KnowledgeRetrievalTrace,
};

#[test]
fn provider_manifest_declares_standard_knowledge_capabilities() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let manifest = provider.provider_manifest();

    assert_eq!(manifest.provider_id, SDKWORK_KNOWLEDGEBASE_PROVIDER_ID);
    assert_eq!(manifest.provider_family, "knowledge");
    assert!(manifest
        .capabilities
        .contains(&"knowledge.search".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"knowledge.read".to_string()));
    assert!(manifest
        .capabilities
        .contains(&"knowledge.list".to_string()));
}

#[test]
fn search_maps_kernel_request_to_knowledgebase_retrieval_and_back() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let results = provider
        .search(
            KnowledgeSearchRequest::new("RAG boundary")
                .with_tenant_id("100001")
                .with_namespace("space:7")
                .with_top_k(3)
                .with_method(KnowledgeRetrievalMethod::Hybrid)
                .with_metadata("sdkwork.knowledge.retrieval_profile_id", "31"),
        )
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].document_id, "7/301");
    assert_eq!(results[0].title, "RAG Boundary");
    assert_eq!(
        results[0].retrieval_method,
        KnowledgeRetrievalMethod::Hybrid
    );
    assert_eq!(results[0].score, Some(0.91));
    assert_eq!(results[0].trust_level, TrustLevel::TrustedHost);
    assert_eq!(
        results[0].redaction_classification,
        RedactionClassification::TenantSensitive
    );
    assert_eq!(
        results[0]
            .metadata
            .iter()
            .find(|(key, _)| key == "sdkwork.knowledge.space_id")
            .map(|(_, value)| value.as_str()),
        Some("7")
    );
    assert_eq!(
        results[0]
            .metadata
            .iter()
            .find(|(key, _)| key == "sdkwork.knowledge.chunk_id")
            .map(|(_, value)| value.as_str()),
        Some("201")
    );
}

#[test]
fn search_requires_namespace_space_id_to_preserve_scope() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let error = provider
        .search(KnowledgeSearchRequest::new("missing scope"))
        .unwrap_err();

    assert_eq!(error.code(), "validation_error");
}

#[test]
fn read_and_list_delegate_to_typed_client() {
    let provider = SdkworkKnowledgebaseProvider::new(FakeKnowledgebaseClient, 100001);

    let document = provider.read("301").unwrap();
    let documents = provider
        .list(KnowledgeDocumentFilter::new().with_namespace("space:7"))
        .unwrap();

    assert_eq!(document.document_id, "301");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].namespace.as_deref(), Some("space:7"));
}

struct FakeKnowledgebaseClient;

impl KnowledgebaseRetrievalClient for FakeKnowledgebaseClient {
    fn retrieve(
        &self,
        request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, String> {
        assert_eq!(request.tenant_id, 100001);
        assert_eq!(request.bindings[0].space_id, 7);
        assert_eq!(request.bindings[0].top_k, Some(3));
        assert_eq!(request.retrieval_profile_id, Some(31));

        Ok(KnowledgeRetrievalResult {
            retrieval_id: 101,
            trace: Some(KnowledgeRetrievalTrace {
                retrieval_trace_id: 103,
                status: "succeeded".to_string(),
                latency_ms: Some(20),
                result_count: 1,
            }),
            hits: vec![KnowledgeContextFragment {
                chunk_id: 201,
                document_id: 301,
                document_version_id: Some(401),
                space_id: 7,
                title: "RAG Boundary".to_string(),
                content: "Knowledge retrieval is separate from model generation.".to_string(),
                score: Some(0.91),
                rank: 1,
                token_count: Some(8),
                retrieval_method: sdkwork_knowledgebase_contract::KnowledgeRetrievalMethod::Hybrid,
                citation: None,
            }],
        })
    }

    fn read_document(&self, document_id: &str) -> Result<KnowledgeDocument, String> {
        Ok(KnowledgeDocument::new(
            document_id,
            KnowledgeDocumentKind::Spec,
            "Knowledge SPI",
            "KnowledgeProvider search/read/list.",
        )
        .with_namespace("space:7"))
    }

    fn list_documents(
        &self,
        _filter: KnowledgeDocumentFilter,
    ) -> Result<Vec<KnowledgeDocument>, String> {
        Ok(vec![KnowledgeDocument::new(
            "301",
            KnowledgeDocumentKind::Spec,
            "Knowledge SPI",
            "KnowledgeProvider search/read/list.",
        )
        .with_namespace("space:7")])
    }
}
