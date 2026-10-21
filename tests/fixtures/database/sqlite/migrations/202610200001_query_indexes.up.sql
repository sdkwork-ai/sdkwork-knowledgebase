-- sdkwork:migration
-- id: 202610200001_query_indexes
-- engine: sqlite
-- module: knowledgebase
-- purpose: Mirror the PostgreSQL 0004 query-shape indexes for the SQLite reference schema.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- contract_version: 1.4.0

CREATE INDEX IF NOT EXISTS idx_kb_document_scope_space_status_id
    ON kb_document (tenant_id, organization_id, space_id, status, id);
CREATE INDEX IF NOT EXISTS idx_kb_document_scope_index_state
    ON kb_document (tenant_id, organization_id, status, index_state, updated_at, id);
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_claimable_scope
    ON kb_source_file_projection (
        tenant_id, organization_id, site_publication_id, source_state, next_processing_at, id
    ) WHERE status = 1 AND source_state IN ('DISCOVERED', 'QUEUED', 'ERROR', 'READY');
-- SQLite reference equivalent of the PostgreSQL jsonb_path_ops GIN index.
CREATE INDEX IF NOT EXISTS idx_kb_document_metadata_gin
    ON kb_document (metadata);
CREATE TABLE IF NOT EXISTS kb_audit_legal_hold (
    id INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL,
    tenant_id INTEGER NOT NULL,
    organization_id INTEGER NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    case_reference TEXT,
    status INTEGER NOT NULL DEFAULT 1,
    created_by TEXT NOT NULL,
    released_by TEXT,
    released_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_audit_legal_hold_uuid
    ON kb_audit_legal_hold (tenant_id, organization_id, uuid);
CREATE INDEX IF NOT EXISTS idx_kb_audit_legal_hold_subject
    ON kb_audit_legal_hold (tenant_id, organization_id, status, subject_type, subject_id);
CREATE INDEX IF NOT EXISTS idx_kb_audit_event_scope_created_id
    ON kb_audit_event (tenant_id, organization_id, created_at, id);
