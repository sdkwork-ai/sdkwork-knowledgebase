-- sdkwork:migration
-- id: 202608040001_outbox_retry_backoff
-- engine: sqlite
-- module: knowledgebase
-- purpose: Add scheduled retry (`next_attempt_at`) to outbox events, mirroring the
--          PostgreSQL folded baseline retry-backoff section
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: medium
-- contract_version: 1.3.0

ALTER TABLE kb_outbox_event ADD COLUMN next_attempt_at TEXT;

-- Serve the worker's requeue scan (status + due-time predicate).
CREATE INDEX IF NOT EXISTS idx_kb_outbox_event_scope_status_retry
    ON kb_outbox_event (tenant_id, organization_id, status, next_attempt_at);
