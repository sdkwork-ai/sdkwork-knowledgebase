-- sdkwork:migration
-- id: 202608040002_audit_event_scope_actor_index
-- engine: sqlite
-- module: knowledgebase
-- purpose: Composite (tenant, organization, actor, created_at) index serving the backend
--          audit-event cursor list, mirroring the PostgreSQL folded baseline
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: medium
-- contract_version: 1.3.0

CREATE INDEX IF NOT EXISTS idx_kb_audit_event_scope_actor_created
    ON kb_audit_event (tenant_id, organization_id, actor_id, created_at DESC, id DESC);
