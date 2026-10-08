use async_trait::async_trait;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_memory_context::{
    KnowledgeMemoryContextProvider, KnowledgeMemoryContextRequest,
};
use sdkwork_knowledgebase_memory::KnowledgebaseMemoryContextProviderAdapter;
use sdkwork_memory_spi::{
    AssembleMemoryContextCommand, MAX_MEMORY_RETRIEVAL_CANDIDATES, MemoryContextAssemblerPort,
    MemoryContextPackDraft, MemoryRetrieverPort, MemoryRetrieverResult, MemorySpiResult,
    MemorySensitivityReadScope, RetrieveMemoryCandidatesCommand,
};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn adapter_maps_memory_spi_context_into_knowledgebase_memory_fragments() {
    let retriever = Arc::new(RecordingMemoryRetriever::new(vec![
        "mem-001".to_string(),
        "mem-002".to_string(),
    ]));
    let assembler = Arc::new(RecordingMemoryAssembler::new(
        "prefers concise answers\nlikes SDKWork examples",
    ));
    let adapter =
        KnowledgebaseMemoryContextProviderAdapter::new(retriever.clone(), assembler.clone());

    let result = adapter
        .build_memory_context(KnowledgeMemoryContextRequest {
            tenant_id: 100001,
            actor_id: Some(30001),
            query: "concise sdkwork examples".to_string(),
            memory_policy_ref: "memory.session.summary".to_string(),
            max_tokens: 6,
        })
        .await
        .unwrap();

    assert_eq!(
        retriever.requests(),
        vec![RetrieveMemoryCandidatesCommand {
            query: "concise sdkwork examples".to_string(),
            // The adapter's documented default: read Public-sensitivity memories
            // until a caller-facing scope contract exists.
            read_scope: MemorySensitivityReadScope::Public,
            limit: MAX_MEMORY_RETRIEVAL_CANDIDATES,
        }]
    );
    assert_eq!(
        assembler.requests(),
        vec![AssembleMemoryContextCommand {
            memory_ids: vec!["mem-001".to_string(), "mem-002".to_string()],
        }]
    );
    assert_eq!(result.fragments.len(), 2);
    assert_eq!(result.fragments[0].memory_id, "mem-001");
    assert_eq!(result.fragments[0].content, "prefers concise answers");
    assert_eq!(result.fragments[0].rank, 1);
    assert_eq!(result.fragments[0].token_count, Some(3));
    assert_eq!(
        result.fragments[0].source_uri.as_deref(),
        Some("memory://mem-001")
    );
    assert_eq!(
        result.fragments[0].policy_ref.as_deref(),
        Some("memory.session.summary")
    );
    assert_eq!(result.fragments[1].memory_id, "mem-002");
    assert_eq!(result.fragments[1].content, "likes SDKWork examples");
    assert!(!result.truncated);
}

struct RecordingMemoryRetriever {
    memory_ids: Vec<String>,
    requests: Mutex<Vec<RetrieveMemoryCandidatesCommand>>,
}

impl RecordingMemoryRetriever {
    fn new(memory_ids: Vec<String>) -> Self {
        Self {
            memory_ids,
            requests: Mutex::new(vec![]),
        }
    }

    fn requests(&self) -> Vec<RetrieveMemoryCandidatesCommand> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl MemoryRetrieverPort for RecordingMemoryRetriever {
    fn retriever_code(&self) -> &str {
        "recording"
    }

    async fn retrieve(
        &self,
        command: RetrieveMemoryCandidatesCommand,
    ) -> MemorySpiResult<MemoryRetrieverResult> {
        self.requests.lock().unwrap().push(command);
        Ok(MemoryRetrieverResult {
            memory_ids: self.memory_ids.clone(),
        })
    }
}

struct RecordingMemoryAssembler {
    context_text: String,
    requests: Mutex<Vec<AssembleMemoryContextCommand>>,
}

impl RecordingMemoryAssembler {
    fn new(context_text: &str) -> Self {
        Self {
            context_text: context_text.to_string(),
            requests: Mutex::new(vec![]),
        }
    }

    fn requests(&self) -> Vec<AssembleMemoryContextCommand> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl MemoryContextAssemblerPort for RecordingMemoryAssembler {
    async fn assemble(
        &self,
        command: AssembleMemoryContextCommand,
    ) -> MemorySpiResult<MemoryContextPackDraft> {
        self.requests.lock().unwrap().push(command.clone());
        Ok(MemoryContextPackDraft {
            memory_ids: command.memory_ids,
            context_text: self.context_text.clone(),
        })
    }
}
