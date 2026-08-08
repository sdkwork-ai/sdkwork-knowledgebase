-- sdkwork:migration
-- id: 202607310002_outbox_claim_fencing
-- engine: sqlite
-- module: knowledgebase
-- purpose: Fence outbox claim completion by owner/token and persist exhausted deliveries,
--          mirroring the PostgreSQL folded baseline outbox claim-fencing section
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: heavyweight
-- contract_version: 1.3.0

ALTER TABLE kb_outbox_event ADD COLUMN claim_owner TEXT;
ALTER TABLE kb_outbox_event ADD COLUMN claim_token TEXT;
ALTER TABLE kb_outbox_event ADD COLUMN dead_lettered_at TEXT;

-- Reset only stale in-flight claims (claimed longer than the five-minute stale window).
-- Claims still owned by an active worker are preserved so a mid-delivery worker is never
-- silently fenced into duplicate delivery. `claimed_at` is stored as RFC3339 UTC text;
-- SQLite `datetime()` parses it when the `T` separator is normalized to a space.
UPDATE kb_outbox_event
SET status = 0, claimed_at = NULL, claim_owner = NULL, claim_token = NULL
WHERE status = 3
  AND claimed_at IS NOT NULL
  AND replace(claimed_at, 'T', ' ') < datetime('now', '-5 minutes');

-- The PostgreSQL claim-pair and dead-letter CHECK constraints cannot be added through
-- SQLite `ALTER TABLE`; the store layer enforces the same invariants transactionally.
CREATE INDEX IF NOT EXISTS idx_kb_outbox_event_scope_claim
    ON kb_outbox_event (tenant_id, organization_id, status, claimed_at, id);
CREATE INDEX IF NOT EXISTS idx_kb_outbox_event_scope_dead_letter
    ON kb_outbox_event (tenant_id, organization_id, dead_lettered_at, id)
    WHERE status = 4;
