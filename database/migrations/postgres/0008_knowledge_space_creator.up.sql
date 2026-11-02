-- sdkwork:migration
-- id: 202611020001_knowledge_space_creator
-- engine: postgres
-- module: knowledgebase
-- purpose: Record the creating actor on kb_space so a knowledge space always has an
--   object-level owner. Space access previously depended only on the Drive node ACL
--   projection, which is written separately from the space row: a space created before
--   that projection existed, or whose projection is missing or stale, became unreachable
--   for the user who created it (`40301 actor does not have access to space ...`). The
--   creator is now admitted from the space record and the missing grant is repaired on
--   access. Fresh installs receive the column from the folded baseline; this migration
--   converges initialized environments and backfills the creator from the durable
--   creation-time actor already stored on the space's Wiki publication.
-- reversible: false
-- rollback: forward-fix (the column is additive and carries a constant default; dropping
--   it would discard ownership evidence, so repair forward instead)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 60s
-- contract_version: 1.7.0
-- rewrite_expectation: column addition with a constant default only; no table rewrite.
--   The backfill rewrites only rows with a derivable creator.
-- cancellation: cancel and rerun the idempotent ALTER and backfill
-- replication_impact: catalog change plus a bounded UPDATE of legacy space rows
-- recovery: rerun the idempotent statements after lock contention
--
-- No index is added: authorization loads the space by its scoped primary key
-- (tenant_id, organization_id, id) and compares created_by in memory. No query filters
-- or sorts by created_by, so a dedicated index would only add write cost.

BEGIN;

SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '60s';

ALTER TABLE kb_space
    ADD COLUMN IF NOT EXISTS created_by BIGINT NOT NULL DEFAULT 0;

-- Backfill from the durable creation-time actor recorded on the space's active Wiki
-- publication: `kb_site_publication.created_by` is written from the authenticated actor
-- in the same transaction that binds the space's Drive space, so it is the only
-- pre-existing owner evidence for a legacy row.
--
-- `kb_space` runs with FORCE ROW LEVEL SECURITY, so an unscoped migration connection
-- would silently match zero rows. Dropping FORCE for the length of this transaction lets
-- the backfill reach every scope, and the table-level lock it takes is released when the
-- transaction commits. ENABLE ROW LEVEL SECURITY and the organization policy stay in
-- place for business sessions the whole time, and the state is restored before COMMIT.
ALTER TABLE kb_space NO FORCE ROW LEVEL SECURITY;

UPDATE kb_space AS s
SET created_by = p.created_by
FROM kb_site_publication AS p
WHERE s.created_by = 0
  AND p.tenant_id = s.tenant_id
  AND p.organization_id = s.organization_id
  AND p.space_id = s.id
  AND p.status = 1
  AND p.created_by > 0;

ALTER TABLE kb_space FORCE ROW LEVEL SECURITY;

COMMIT;
