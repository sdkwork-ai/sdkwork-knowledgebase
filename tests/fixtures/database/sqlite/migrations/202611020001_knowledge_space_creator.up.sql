-- sdkwork:migration
-- id: 202611020001_knowledge_space_creator
-- engine: sqlite
-- module: knowledgebase
-- purpose: Test-fixture mirror of the PostgreSQL knowledge-space creator column
--          (database/migrations/postgres/0008_knowledge_space_creator.up.sql, folded
--          into the PostgreSQL baseline): record the creating actor so a space always
--          has an object-level owner instead of relying only on the Drive ACL
--          projection.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: lightweight
-- contract_version: 1.7.0

-- The fixture baseline predates the ownership cutover, so the column arrives through
-- this migration and the effective fixture schema stays identical to the folded
-- PostgreSQL baseline. The PostgreSQL backfill has no fixture analogue: the reference
-- schema carries no RLS session scope and no seeded legacy space rows.
ALTER TABLE kb_space ADD COLUMN created_by INTEGER NOT NULL DEFAULT 0;
