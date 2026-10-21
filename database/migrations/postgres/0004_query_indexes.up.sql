-- sdkwork:migration
-- id: 202610200002_query_indexes
-- engine: postgres
-- module: knowledgebase
-- purpose: Cover the document list keyset, the durable post-ingest embedding
--   backlog scan, and the Wiki source-processing claim query with indexes that
--   match the predicates the repository actually issues. Without them each page
--   or worker claim scans every document in the tenant organization.
-- reversible: false
-- rollback: forward-fix (indexes are additive; dropping them only restores the
--   slower plan and loses the performance evidence, so repair forward instead)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s
-- contract_version: 1.4.0
-- rewrite_expectation: CREATE INDEX only; no table rewrite and no data change
-- cancellation: cancel and rerun the idempotent CREATE INDEX statements
-- replication_impact: index build WAL only for the pre-launch dataset
-- recovery: rerun after resolving lock contention; indexes are idempotent

BEGIN;

-- Space document listing: WHERE tenant, organization, space, status ORDER BY id LIMIT n.
CREATE INDEX IF NOT EXISTS idx_kb_document_scope_space_status_id
    ON kb_document (tenant_id, organization_id, space_id, status, id);

-- Post-ingest embedding backlog: pending/running documents with a current version,
-- ordered by updated_at for oldest-first draining.
CREATE INDEX IF NOT EXISTS idx_kb_document_scope_index_state
    ON kb_document (tenant_id, organization_id, status, index_state, updated_at, id);

-- Wiki source-processing claim: scoped by publication and processing state, ordered by id.
CREATE INDEX IF NOT EXISTS idx_kb_source_projection_claimable_scope
    ON kb_source_file_projection (
        tenant_id,
        organization_id,
        site_publication_id,
        source_state,
        next_processing_at,
        id
    )
    WHERE status = 1
      AND source_state IN ('DISCOVERED', 'QUEUED', 'ERROR', 'READY');

COMMIT;
