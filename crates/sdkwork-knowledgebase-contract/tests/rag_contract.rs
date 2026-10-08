use sdkwork_knowledgebase_contract::{
    KnowledgeAgentBinding, KnowledgeAgentProfile, KnowledgeAgentStatus, KnowledgeCitation,
    KnowledgeContextFragment, KnowledgeContextPack, KnowledgeContextPackRequest, KnowledgeFilter,
    KnowledgeMemoryContextFragment, KnowledgeRetrievalBinding, KnowledgeRetrievalMethod,
    KnowledgeRetrievalRequest, KnowledgeRetrievalResult, KnowledgeRetrievalTrace,
};

#[test]
fn retrieval_request_supports_multi_space_hybrid_rag() {
    let request = KnowledgeRetrievalRequest {
        tenant_id: 100001,
        actor_id: Some(1001),
        query: "How does SDKWork Knowledgebase expose RAG?".to_string(),
        retrieval_profile_id: Some(31),
        bindings: vec![
            KnowledgeRetrievalBinding {
                space_id: 7,
                source_filter: Some(vec![KnowledgeFilter {
                    key: "sourceType".to_string(),
                    value: "drive".to_string(),
                }]),
                document_filter: None,
                priority: 10,
                top_k: Some(8),
                min_score: Some(0.62),
            },
            KnowledgeRetrievalBinding {
                space_id: 11,
                source_filter: None,
                document_filter: Some(vec![KnowledgeFilter {
                    key: "language".to_string(),
                    value: "en".to_string(),
                }]),
                priority: 20,
                top_k: Some(4),
                min_score: None,
            },
        ],
        methods: vec![
            KnowledgeRetrievalMethod::Keyword,
            KnowledgeRetrievalMethod::Vector,
            KnowledgeRetrievalMethod::Hybrid,
            KnowledgeRetrievalMethod::LlmRerank,
        ],
        top_k: Some(12),
        include_citations: true,
        include_trace: true,
        context_budget_tokens: Some(2048),
        metadata: vec![KnowledgeFilter {
            key: "sdkwork.agent.profileId".to_string(),
            value: "41".to_string(),
        }],
    };

    let json = serde_json::to_value(&request).unwrap();

    assert_eq!(json["tenantId"], "100001");
    assert_eq!(json["retrievalProfileId"], "31");
    assert_eq!(json["bindings"].as_array().unwrap().len(), 2);
    assert_eq!(json["methods"][2], "hybrid");
    assert_eq!(json["methods"][3], "llm_rerank");
    assert_eq!(json["includeCitations"], true);
    assert_eq!(json["contextBudgetTokens"], 2048);
}

#[test]
fn retrieval_result_preserves_citation_and_trace_identity() {
    let result = KnowledgeRetrievalResult {
        retrieval_id: 101,
        trace: Some(KnowledgeRetrievalTrace {
            retrieval_trace_id: 103,
            status: "succeeded".to_string(),
            latency_ms: Some(84),
            result_count: 1,
        }),
        hits: vec![KnowledgeContextFragment {
            chunk_id: 201,
            document_id: 301,
            document_version_id: Some(401),
            space_id: 7,
            title: "Knowledge Provider SPI".to_string(),
            content: "Knowledge retrieval stays separate from model generation.".to_string(),
            score: Some(0.91),
            rank: 1,
            token_count: Some(16),
            retrieval_method: KnowledgeRetrievalMethod::Hybrid,
            citation: Some(KnowledgeCitation {
                document_id: 301,
                document_version_id: Some(401),
                chunk_id: Some(201),
                title: "Knowledge Provider SPI".to_string(),
                source_uri: Some("drive://space/doc.md".to_string()),
                locator: Some("section=rag-boundary".to_string()),
                score: Some(0.91),
            }),
        }],
    };

    let json = serde_json::to_value(&result).unwrap();

    assert_eq!(json["retrievalId"], "101");
    assert_eq!(json["trace"]["retrievalTraceId"], "103");
    assert_eq!(json["hits"][0]["chunkId"], "201");
    assert_eq!(json["hits"][0]["citation"]["documentVersionId"], "401");
    assert_eq!(json["hits"][0]["retrievalMethod"], "hybrid");
}

#[test]
fn context_pack_is_a_bounded_prompt_input_not_a_model_answer() {
    let request = KnowledgeContextPackRequest {
        tenant_id: 100001,
        actor_id: Some(1001),
        query: "agent memory boundaries".to_string(),
        retrieval_profile_id: Some(31),
        bindings: vec![KnowledgeRetrievalBinding {
            space_id: 7,
            source_filter: None,
            document_filter: None,
            priority: 10,
            top_k: Some(6),
            min_score: Some(0.4),
        }],
        context_budget_tokens: 1200,
        include_citations: true,
        memory_policy_ref: Some("memory.session.summary".to_string()),
    };

    let pack = KnowledgeContextPack {
        context_pack_id: 501,
        retrieval_id: Some(101),
        query: request.query.clone(),
        fragments: vec![KnowledgeContextFragment {
            chunk_id: 201,
            document_id: 301,
            document_version_id: None,
            space_id: 7,
            title: "Agent Memory".to_string(),
            content: "Memory and knowledge are separate agent inputs.".to_string(),
            score: Some(0.88),
            rank: 1,
            token_count: Some(9),
            retrieval_method: KnowledgeRetrievalMethod::Keyword,
            citation: None,
        }],
        memory_fragments: vec![KnowledgeMemoryContextFragment {
            memory_id: "mem-001".to_string(),
            title: Some("Session preference".to_string()),
            content: "The user prefers concise operational answers.".to_string(),
            score: Some(0.83),
            rank: 1,
            token_count: Some(7),
            source_uri: Some("memory://mem-001".to_string()),
            policy_ref: Some("memory.session.summary".to_string()),
        }],
        estimated_tokens: 9,
        citations: vec![],
        truncated: false,
    };

    let request_json = serde_json::to_value(&request).unwrap();
    let pack_json = serde_json::to_value(&pack).unwrap();

    assert_eq!(request_json["memoryPolicyRef"], "memory.session.summary");
    assert_eq!(pack_json["memoryFragments"][0]["memoryId"], "mem-001");
    assert_eq!(
        pack_json["memoryFragments"][0]["sourceUri"],
        "memory://mem-001"
    );
    assert_eq!(pack_json["fragments"][0]["chunkId"], "201");
    assert!(pack_json["memoryFragments"][0].get("chunkId").is_none());
    assert_eq!(request.context_budget_tokens, 1200);
    assert_eq!(pack.estimated_tokens, 9);
    assert!(pack.fragments[0].content.contains("separate agent inputs"));
}

#[test]
fn knowledge_agent_profile_selects_model_provider_and_multiple_knowledge_bindings() {
    let profile = KnowledgeAgentProfile {
        profile_id: 41,
        tenant_id: 100001,
        name: "Support Knowledge Agent".to_string(),
        description: Some("Answers from approved knowledge spaces.".to_string()),
        system_instruction: "Answer only from cited knowledge.".to_string(),
        model_provider_id: "provider.model.openai".to_string(),
        model_id: "gpt-4.1".to_string(),
        model_parameters: Some(r#"{"temperature":0.1}"#.to_string()),
        retrieval_profile_id: Some(31),
        citation_policy: Some(r#"{"required":true}"#.to_string()),
        memory_policy_ref: Some("memory.session.summary".to_string()),
        tool_policy_ref: Some("tools.readonly".to_string()),
        answer_policy: Some(r#"{"abstainWhenNoEvidence":true}"#.to_string()),
        knowledge_mode: Default::default(),
        agent_implementation_id: sdkwork_knowledgebase_contract::default_agent_implementation_id(),
        status: KnowledgeAgentStatus::Active,
        bindings: vec![
            KnowledgeAgentBinding {
                binding_id: 61,
                profile_id: 41,
                tenant_id: 100001,
                space_id: 7,
                source_filter: None,
                document_filter: None,
                priority: 10,
                top_k: Some(8),
                min_score: Some(0.5),
                enabled: true,
            },
            KnowledgeAgentBinding {
                binding_id: 62,
                profile_id: 41,
                tenant_id: 100001,
                space_id: 11,
                source_filter: None,
                document_filter: None,
                priority: 20,
                top_k: Some(4),
                min_score: None,
                enabled: true,
            },
        ],
    };

    let json = serde_json::to_value(&profile).unwrap();

    assert_eq!(json["profileId"], "41");
    assert_eq!(json["modelProviderId"], "provider.model.openai");
    assert_eq!(json["modelId"], "gpt-4.1");
    assert_eq!(json["status"], "active");
    assert_eq!(json["bindings"].as_array().unwrap().len(), 2);
}
