-- sdkwork:migration
-- id: 202610220001_audit_retention_legal_hold
-- engine: postgres
-- module: knowledgebase
-- purpose: Enable bounded, auditable audit-event retention. Adds subject-scoped legal
--   holds that exempt matching events from purge, plus the (tenant, organization, created_at)
--   scan index used by the batched purge and a unique scope index for hold identity.
-- reversible: false
-- rollback: forward-fix
-- transactional: true
-- lock: lightweight
-- lock_timeout: 3s
-- statement_timeout: 60s
-- contract_version: 1.6.0
-- rewrite_expectation: new table and indexes only; no table rewrite
-- cancellation: cancel and rerun the idempotent CREATE statements
-- replication_impact: catalog changes and index builds on hold/purge columns
-- recovery: rerun the idempotent statements after lock contention

SET LOCAL lock_timeout = '3s';
SET LOCAL statement_timeout = '60s';

CREATE TABLE IF NOT EXISTS kb_audit_legal_hold (
    id BIGINT PRIMARY KEY,
    uuid VARCHAR(64) NOT NULL,
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    subject_type VARCHAR(32) NOT NULL,
    subject_id VARCHAR(128) NOT NULL,
    reason VARCHAR(512) NOT NULL,
    case_reference VARCHAR(128),
    status INTEGER NOT NULL DEFAULT 1,
    created_by VARCHAR(128) NOT NULL,
    released_by VARCHAR(128),
    released_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS uk_kb_audit_legal_hold_uuid
    ON kb_audit_legal_hold (tenant_id, organization_id, uuid);

CREATE INDEX IF NOT EXISTS idx_kb_audit_legal_hold_subject
    ON kb_audit_legal_hold (tenant_id, organization_id, status, subject_type, subject_id);

CREATE INDEX IF NOT EXISTS idx_kb_audit_event_scope_created_id
    ON kb_audit_event (tenant_id, organization_id, created_at, id);

-- New tables are not covered by the earlier RLS loop on initialized environments.
ALTER TABLE kb_audit_legal_hold ENABLE ROW LEVEL SECURITY;
ALTER TABLE kb_audit_legal_hold FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation ON kb_audit_legal_hold;
DROP POLICY IF EXISTS organization_isolation ON kb_audit_legal_hold;
CREATE POLICY organization_isolation ON kb_audit_legal_hold AS PERMISSIVE FOR ALL TO PUBLIC
    USING (
        tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::bigint
        AND organization_id = NULLIF(current_setting('app.current_organization_id', true), '')::bigint
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::bigint
        AND organization_id = NULLIF(current_setting('app.current_organization_id', true), '')::bigint
    );
