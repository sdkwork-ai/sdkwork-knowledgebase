-- sdkwork:migration
-- id: 202607310001_organization_isolation
-- engine: sqlite
-- module: knowledgebase
-- purpose: Add non-null organization ownership to legacy business tables, mirroring the
--          PostgreSQL folded baseline `0001_knowledgebase_baseline.sql` organization
--          isolation section
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: heavyweight
-- contract_version: 1.3.0

-- Organization ownership columns (SQLite `ALTER TABLE ADD COLUMN` supports a constant
-- `NOT NULL DEFAULT`, matching the PostgreSQL `SET NOT NULL` end state on a fixture that
-- is created fresh and therefore has no rows to backfill).
ALTER TABLE kb_collection ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_source ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_drive_object_ref ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_document ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_document_version ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_chunk ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_index ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_embedding ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_retrieval_profile ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_retrieval_trace ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_retrieval_hit ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_agent_profile ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_agent_knowledge_binding ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_ingestion_job ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_ingestion_job_item ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_concept ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_concept_revision ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_bundle_file ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_schema_profile ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_log_entry ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_local_mirror_package ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_space_context_binding ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_outbox_event ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_concept_link ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_okf_candidate ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_market_listing ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_market_subscription ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE kb_audit_event ADD COLUMN organization_id INTEGER NOT NULL DEFAULT 0;

-- Organization-scoped indexes serving the same cursor list and claim queries as the
-- PostgreSQL folded section. Vector/HNSW/GIN indexes are intentionally omitted: the
-- SQLite fixture stores vectors as JSON text and never executes similarity search.
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_space_scope_id
    ON kb_space (tenant_id, organization_id, id);
CREATE INDEX IF NOT EXISTS idx_kb_source_scope_active
    ON kb_source (tenant_id, organization_id, space_id, status, id);
CREATE INDEX IF NOT EXISTS idx_kb_chunk_scope_search
    ON kb_chunk (tenant_id, organization_id, space_id, collection_id, status, id);
CREATE INDEX IF NOT EXISTS idx_kb_embedding_scope_chunk
    ON kb_embedding (tenant_id, organization_id, chunk_id, status, id);
CREATE INDEX IF NOT EXISTS idx_kb_retrieval_trace_scope_id
    ON kb_retrieval_trace (tenant_id, organization_id, id);
CREATE INDEX IF NOT EXISTS idx_kb_retrieval_hit_scope_trace_rank
    ON kb_retrieval_hit (tenant_id, organization_id, retrieval_trace_id, result_rank, id);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_index_active_scope_kind
    ON kb_index (tenant_id, organization_id, space_id, index_kind)
    WHERE status = 1;
