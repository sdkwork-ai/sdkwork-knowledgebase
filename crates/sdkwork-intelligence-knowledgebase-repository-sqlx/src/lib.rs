//! SQL storage support for SDKWork Knowledgebase.

mod agent_profile_store;
mod audit_event_store;
mod binding_scope_filters;
mod browser_projection_store;
mod chunk_transaction;
pub mod db;
mod drive_import_linkage_snapshot;
mod drive_object_ref_store;
mod embedding_store;
mod id;
mod index_store;
mod keyword_search;
pub mod mapper;
pub mod migrations;
mod okf_candidate_store;
mod okf_concept_link_store;
mod okf_concept_store;
mod postgres_chunk_store;
mod postgres_commerce_store;
mod postgres_context_binding_store;
mod postgres_drive_import_metadata_store;
mod postgres_group_space_binding_store;
mod postgres_import_stores;
mod postgres_knowledge_document_metadata_transaction;
mod postgres_markdown_index_metadata_store;
mod postgres_okf_candidate_transaction;
mod postgres_okf_concept_revision_metadata_store;
mod postgres_okf_concept_transaction;
mod postgres_outbox_store;
mod postgres_space_stores;
pub mod repository;
mod retrieval_profile_store;
mod retrieval_store;
mod wiki_persistence;

pub mod pgvector_layered_retrieval;
mod postgres_pgvector_retrieval;
mod provider_binding_readiness_store;
mod provider_binding_store;
mod provider_migration_store;
mod quota_transaction;

pub use agent_profile_store::PostgresKnowledgeAgentProfileStore;
pub use audit_event_store::{
    CreateKnowledgeAuditLegalHold, KnowledgeAuditEventRecord, KnowledgeAuditEventStore,
    KnowledgeAuditEventStoreError, KnowledgeAuditLegalHold, PostgresKnowledgeAuditEventStore,
};
pub use browser_projection_store::PostgresKnowledgeBrowserProjectionStore;
pub use db::{
    connect_knowledgebase_and_install_schema, connect_postgres_and_install_schema,
    connect_postgres_pool, connect_postgres_via_framework_lifecycle, database_config_from_url,
    is_postgres_database_url, knowledgebase_health_check,
    knowledgebase_process_pool_budget_from_url, postgres_health_check,
    require_postgres_rls_organization_id, require_postgres_rls_tenant_id,
    set_postgres_session_organization_id, set_postgres_session_tenant_id,
    KnowledgebaseProcessPoolBudget, PostgresRepositoryError, POSTGRES_ORGANIZATION_SESSION_KEY,
    POSTGRES_TENANT_SESSION_KEY,
};
pub use drive_object_ref_store::PostgresKnowledgeDriveObjectRefStore;
pub use embedding_store::PostgresKnowledgeEmbeddingStore;
pub use id::{
    default_knowledge_id_generator, install_default_knowledge_id_generator, KnowledgeIdGenerator,
    KnowledgeIdGeneratorError, SnowflakeKnowledgeIdGenerator,
};
pub use index_store::{KnowledgeIndexStoreError, PostgresKnowledgeIndexStore};
pub use keyword_search::{keyword_search_backend_for_database_url, KeywordSearchBackend};
pub use okf_candidate_store::PostgresKnowledgeOkfCandidateStore;
pub use okf_concept_link_store::PostgresKnowledgeOkfConceptLinkStore;
pub use okf_concept_store::PostgresKnowledgeOkfConceptStore;
pub use pgvector_layered_retrieval::PgVectorLayeredRetrievalBackend;
pub use postgres_chunk_store::PostgresKnowledgeChunkStore;
pub use postgres_commerce_store::PostgresCommerceStore;
pub use postgres_context_binding_store::PostgresContextBindingStore;
pub use postgres_drive_import_metadata_store::PostgresDriveImportMetadataStore;
pub use postgres_group_space_binding_store::PostgresGroupKnowledgeSpaceBindingStore;
pub use postgres_import_stores::{
    PostgresIngestionJobStore, PostgresKnowledgeDocumentStore,
    PostgresKnowledgeDocumentVersionStore, PostgresKnowledgeSourceStore,
};
pub use postgres_markdown_index_metadata_store::PostgresMarkdownIndexMetadataStore;
pub use postgres_okf_concept_revision_metadata_store::PostgresOkfConceptRevisionMetadataStore;
pub use postgres_outbox_store::PostgresKnowledgeOutboxStore;
pub use postgres_pgvector_retrieval::PgVectorKnowledgeRetrievalBackend;
pub use postgres_space_stores::{
    PostgresKnowledgeOkfBundleFileStore, PostgresKnowledgeSpaceStore, TenantKnowledgebaseSummary,
};
pub use provider_binding_readiness_store::SqlxKnowledgeEngineProviderBindingReadinessStore;
pub use provider_binding_store::SqlxKnowledgeEngineProviderBindingStore;
pub use provider_migration_store::SqlxKnowledgeEngineProviderMigrationStore;
pub use retrieval_profile_store::{
    KnowledgeRetrievalProfileStoreError, PostgresKnowledgeRetrievalProfileStore,
};
pub use retrieval_store::PostgresKnowledgeChunkRetrievalStore;
pub use wiki_persistence::SqlxWikiPersistenceStore;
