-- sdkwork:migration
-- id: 202610210002_document_metadata_gin
-- engine: postgres
-- module: knowledgebase
-- purpose: Support retrieval document metadata filters with an index. The filter
--   compiles to `metadata @> jsonb_build_object(key, value)`, which `jsonb_path_ops`
--   serves directly; the previous `metadata ->> key = value` form could not use an
--   index for variable keys.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: build-index
-- lock_timeout: 5s
-- statement_timeout: 300s
-- contract_version: 1.5.0
-- rewrite_expectation: GIN index build; no table rewrite
-- cancellation: cancel and rerun `CREATE INDEX IF NOT EXISTS`
-- replication_impact: index build is WAL-logged; prefer a maintenance window on large tables
-- recovery: drop and rebuild the index if the build is interrupted

CREATE INDEX IF NOT EXISTS idx_kb_document_metadata_gin
    ON kb_document USING gin (metadata jsonb_path_ops);
