-- sdkwork:migration
-- id: 202610200001_agent_profile_owner
-- engine: postgres
-- module: knowledgebase
-- purpose: Record the creating actor on kb_agent_profile so a profile with no
--   enabled space binding still has an object-level owner. Previously the only
--   object-level check for an agent profile was the set of its enabled space
--   bindings, so a binding-less profile was readable, updatable, and deletable
--   by any authenticated user in the tenant. Fresh installs receive the column
--   from the folded baseline; this migration converges initialized environments.
-- reversible: false
-- rollback: forward-fix (the column is additive and carries a constant default;
--   dropping it would discard ownership evidence, so repair forward instead)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s
-- contract_version: 1.4.0
-- rewrite_expectation: column addition with a constant default only; no table
--   rewrite and no row backfill is required
-- cancellation: cancel and rerun the idempotent ALTER
-- replication_impact: catalog change only; existing rows read the constant
--   default without a WAL-heavy rewrite on supported PostgreSQL versions
-- recovery: restore the pre-migration snapshot or rerun the idempotent ALTER
--
-- No index is added: authorization loads the profile by its scoped primary key
-- (tenant_id, organization_id, id) and compares created_by in memory. No query
-- filters or sorts by created_by, so a dedicated index would only add write cost.

BEGIN;

ALTER TABLE kb_agent_profile
    ADD COLUMN IF NOT EXISTS created_by BIGINT NOT NULL DEFAULT 0;

COMMIT;
