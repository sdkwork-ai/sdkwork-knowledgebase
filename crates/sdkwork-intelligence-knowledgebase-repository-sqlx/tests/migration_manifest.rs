use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::{
    POSTGRES_ACCESS_MODE_MIGRATION, POSTGRES_AGENT_IMPLEMENTATION_MIGRATION,
    POSTGRES_CONTEXT_BINDING_MIGRATION, POSTGRES_CORE_MIGRATION,
    POSTGRES_GROUP_KNOWLEDGE_SPACE_MIGRATION, POSTGRES_GROUP_MEMBERSHIP_PROJECTION_MIGRATION,
    POSTGRES_OUTBOX_MIGRATION, POSTGRES_PGVECTOR_MIGRATION,
};
use std::collections::BTreeSet;
const APP_ROOT_POSTGRES_BASELINE: &str =
    include_str!("../../../database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql");
// Post-baseline migrations are folded into the consolidated pre-launch baseline
// (see database/migrations/postgres/README.md); these contract probes therefore run
// against the baseline itself.
const APP_ROOT_POSTGRES_TENANT_SCOPE_MIGRATION: &str = APP_ROOT_POSTGRES_BASELINE;
const APP_ROOT_POSTGRES_LIVE_WIKI_MIGRATION: &str = APP_ROOT_POSTGRES_BASELINE;
const APP_ROOT_POSTGRES_ORGANIZATION_ISOLATION_MIGRATION: &str = APP_ROOT_POSTGRES_BASELINE;
const APP_ROOT_POSTGRES_OUTBOX_CLAIM_FENCING_MIGRATION: &str = APP_ROOT_POSTGRES_BASELINE;
const APP_ROOT_POSTGRES_LIVE_WIKI_ROLLBACK: &str = APP_ROOT_POSTGRES_BASELINE;
const APP_ROOT_DATABASE_MANIFEST: &str = include_str!("../../../database/database.manifest.json");
const APP_ROOT_DATABASE_SCHEMA: &str = include_str!("../../../database/contract/schema.yaml");
const APP_ROOT_DATABASE_TABLE_REGISTRY: &str =
    include_str!("../../../database/contract/table-registry.json");
const AGENT_PROFILE_STORE_SOURCE: &str = include_str!("../src/agent_profile_store.rs");
const AUDIT_EVENT_STORE_SOURCE: &str = include_str!("../src/audit_event_store.rs");
const DRIVE_OBJECT_REF_STORE_SOURCE: &str = include_str!("../src/drive_object_ref_store.rs");
const INDEX_STORE_SOURCE: &str = include_str!("../src/index_store.rs");
const OKF_CONCEPT_LINK_STORE_SOURCE: &str = include_str!("../src/okf_concept_link_store.rs");
const OKF_CONCEPT_STORE_SOURCE: &str = include_str!("../src/okf_concept_store.rs");
const RETRIEVAL_PROFILE_STORE_SOURCE: &str = include_str!("../src/retrieval_profile_store.rs");
const RETRIEVAL_STORE_SOURCE: &str = include_str!("../src/retrieval_store.rs");
const POSTGRES_COMMERCE_STORE_SOURCE: &str = include_str!("../src/postgres_commerce_store.rs");
const POSTGRES_CONTEXT_BINDING_STORE_SOURCE: &str =
    include_str!("../src/postgres_context_binding_store.rs");
const POSTGRES_DRIVE_IMPORT_METADATA_STORE_SOURCE: &str =
    include_str!("../src/postgres_drive_import_metadata_store.rs");
const POSTGRES_IMPORT_STORES_SOURCE: &str = include_str!("../src/postgres_import_stores.rs");
const POSTGRES_KNOWLEDGE_DOCUMENT_METADATA_TRANSACTION_SOURCE: &str =
    include_str!("../src/postgres_knowledge_document_metadata_transaction.rs");
const POSTGRES_OKF_CANDIDATE_TRANSACTION_SOURCE: &str =
    include_str!("../src/postgres_okf_candidate_transaction.rs");
const POSTGRES_OKF_CONCEPT_REVISION_METADATA_STORE_SOURCE: &str =
    include_str!("../src/postgres_okf_concept_revision_metadata_store.rs");
const POSTGRES_OKF_CONCEPT_TRANSACTION_SOURCE: &str =
    include_str!("../src/postgres_okf_concept_transaction.rs");
const POSTGRES_OUTBOX_STORE_SOURCE: &str = include_str!("../src/postgres_outbox_store.rs");
const POSTGRES_SPACE_STORES_SOURCE: &str = include_str!("../src/postgres_space_stores.rs");
const WIKI_CHECKPOINT_STORE_SOURCE: &str = include_str!("../src/wiki_persistence/checkpoint.rs");
const WIKI_INBOX_STORE_SOURCE: &str = include_str!("../src/wiki_persistence/inbox.rs");
const WIKI_PROJECTION_STORE_SOURCE: &str = include_str!("../src/wiki_persistence/projection.rs");
const WIKI_PUBLICATION_STORE_SOURCE: &str = include_str!("../src/wiki_persistence/publication.rs");
const WIKI_RENDITION_STORE_SOURCE: &str = include_str!("../src/wiki_persistence/rendition.rs");

const REQUIRED_CORE_TABLES: [&str; 22] = [
    "kb_space",
    "kb_collection",
    "kb_source",
    "kb_drive_object_ref",
    "kb_document",
    "kb_document_version",
    "kb_chunk",
    "kb_index",
    "kb_embedding",
    "kb_retrieval_profile",
    "kb_retrieval_trace",
    "kb_retrieval_hit",
    "kb_agent_profile",
    "kb_agent_knowledge_binding",
    "kb_ingestion_job",
    "kb_ingestion_job_item",
    "kb_okf_concept",
    "kb_okf_concept_revision",
    "kb_okf_bundle_file",
    "kb_okf_schema_profile",
    "kb_okf_log_entry",
    "kb_local_mirror_package",
];

const REQUIRED_CORE_INDEXES: [&str; 49] = [
    "uk_kb_space_uuid",
    "uk_kb_space_drive_space",
    "uk_kb_collection_uuid",
    "uk_kb_source_uuid",
    "uk_kb_source_identity",
    "uk_kb_drive_object_ref_uuid",
    "idx_kb_drive_object_locator",
    "uk_kb_drive_object_ref_locator",
    "idx_kb_drive_object_role",
    "idx_kb_drive_object_drive_node",
    "uk_kb_document_uuid",
    "idx_kb_document_drive_node",
    "uk_kb_document_identity",
    "uk_kb_document_version_uuid",
    "uk_kb_document_version_no",
    "uk_kb_chunk_uuid",
    "idx_kb_chunk_document_version",
    "idx_kb_chunk_space_status",
    "uk_kb_index_uuid",
    "idx_kb_index_scope",
    "uk_kb_embedding_uuid",
    "uk_kb_embedding_index_chunk",
    "idx_kb_embedding_chunk",
    "uk_kb_retrieval_profile_uuid",
    "idx_kb_retrieval_profile_tenant_status",
    "uk_kb_retrieval_trace_uuid",
    "idx_kb_retrieval_trace_profile_created",
    "idx_kb_retrieval_trace_actor_created",
    "uk_kb_retrieval_hit_uuid",
    "idx_kb_retrieval_hit_trace_rank",
    "idx_kb_retrieval_hit_chunk",
    "uk_kb_agent_profile_uuid",
    "idx_kb_agent_profile_model",
    "uk_kb_agent_knowledge_binding_uuid",
    "idx_kb_agent_knowledge_binding_profile",
    "uk_kb_ingestion_job_uuid",
    "uk_kb_ingestion_job_idempotency",
    "uk_kb_ingestion_job_item_uuid",
    "uk_kb_okf_concept_uuid",
    "uk_kb_okf_concept_id",
    "uk_kb_okf_concept_path",
    "idx_kb_okf_concept_state",
    "uk_kb_okf_concept_revision_uuid",
    "uk_kb_okf_concept_revision_no",
    "uk_kb_okf_bundle_file_uuid",
    "uk_kb_okf_bundle_file_path",
    "uk_kb_okf_schema_profile_uuid",
    "uk_kb_okf_log_entry_uuid",
    "uk_kb_local_mirror_package_uuid",
];

const LIVE_WIKI_TABLES: [&str; 5] = [
    "kb_site_publication",
    "kb_source_file_projection",
    "kb_source_file_rendition",
    "kb_drive_source_checkpoint",
    "kb_drive_event_inbox",
];

#[test]
fn core_migrations_include_required_knowledgebase_tables() {
    for migration in [POSTGRES_CORE_MIGRATION] {
        assert!(migration.contains("description"));
        assert!(migration.contains("okf_bundle_initialized"));

        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        for table in REQUIRED_CORE_TABLES {
            assert!(tables.contains(table), "missing required table: {table}");
        }
    }
}

#[test]
fn core_migrations_use_kb_prefix_for_defined_database_objects() {
    for migration in [POSTGRES_CORE_MIGRATION] {
        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        let indexes = defined_database_objects(migration, "CREATE INDEX IF NOT EXISTS ")
            .into_iter()
            .chain(defined_database_objects(
                migration,
                "CREATE UNIQUE INDEX IF NOT EXISTS ",
            ))
            .collect::<BTreeSet<_>>();

        for table in tables {
            assert!(
                table.starts_with("kb_"),
                "knowledgebase table must use kb_ prefix: {table}"
            );
        }

        for index in indexes {
            assert!(
                index.starts_with("idx_kb_") || index.starts_with("uk_kb_"),
                "knowledgebase index must use idx_kb_ or uk_kb_ prefix: {index}"
            );
        }

        assert!(!migration.contains(" ON knowledge_"));
        assert!(!migration.contains("uk_knowledge_"));
        assert!(!migration.contains("idx_knowledge_"));
    }
}

#[test]
fn drive_object_ref_migrations_store_stable_locator_metadata_not_delivery_secrets() {
    {
        let migration = POSTGRES_CORE_MIGRATION;
        assert!(migration.contains("drive_provider_kind"));
        assert!(migration.contains("drive_bucket"));
        assert!(migration.contains("drive_object_key"));
        assert!(migration.contains("drive_object_version"));
        assert!(migration.contains("drive_etag"));
        assert!(migration.contains("drive_metadata"));
        assert!(migration.contains("object_role"));
        assert!(migration.contains("access_mode"));
        assert!(migration.contains("idx_kb_drive_object_locator"));
        assert!(migration.contains("idx_kb_drive_object_role"));

        let lowercase = migration.to_ascii_lowercase();
        assert!(!lowercase.contains("presigned"));
        assert!(!lowercase.contains("credential"));
        assert!(!lowercase.contains("secret"));
    }
}

#[test]
fn core_migrations_define_identity_and_idempotency_uniques() {
    for migration in [POSTGRES_CORE_MIGRATION] {
        let indexes = defined_database_objects(migration, "CREATE UNIQUE INDEX IF NOT EXISTS ");
        for index in [
            "uk_kb_space_uuid",
            "uk_kb_source_identity",
            "uk_kb_drive_object_ref_locator",
            "uk_kb_document_identity",
            "uk_kb_document_version_no",
            "uk_kb_ingestion_job_idempotency",
            "uk_kb_okf_concept_id",
            "uk_kb_okf_concept_revision_no",
            "uk_kb_okf_bundle_file_path",
            "uk_kb_okf_log_entry_sequence",
        ] {
            assert!(
                indexes.contains(index),
                "missing required unique index: {index}"
            );
        }
    }
}

#[test]
fn core_migrations_define_uuid_unique_indexes_for_all_uuid_tables() {
    for migration in [POSTGRES_CORE_MIGRATION] {
        let indexes = defined_database_objects(migration, "CREATE UNIQUE INDEX IF NOT EXISTS ");
        for table in REQUIRED_CORE_TABLES {
            let index = format!("uk_{table}_uuid");
            assert!(
                indexes.contains(index.as_str()),
                "missing uuid unique index for {table}: {index}"
            );
        }
    }
}

#[test]
fn core_migrations_define_document_identity_scope_strategy() {
    {
        let migration = POSTGRES_CORE_MIGRATION;
        assert!(migration.contains("identity_scope"));
        assert!(migration.contains("source_only"));
        assert!(migration.contains("source_and_original_drive_node"));
        assert!(migration.contains("identity_scope,"));
        assert!(migration.contains("WHEN identity_scope = 'source_only' THEN ''"));
        assert!(migration.contains("ELSE COALESCE(original_file_drive_node_id, '')"));
    }
}

#[test]
fn core_migrations_harden_nullable_identity_columns() {
    assert!(POSTGRES_CORE_MIGRATION.contains("idempotency_key VARCHAR(128) NOT NULL"));
    assert!(POSTGRES_CORE_MIGRATION.contains("okf_log_sequence_counter BIGINT NOT NULL DEFAULT 0"));
    assert!(POSTGRES_CORE_MIGRATION.contains("revision_counter BIGINT NOT NULL DEFAULT 0"));

    {
        let migration = POSTGRES_CORE_MIGRATION;
        assert!(migration.contains("COALESCE(drive_object_version"));
    }
}

#[test]
fn core_migrations_define_all_required_indexes_with_kb_prefix() {
    for migration in [POSTGRES_CORE_MIGRATION] {
        let indexes = defined_database_objects(migration, "CREATE INDEX IF NOT EXISTS ")
            .into_iter()
            .chain(defined_database_objects(
                migration,
                "CREATE UNIQUE INDEX IF NOT EXISTS ",
            ))
            .collect::<BTreeSet<_>>();

        for index in REQUIRED_CORE_INDEXES {
            assert!(indexes.contains(index), "missing required index: {index}");
        }
    }
}

#[test]
fn rag_migrations_define_retrieval_index_trace_and_agent_binding_columns() {
    for migration in [POSTGRES_CORE_MIGRATION] {
        for snippet in [
            "CREATE TABLE IF NOT EXISTS kb_chunk",
            "document_version_id",
            "chunk_index",
            "content_text",
            "token_count",
            "locator",
            "CREATE TABLE IF NOT EXISTS kb_index",
            "index_kind",
            "embedding_provider_id",
            "embedding_model",
            "dimension",
            "metric",
            "CREATE TABLE IF NOT EXISTS kb_embedding",
            "vector_ref",
            "embedding_hash",
            "CREATE TABLE IF NOT EXISTS kb_retrieval_profile",
            "strategy",
            "rerank_enabled",
            "context_budget_tokens",
            "CREATE TABLE IF NOT EXISTS kb_retrieval_trace",
            "query_text_redacted",
            "latency_ms",
            "result_count",
            "CREATE TABLE IF NOT EXISTS kb_retrieval_hit",
            "retrieval_trace_id",
            "match_reason",
            "citation",
            "CREATE TABLE IF NOT EXISTS kb_agent_profile",
            "model_provider_id",
            "model_id",
            "system_instruction",
            "CREATE TABLE IF NOT EXISTS kb_agent_knowledge_binding",
            "profile_id",
            "space_id",
            "source_filter",
            "document_filter",
            "min_score",
        ] {
            assert!(
                migration.contains(snippet),
                "RAG migration must include snippet: {snippet}"
            );
        }

        let lowercase = migration.to_ascii_lowercase();
        assert!(!lowercase.contains("presigned"));
        assert!(!lowercase.contains("access_token"));
        assert!(!lowercase.contains("refresh_token"));
        assert!(!lowercase.contains("api_key"));
    }
}

#[test]
fn access_mode_migrations_add_profile_space_mode_and_vector_json() {
    for migration in [POSTGRES_ACCESS_MODE_MIGRATION] {
        for snippet in [
            "knowledge_mode",
            "vector_json",
            "idx_kb_agent_profile_knowledge_mode",
            "idx_kb_space_knowledge_mode",
        ] {
            assert!(
                migration.contains(snippet),
                "access mode migration must include snippet: {snippet}"
            );
        }
    }
}

#[test]
fn agent_implementation_migrations_add_profile_runtime_selector() {
    for migration in [POSTGRES_AGENT_IMPLEMENTATION_MIGRATION] {
        for snippet in [
            "agent_implementation_id",
            "plugin.intelligence.rig",
            "idx_kb_agent_profile_agent_implementation",
        ] {
            assert!(
                migration.contains(snippet),
                "agent implementation migration must include snippet: {snippet}"
            );
        }
    }
}

#[test]
fn context_binding_migrations_define_space_context_binding_table() {
    for migration in [POSTGRES_CONTEXT_BINDING_MIGRATION] {
        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        assert!(tables.contains("kb_space_context_binding"));
        let indexes = defined_database_objects(migration, "CREATE INDEX IF NOT EXISTS ")
            .into_iter()
            .chain(defined_database_objects(
                migration,
                "CREATE UNIQUE INDEX IF NOT EXISTS ",
            ))
            .collect::<BTreeSet<_>>();
        for index in [
            "uk_kb_space_context",
            "idx_kb_space_context_lookup",
            "idx_kb_space_context_space",
        ] {
            assert!(
                indexes.contains(index),
                "missing context binding index: {index}"
            );
        }
    }
}

#[test]
fn outbox_migrations_define_kb_outbox_event_table() {
    for migration in [POSTGRES_OUTBOX_MIGRATION] {
        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        assert!(tables.contains("kb_outbox_event"));
        let indexes = defined_database_objects(migration, "CREATE INDEX IF NOT EXISTS ")
            .into_iter()
            .chain(defined_database_objects(
                migration,
                "CREATE UNIQUE INDEX IF NOT EXISTS ",
            ))
            .collect::<BTreeSet<_>>();
        for index in [
            "uk_kb_outbox_event_uuid",
            "idx_kb_outbox_event_status_created",
        ] {
            assert!(indexes.contains(index), "missing outbox index: {index}");
        }
    }
}

#[test]
fn postgres_pgvector_migration_defines_vector_embedding_column() {
    for snippet in [
        "CREATE EXTENSION IF NOT EXISTS vector",
        "embedding_vector vector(1536)",
        "idx_kb_embedding_vector_hnsw",
    ] {
        assert!(
            POSTGRES_PGVECTOR_MIGRATION.contains(snippet),
            "pgvector migration must include snippet: {snippet}"
        );
    }
}

#[test]
fn okf_migrations_define_link_and_candidate_tables() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_OKF_LINK_CANDIDATE_MIGRATION;

    for migration in [POSTGRES_OKF_LINK_CANDIDATE_MIGRATION] {
        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        assert!(tables.contains("kb_okf_concept_link"));
        assert!(tables.contains("kb_okf_candidate"));
        let indexes = defined_database_objects(migration, "CREATE INDEX IF NOT EXISTS ")
            .into_iter()
            .chain(defined_database_objects(
                migration,
                "CREATE UNIQUE INDEX IF NOT EXISTS ",
            ))
            .collect::<BTreeSet<_>>();
        for index in [
            "uk_kb_okf_concept_link_uuid",
            "uk_kb_okf_concept_link_edge",
            "idx_kb_okf_concept_link_space_from",
            "idx_kb_okf_concept_link_space_to",
            "uk_kb_okf_candidate_uuid",
            "idx_kb_okf_candidate_space_state",
        ] {
            assert!(
                indexes.contains(index),
                "missing okf migration index: {index}"
            );
        }
    }
}

#[test]
fn outbox_delivery_migrations_add_retry_metadata_columns() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_OUTBOX_DELIVERY_MIGRATION;

    for migration in [POSTGRES_OUTBOX_DELIVERY_MIGRATION] {
        for snippet in ["last_error", "retry_count", "kb_outbox_event"] {
            assert!(
                migration.contains(snippet),
                "outbox delivery migration must include snippet: {snippet}"
            );
        }
    }
}

#[test]
fn chunk_fts_migrations_define_keyword_search_primitives() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_CHUNK_FTS_MIGRATION;

    assert!(POSTGRES_CHUNK_FTS_MIGRATION.contains("search_vector"));
    assert!(POSTGRES_CHUNK_FTS_MIGRATION.contains("idx_kb_chunk_search_vector"));
}

#[test]
fn performance_index_migrations_target_outbox_event_table() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_PERFORMANCE_INDEXES_MIGRATION;

    {
        let migration = POSTGRES_PERFORMANCE_INDEXES_MIGRATION;
        assert!(migration.contains("idx_kb_ingestion_job_tenant_state_status"));
        assert!(migration.contains("idx_kb_outbox_stale_claim"));
        assert!(migration.contains("kb_outbox_event"));
        assert!(!migration.contains(" ON kb_outbox "));
    }
}

#[test]
fn market_migrations_define_market_tables() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_MARKET_MIGRATION;

    for migration in [POSTGRES_MARKET_MIGRATION] {
        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        for table in ["kb_market_listing", "kb_market_subscription"] {
            assert!(tables.contains(table), "missing market table: {table}");
        }
        assert!(!migration.contains("site_deployment"));
    }
}

#[test]
fn runtime_sql_value_bindings_are_generated_by_database_dialect() {
    for (file, source) in [
        ("agent_profile_store.rs", AGENT_PROFILE_STORE_SOURCE),
        ("audit_event_store.rs", AUDIT_EVENT_STORE_SOURCE),
        ("drive_object_ref_store.rs", DRIVE_OBJECT_REF_STORE_SOURCE),
        ("index_store.rs", INDEX_STORE_SOURCE),
        ("okf_concept_link_store.rs", OKF_CONCEPT_LINK_STORE_SOURCE),
        ("okf_concept_store.rs", OKF_CONCEPT_STORE_SOURCE),
        ("retrieval_profile_store.rs", RETRIEVAL_PROFILE_STORE_SOURCE),
        ("retrieval_store.rs", RETRIEVAL_STORE_SOURCE),
        ("postgres_commerce_store.rs", POSTGRES_COMMERCE_STORE_SOURCE),
        (
            "postgres_context_binding_store.rs",
            POSTGRES_CONTEXT_BINDING_STORE_SOURCE,
        ),
        (
            "postgres_drive_import_metadata_store.rs",
            POSTGRES_DRIVE_IMPORT_METADATA_STORE_SOURCE,
        ),
        ("postgres_import_stores.rs", POSTGRES_IMPORT_STORES_SOURCE),
        (
            "postgres_knowledge_document_metadata_transaction.rs",
            POSTGRES_KNOWLEDGE_DOCUMENT_METADATA_TRANSACTION_SOURCE,
        ),
        (
            "postgres_okf_candidate_transaction.rs",
            POSTGRES_OKF_CANDIDATE_TRANSACTION_SOURCE,
        ),
        (
            "postgres_okf_concept_revision_metadata_store.rs",
            POSTGRES_OKF_CONCEPT_REVISION_METADATA_STORE_SOURCE,
        ),
        (
            "postgres_okf_concept_transaction.rs",
            POSTGRES_OKF_CONCEPT_TRANSACTION_SOURCE,
        ),
        ("postgres_outbox_store.rs", POSTGRES_OUTBOX_STORE_SOURCE),
        ("postgres_space_stores.rs", POSTGRES_SPACE_STORES_SOURCE),
        (
            "wiki_persistence/checkpoint.rs",
            WIKI_CHECKPOINT_STORE_SOURCE,
        ),
        ("wiki_persistence/inbox.rs", WIKI_INBOX_STORE_SOURCE),
        (
            "wiki_persistence/projection.rs",
            WIKI_PROJECTION_STORE_SOURCE,
        ),
        (
            "wiki_persistence/publication.rs",
            WIKI_PUBLICATION_STORE_SOURCE,
        ),
        ("wiki_persistence/rendition.rs", WIKI_RENDITION_STORE_SOURCE),
    ] {
        assert!(
            !source.contains("AS TIMESTAMP)"),
            "{file} must use SqlTimestampDialect::sql_timestamp_expr instead of hard-coded PostgreSQL timestamp casts"
        );
        assert!(
            !source.contains("AS JSONB)"),
            "{file} must use SqlTimestampDialect::sql_json_expr instead of hard-coded PostgreSQL JSONB casts"
        );
    }

    assert!(
        [
            AGENT_PROFILE_STORE_SOURCE,
            AUDIT_EVENT_STORE_SOURCE,
            DRIVE_OBJECT_REF_STORE_SOURCE,
            INDEX_STORE_SOURCE,
            OKF_CONCEPT_LINK_STORE_SOURCE,
            OKF_CONCEPT_STORE_SOURCE,
            RETRIEVAL_PROFILE_STORE_SOURCE,
            RETRIEVAL_STORE_SOURCE,
            POSTGRES_COMMERCE_STORE_SOURCE,
            POSTGRES_CONTEXT_BINDING_STORE_SOURCE,
            POSTGRES_DRIVE_IMPORT_METADATA_STORE_SOURCE,
            POSTGRES_IMPORT_STORES_SOURCE,
            POSTGRES_KNOWLEDGE_DOCUMENT_METADATA_TRANSACTION_SOURCE,
            POSTGRES_OKF_CANDIDATE_TRANSACTION_SOURCE,
            POSTGRES_OKF_CONCEPT_REVISION_METADATA_STORE_SOURCE,
            POSTGRES_OKF_CONCEPT_TRANSACTION_SOURCE,
            POSTGRES_OUTBOX_STORE_SOURCE,
            POSTGRES_SPACE_STORES_SOURCE,
            WIKI_CHECKPOINT_STORE_SOURCE,
            WIKI_INBOX_STORE_SOURCE,
            WIKI_PROJECTION_STORE_SOURCE,
            WIKI_PUBLICATION_STORE_SOURCE,
            WIKI_RENDITION_STORE_SOURCE,
        ]
        .iter()
        .any(|source| source.contains("sql_timestamp_expr")),
        "runtime repositories must generate PostgreSQL timestamp casts through SqlTimestampDialect"
    );
    assert!(
        [
            AGENT_PROFILE_STORE_SOURCE,
            AUDIT_EVENT_STORE_SOURCE,
            OKF_CONCEPT_STORE_SOURCE,
            RETRIEVAL_STORE_SOURCE,
            POSTGRES_DRIVE_IMPORT_METADATA_STORE_SOURCE,
            POSTGRES_IMPORT_STORES_SOURCE,
            POSTGRES_KNOWLEDGE_DOCUMENT_METADATA_TRANSACTION_SOURCE,
            POSTGRES_OKF_CONCEPT_TRANSACTION_SOURCE,
            POSTGRES_OUTBOX_STORE_SOURCE,
        ]
        .iter()
        .any(|source| source.contains("sql_json_expr")),
        "runtime repositories must generate PostgreSQL JSONB casts through SqlTimestampDialect"
    );
    for (file, source, projection) in [
        (
            "postgres_import_stores.rs",
            POSTGRES_IMPORT_STORES_SOURCE,
            "CAST(metadata AS TEXT) AS metadata",
        ),
        (
            "postgres_knowledge_document_metadata_transaction.rs",
            POSTGRES_KNOWLEDGE_DOCUMENT_METADATA_TRANSACTION_SOURCE,
            "CAST(metadata AS TEXT) AS metadata",
        ),
        (
            "postgres_drive_import_metadata_store.rs",
            POSTGRES_DRIVE_IMPORT_METADATA_STORE_SOURCE,
            "CAST(metadata AS TEXT) AS metadata",
        ),
        (
            "postgres_okf_concept_transaction.rs",
            POSTGRES_OKF_CONCEPT_TRANSACTION_SOURCE,
            "CAST(tags AS TEXT) AS tags",
        ),
        (
            "postgres_okf_concept_revision_metadata_store.rs",
            POSTGRES_OKF_CONCEPT_REVISION_METADATA_STORE_SOURCE,
            "CAST(tags AS TEXT) AS tags",
        ),
        (
            "okf_concept_store.rs",
            OKF_CONCEPT_STORE_SOURCE,
            "CAST(tags AS TEXT) AS tags",
        ),
        (
            "okf_concept_store.rs",
            OKF_CONCEPT_STORE_SOURCE,
            "CAST(metadata AS TEXT) AS metadata",
        ),
        (
            "retrieval_store.rs",
            RETRIEVAL_STORE_SOURCE,
            "CAST(h.citation AS TEXT) AS citation",
        ),
        (
            "postgres_outbox_store.rs",
            POSTGRES_OUTBOX_STORE_SOURCE,
            "CAST(payload AS TEXT) AS payload",
        ),
    ] {
        assert!(
            source.contains(projection),
            "{file} must project PostgreSQL JSONB values as text before decoding them as Rust String"
        );
    }
}

#[test]
fn audit_event_migrations_define_kb_audit_event_table() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_AUDIT_EVENT_MIGRATION;

    {
        let migration = POSTGRES_AUDIT_EVENT_MIGRATION;
        let tables = defined_database_objects(migration, "CREATE TABLE IF NOT EXISTS ");
        assert!(tables.contains("kb_audit_event"));
        assert!(migration.contains("idx_kb_audit_event_tenant_created"));
        assert!(migration.contains("idx_kb_audit_event_event_type"));
    }
}

#[test]
fn outbox_claim_migrations_add_claimed_at_column() {
    use sdkwork_intelligence_knowledgebase_repository_sqlx::migrations::POSTGRES_OUTBOX_CLAIM_MIGRATION;

    {
        let migration = POSTGRES_OUTBOX_CLAIM_MIGRATION;
        assert!(migration.contains("claimed_at"));
        assert!(migration.contains("kb_outbox_event"));
    }
}

#[test]
fn outbox_claim_fencing_migrations_require_scope_owner_token_and_dead_letter_state() {
    for migration in [APP_ROOT_POSTGRES_OUTBOX_CLAIM_FENCING_MIGRATION] {
        for required in [
            "claim_owner",
            "claim_token",
            "dead_lettered_at",
            "organization_id",
            "idx_kb_outbox_event_scope_claim",
        ] {
            assert!(
                migration.contains(required),
                "migration is missing {required}"
            );
        }
    }
    assert!(
        APP_ROOT_POSTGRES_OUTBOX_CLAIM_FENCING_MIGRATION.contains("ck_kb_outbox_event_claim_pair")
    );
    assert!(
        APP_ROOT_POSTGRES_OUTBOX_CLAIM_FENCING_MIGRATION.contains("ck_kb_outbox_event_dead_letter")
    );
}

#[test]
fn group_aggregate_baseline_preserves_postgres_rls_for_every_tenant_table() {
    let rls_section_start = APP_ROOT_POSTGRES_BASELINE
        .find("FOR table_name IN")
        .expect("postgres baseline must define the group aggregate RLS loop");
    let rls_section = &APP_ROOT_POSTGRES_BASELINE[rls_section_start..];

    for table in [
        "kb_group_knowledge_space_binding",
        "kb_group_knowledge_space_member",
        "kb_group_knowledge_space_event_inbox",
        "kb_group_knowledge_space_membership_projection",
    ] {
        assert!(
            rls_section.contains(&format!("'{table}'")),
            "postgres baseline RLS loop must include {table}"
        );
    }
    for required_statement in [
        "ALTER TABLE %I ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE %I FORCE ROW LEVEL SECURITY",
        "CREATE POLICY tenant_isolation ON %I",
        "current_setting(''app.current_tenant_id'', true)::bigint",
    ] {
        assert!(
            rls_section.contains(required_statement),
            "postgres baseline RLS loop is missing {required_statement}"
        );
    }

    for table in [
        "kb_group_knowledge_space_binding",
        "kb_group_knowledge_space_member",
        "kb_group_knowledge_space_event_inbox",
    ] {
        assert!(
            POSTGRES_GROUP_KNOWLEDGE_SPACE_MIGRATION.contains(&format!("'{table}'")),
            "group aggregate migration must protect {table}"
        );
    }
    for required_statement in [
        "ALTER TABLE kb_group_knowledge_space_membership_projection ENABLE ROW LEVEL SECURITY",
        "ALTER TABLE kb_group_knowledge_space_membership_projection FORCE ROW LEVEL SECURITY",
        "CREATE POLICY tenant_isolation",
        "current_setting('app.current_tenant_id', true)::bigint",
    ] {
        assert!(
            POSTGRES_GROUP_MEMBERSHIP_PROJECTION_MIGRATION.contains(required_statement),
            "membership projection migration is missing {required_statement}"
        );
    }
}

#[test]
fn group_tenant_scope_upgrade_migrations_match_the_greenfield_contract() {
    assert!(
        APP_ROOT_POSTGRES_TENANT_SCOPE_MIGRATION
            .matches("CHECK (organization_id >= 0)")
            .count()
            >= 4,
        "PostgreSQL baseline must relax the group aggregate scope constraints",
    );
}

#[test]
fn live_wiki_postgres_tables_are_forced_behind_tenant_rls() {
    for table in LIVE_WIKI_TABLES {
        let table_literal = format!("'{table}'");
        assert!(
            APP_ROOT_POSTGRES_BASELINE.contains(&table_literal),
            "postgres baseline RLS inventory is missing {table}"
        );
        assert!(
            APP_ROOT_POSTGRES_LIVE_WIKI_MIGRATION.contains(&table_literal),
            "postgres migration RLS inventory is missing {table}"
        );
    }

    for source in [
        APP_ROOT_POSTGRES_BASELINE,
        APP_ROOT_POSTGRES_LIVE_WIKI_MIGRATION,
    ] {
        assert!(source.contains("ENABLE ROW LEVEL SECURITY"));
        assert!(source.contains("FORCE ROW LEVEL SECURITY"));
        assert!(source.contains("CREATE POLICY tenant_isolation"));
        assert!(source.contains("app.current_tenant_id"));
    }
}

#[test]
fn live_wiki_history_and_current_database_contract_versions_are_explicit() {
    for source in [
        APP_ROOT_POSTGRES_BASELINE,
        APP_ROOT_POSTGRES_LIVE_WIKI_MIGRATION,
        APP_ROOT_POSTGRES_LIVE_WIKI_ROLLBACK,
    ] {
        assert!(source.contains("contract_version: 1.1.0"));
    }
    // The folded baseline creates every live-wiki table (no ordered rollback exists in the
    // pre-launch consolidated schema).
    for table in LIVE_WIKI_TABLES {
        assert!(
            APP_ROOT_POSTGRES_BASELINE.contains(table),
            "baseline is missing {table}"
        );
    }

    assert!(APP_ROOT_POSTGRES_ORGANIZATION_ISOLATION_MIGRATION.contains("contract_version: 1.2.0"));
    assert!(APP_ROOT_POSTGRES_OUTBOX_CLAIM_FENCING_MIGRATION.contains("contract_version: 1.3.0"));
    assert!(APP_ROOT_DATABASE_MANIFEST.contains("\"contractVersion\": \"1.3.0\""));
    assert!(APP_ROOT_DATABASE_SCHEMA.contains("contract_version: 1.3.0"));
    for table in LIVE_WIKI_TABLES {
        assert!(
            APP_ROOT_DATABASE_SCHEMA.contains(&format!("name: {table}")),
            "schema contract is missing {table}"
        );
        assert!(
            APP_ROOT_DATABASE_TABLE_REGISTRY.contains(&format!("\"table_name\": \"{table}\"")),
            "table registry is missing {table}"
        );
    }
}

fn defined_database_objects(migration: &'static str, prefix: &str) -> BTreeSet<&'static str> {
    migration
        .lines()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .filter_map(|tail| tail.split_whitespace().next())
        .map(|object_name| object_name.trim_matches('"'))
        .collect()
}
