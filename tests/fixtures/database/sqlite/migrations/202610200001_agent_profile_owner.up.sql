-- sdkwork:migration
-- id: 202610200001_agent_profile_owner
-- engine: sqlite
-- module: knowledgebase
-- purpose: Test-fixture mirror of the PostgreSQL agent-profile ownership column
--          (database/migrations/postgres/0003_agent_profile_owner.up.sql, folded
--          into the PostgreSQL baseline): record the creating actor so a profile
--          without enabled space bindings has an object-level owner.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: lightweight
-- contract_version: 1.4.0

-- The fixture baseline predates the ownership cutover (like the organization
-- isolation columns), so the column arrives through this migration and the
-- effective fixture schema stays identical to the folded PostgreSQL baseline.
ALTER TABLE kb_agent_profile ADD COLUMN created_by INTEGER NOT NULL DEFAULT 0;
