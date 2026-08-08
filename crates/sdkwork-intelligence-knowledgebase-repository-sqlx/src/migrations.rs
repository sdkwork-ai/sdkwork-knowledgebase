pub const POSTGRES_CORE_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606010001__knowledgebase_core.sql");

pub const POSTGRES_ACCESS_MODE_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606170001__knowledge_access_mode.sql");

pub const POSTGRES_AGENT_IMPLEMENTATION_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606180001__agent_implementation.sql");

pub const POSTGRES_CONTEXT_BINDING_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606140001__knowledgebase_context_binding.sql");

pub const POSTGRES_PGVECTOR_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606190001__knowledgebase_pgvector.sql");

pub const POSTGRES_OUTBOX_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606200001__knowledgebase_outbox.sql");

pub const POSTGRES_OKF_LINK_CANDIDATE_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606210001__okf_link_and_candidate.sql");

pub const POSTGRES_OUTBOX_DELIVERY_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606220001__knowledgebase_outbox_delivery.sql");

pub const POSTGRES_OUTBOX_CLAIM_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606220003__knowledgebase_outbox_claim.sql");

pub const POSTGRES_CHUNK_FTS_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606220002__knowledgebase_chunk_fts.sql");

pub const POSTGRES_PERFORMANCE_INDEXES_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606230001__knowledgebase_performance_indexes.sql");

pub const POSTGRES_MARKET_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606240001__knowledge_market.sql");

pub const POSTGRES_AUDIT_EVENT_MIGRATION: &str =
    include_str!("../migrations/postgres/V202606250001__knowledgebase_audit_event.sql");

pub const POSTGRES_GROUP_KNOWLEDGE_SPACE_MIGRATION: &str =
    include_str!("../migrations/postgres/V202607130001__group_knowledge_space.sql");

pub const POSTGRES_GROUP_MEMBERSHIP_PROJECTION_MIGRATION: &str =
    include_str!("../migrations/postgres/V202607130002__group_membership_projection.sql");

pub const POSTGRES_GROUP_ARCHIVE_SAGA_MIGRATION: &str = include_str!(
    "../migrations/postgres/V202607130003__group_archive_saga_and_scope_integrity.sql"
);

pub const POSTGRES_INGESTION_JOB_LEASE_MIGRATION: &str =
    include_str!("../migrations/postgres/V202607160002__ingestion_job_lease.sql");

pub const POSTGRES_LIVE_WIKI_PUBLICATION_MIGRATION: &str =
    include_str!("../migrations/postgres/V202607210001__live_wiki_publication.sql");

// Legacy migration SQL retained for contract tests only. Runtime PostgreSQL bootstrap uses
// application-root `database/` via `sdkwork-knowledgebase-database-host`; the folded
// baseline there is the schema authority and carries the post-baseline migrations
// (organization isolation, outbox claim fencing, retry backoff, audit scope index), so no
// aggregated `POSTGRES_MIGRATIONS` list is maintained in this crate.
