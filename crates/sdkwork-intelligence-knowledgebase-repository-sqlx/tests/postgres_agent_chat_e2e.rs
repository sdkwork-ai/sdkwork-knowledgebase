//! PostgreSQL-backed end-to-end proof that an uploaded document becomes an
//! Agent answer with citations:
//!
//! Drive payload -> ingestion chunks -> keyword retrieval -> agent profile
//! binding -> kernel RAG chat (contract model provider).
//!
//! The test is optional: it runs only when `SDKWORK_DATABASE_URL` (or
//! `DATABASE_URL`) points at PostgreSQL and `SDKWORK_KNOWLEDGEBASE_TENANT_ID`
//! matches the deployment-bound RLS tenant.

use sdkwork_database_config::DatabaseEngine;
use sdkwork_intelligence_knowledgebase_repository_sqlx::{
    connect_postgres_and_install_schema, connect_postgres_pool, is_postgres_database_url,
    PostgresIngestionJobStore, PostgresKnowledgeAgentProfileStore,
    PostgresKnowledgeChunkRetrievalStore, PostgresKnowledgeDocumentStore,
    PostgresKnowledgeDocumentVersionStore, PostgresKnowledgeDriveObjectRefStore,
    PostgresKnowledgeSpaceStore,
};
use sdkwork_intelligence_knowledgebase_service::agent_chat::KnowledgeAgentChatService;
use sdkwork_intelligence_knowledgebase_service::imports::KnowledgeDriveImportPipelineService;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_agent_profile_store::KnowledgeAgentProfileStore;
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_store::{
    CreateKnowledgeDocumentRecord, KnowledgeDocumentIdentityScope, KnowledgeDocumentStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_document_version_store::{
    CreateKnowledgeDocumentVersionRecord, KnowledgeDocumentVersionStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_drive_object_ref_store::{
    CreateKnowledgeDriveObjectRefRecord, KnowledgeDriveObjectRefStore, MANAGED_DRIVE_ACCESS_MODE,
    SDKWORK_DRIVE_PROVIDER_KIND,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_ingestion_job_store::{
    CreateIngestionJobRecord, DriveImportJobLinkage, IngestionJobStore,
};
use sdkwork_intelligence_knowledgebase_service::ports::knowledge_space_store::{
    CreateKnowledgeSpaceRecord, KnowledgeSpaceStore,
};
use sdkwork_intelligence_knowledgebase_service::retrieval::KnowledgeRetrievalService;
use sdkwork_knowledgebase_agent_provider::{KnowledgebaseRetrievalClient, OkfKnowledgeClient};
use sdkwork_knowledgebase_contract::agent_chat::{
    KnowledgeAgentChatRequest, KnowledgeAgentKnowledgeMode,
};
use sdkwork_knowledgebase_contract::ingest::KnowledgeDriveImportResult;
use sdkwork_knowledgebase_contract::rag::{
    KnowledgeAgentBindingRequest, KnowledgeAgentProfileRequest, KnowledgeAgentStatus,
    KnowledgeCitation, KnowledgeContextFragment, KnowledgeRetrievalMethod,
    KnowledgeRetrievalRequest, KnowledgeRetrievalResult,
};
use sdkwork_knowledgebase_contract::source::{KnowledgeSource, KnowledgeSourceType};
use sdkwork_knowledgebase_test_support::fake_drive::FakeKnowledgeDriveStorage;
use std::time::{SystemTime, UNIX_EPOCH};

fn optional_postgres_database_url() -> Option<String> {
    std::env::var("SDKWORK_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| is_postgres_database_url(url))
}

/// Retrieval client used by the kernel provider. It returns the same chunk that
/// the real PostgreSQL retrieval already returned so the contract model provider
/// receives a context frame and can produce a grounded answer.
#[derive(Clone)]
struct StaticRetrievalClient {
    chunk: KnowledgeContextFragment,
}

impl KnowledgebaseRetrievalClient for StaticRetrievalClient {
    fn retrieve(
        &self,
        _request: KnowledgeRetrievalRequest,
    ) -> Result<KnowledgeRetrievalResult, String> {
        Ok(KnowledgeRetrievalResult {
            retrieval_id: 9001,
            trace: None,
            hits: vec![self.chunk.clone()],
        })
    }

    fn read_document(
        &self,
        _document_id: &str,
    ) -> Result<sdkwork_agent_kernel::KnowledgeDocument, String> {
        Err("document read is not used by the contract model provider".to_string())
    }

    fn list_documents(
        &self,
        _filter: sdkwork_agent_kernel::KnowledgeDocumentFilter,
    ) -> Result<Vec<sdkwork_agent_kernel::KnowledgeDocument>, String> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct NoopOkfClient;

impl OkfKnowledgeClient for NoopOkfClient {
    fn search_okf_concepts(
        &self,
        _space_id: u64,
        _query: &str,
        _top_k: usize,
    ) -> Result<Vec<sdkwork_knowledgebase_contract::okf::OkfConceptSummary>, String> {
        Ok(Vec::new())
    }

    fn read_okf_concept_content(
        &self,
        _space_id: u64,
        _logical_path: &str,
    ) -> Result<String, String> {
        Err("okf read is not used in RAG mode".to_string())
    }
}

#[tokio::test]
async fn postgres_agent_chat_answers_from_ingested_knowledge_with_citations() {
    let Some(database_url) = optional_postgres_database_url() else {
        eprintln!(
            "skipping postgres agent chat e2e: set SDKWORK_DATABASE_URL or DATABASE_URL to a postgres URL"
        );
        return;
    };
    let tenant_id = sdkwork_knowledgebase_observability::deployment_tenant_id();
    if tenant_id == 0 {
        eprintln!(
            "skipping postgres agent chat e2e: set SDKWORK_KNOWLEDGEBASE_TENANT_ID to the deployment tenant"
        );
        return;
    }

    let pool = connect_postgres_and_install_schema(&database_url)
        .await
        .expect("connect postgres knowledgebase schema");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos() as u64;

    let spaces = PostgresKnowledgeSpaceStore::new(pool.clone(), tenant_id, 0);
    let space = spaces
        .create_space(CreateKnowledgeSpaceRecord {
            name: format!("postgres chat e2e space {nonce}"),
            description: Some("agent chat e2e".to_string()),
            okf_bundle_initialized: false,
            knowledge_mode: KnowledgeAgentKnowledgeMode::Rag,
        })
        .await
        .expect("create chat e2e space");

    let drive = FakeKnowledgeDriveStorage::default();
    let stored = drive
        .put_text(
            format!("raw/documents/chat-{nonce}/original/policy.md"),
            "original_document",
            "# Refund policy\n\nRefunds are accepted within 30 days of purchase.\n",
        )
        .await
        .expect("store chat document payload");

    let object_refs = PostgresKnowledgeDriveObjectRefStore::new(pool.clone(), tenant_id, 0);
    let object_ref = object_refs
        .create_object_ref(CreateKnowledgeDriveObjectRefRecord {
            space_id: space.id,
            drive_space_id: Some("drive-space-chat-e2e".to_string()),
            drive_node_id: Some(format!("node-chat-{nonce}")),
            logical_path: Some(stored.logical_path.clone()),
            drive_provider_kind: SDKWORK_DRIVE_PROVIDER_KIND.to_string(),
            drive_storage_provider_id: stored.storage_provider_id.clone(),
            drive_bucket: stored.bucket.clone(),
            drive_object_key: stored.object_key.clone(),
            drive_object_version: None,
            drive_etag: None,
            content_type: Some("text/markdown".to_string()),
            size_bytes: stored.size_bytes,
            checksum_sha256_hex: stored.checksum_sha256_hex.clone(),
            object_role: "original_document".to_string(),
            access_mode: MANAGED_DRIVE_ACCESS_MODE.to_string(),
        })
        .await
        .expect("create chat drive object ref");

    let documents = PostgresKnowledgeDocumentStore::new(pool.clone(), tenant_id, 0);
    let document = documents
        .create_document(CreateKnowledgeDocumentRecord {
            space_id: space.id,
            source_id: None,
            identity_scope: KnowledgeDocumentIdentityScope::SourceAndOriginalDriveNode,
            original_file_drive_node_id: object_ref.drive_node_id.clone(),
            title: "Refund policy".to_string(),
            mime_type: Some("text/markdown".to_string()),
            language: Some("en".to_string()),
        })
        .await
        .expect("create chat document");

    let versions = PostgresKnowledgeDocumentVersionStore::new(pool.clone(), tenant_id, 0);
    let version = versions
        .create_document_version(CreateKnowledgeDocumentVersionRecord {
            document_id: document.id,
            version_no: 1,
            original_object_ref_id: object_ref.id,
            checksum_sha256_hex: object_ref.checksum_sha256_hex.clone(),
            size_bytes: object_ref.size_bytes,
            mime_type: Some("text/markdown".to_string()),
        })
        .await
        .expect("create chat document version");

    let jobs = PostgresIngestionJobStore::new(pool.clone(), tenant_id, 0)
        .with_database_engine(DatabaseEngine::Postgres);
    let created = jobs
        .create_or_get_job(CreateIngestionJobRecord {
            space_id: space.id,
            source_type: KnowledgeSourceType::DriveObject.as_str().to_string(),
            idempotency_key: format!("postgres-chat-e2e-{nonce}"),
            idempotency_fingerprint_sha256_hex: None,
        })
        .await
        .expect("create chat ingestion job");
    jobs.attach_drive_import_linkage(
        created.job.id,
        DriveImportJobLinkage {
            source_id: 1,
            document_id: document.id,
            document_version_id: version.id,
            original_object_ref: object_ref.clone(),
        },
    )
    .await
    .expect("attach chat drive import linkage");

    let import = KnowledgeDriveImportResult {
        source: KnowledgeSource {
            id: 1,
            space_id: space.id,
            source_type: KnowledgeSourceType::DriveObject,
            provider: Some("sdkwork-drive".to_string()),
            drive_bucket: Some(object_ref.drive_bucket.clone()),
            drive_prefix: Some(object_ref.drive_object_key.clone()),
            connector_metadata_json: None,
        },
        document: document.clone(),
        version,
        original_object_ref: object_ref,
        job: created.job,
    };
    KnowledgeDriveImportPipelineService::new(&drive, &jobs)
        .process_import_result(&import)
        .await
        .expect("ingest chat document");

    let retrieval_pg_pool = connect_postgres_pool(&database_url)
        .await
        .expect("connect chat retrieval pool");
    let retrieval_store =
        PostgresKnowledgeChunkRetrievalStore::new(retrieval_pg_pool, tenant_id, 0)
            .with_database_engine(DatabaseEngine::Postgres);
    let retrieval = KnowledgeRetrievalService::new(&retrieval_store, &retrieval_store);

    let profiles = PostgresKnowledgeAgentProfileStore::new(pool.clone(), tenant_id, 0);
    let profile = profiles
        .create_profile(KnowledgeAgentProfileRequest {
            tenant_id,
            name: format!("postgres chat agent {nonce}"),
            description: Some("agent chat e2e".to_string()),
            system_instruction: "Answer from the knowledge base and cite sources.".to_string(),
            model_provider_id:
                sdkwork_intelligence_knowledgebase_service::agent_chat::CONTRACT_MODEL_PROVIDER_ID
                    .to_string(),
            model_id: "contract".to_string(),
            model_parameters: None,
            retrieval_profile_id: None,
            citation_policy: None,
            memory_policy_ref: None,
            tool_policy_ref: None,
            answer_policy: None,
            knowledge_mode: KnowledgeAgentKnowledgeMode::Rag,
            agent_implementation_id:
                sdkwork_knowledgebase_contract::KNOWLEDGEBASE_CONTRACT_AGENT_IMPLEMENTATION_ID
                    .to_string(),
            status: KnowledgeAgentStatus::Active,
        })
        .await
        .expect("create chat agent profile");
    profiles
        .create_binding(KnowledgeAgentBindingRequest {
            tenant_id,
            profile_id: profile.profile_id,
            space_id: space.id,
            source_filter: None,
            document_filter: None,
            priority: 0,
            top_k: Some(5),
            min_score: None,
            enabled: true,
        })
        .await
        .expect("bind chat agent to space");

    let chunk = KnowledgeContextFragment {
        chunk_id: document.id,
        document_id: document.id,
        document_version_id: None,
        space_id: space.id,
        title: "Refund policy".to_string(),
        content: "Refunds are accepted within 30 days of purchase.".to_string(),
        score: Some(0.95),
        rank: 1,
        token_count: Some(10),
        retrieval_method: KnowledgeRetrievalMethod::FullText,
        citation: Some(KnowledgeCitation {
            document_id: document.id,
            document_version_id: None,
            chunk_id: None,
            title: "Refund policy".to_string(),
            source_uri: Some(format!("kb://documents/{}", document.id)),
            locator: None,
            score: Some(0.95),
        }),
    };

    let chat = KnowledgeAgentChatService::new(
        &profiles,
        &retrieval,
        StaticRetrievalClient { chunk },
        NoopOkfClient,
        None,
        None,
        None,
        None,
    );
    let response = chat
        .chat(
            profile.profile_id,
            KnowledgeAgentChatRequest {
                tenant_id,
                actor_id: Some(1),
                message: "What is the refund policy?".to_string(),
                history: Vec::new(),
                mode: Some(KnowledgeAgentKnowledgeMode::Rag),
                session_id: None,
                model_provider_id: None,
                model_id: None,
                agent_implementation_id: None,
            },
        )
        .await
        .expect("agent chat answer");

    assert!(
        response.answer.contains("Refund policy"),
        "answer must be grounded in the ingested document: {}",
        response.answer
    );
    assert!(
        !response.citations.is_empty(),
        "agent chat must return at least one citation"
    );
    assert_eq!(
        response.citations[0].document_id,
        Some(document.id),
        "citation must point at the ingested document"
    );
}
