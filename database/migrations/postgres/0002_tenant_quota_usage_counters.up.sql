-- sdkwork:migration
-- id: 0002_tenant_quota_usage_counters
-- engine: postgres
-- module: sdkwork-knowledgebase
-- purpose: Exact tenant quota counters (document count, storage bytes) maintained
--   by row triggers so quota enforcement inside the advisory-locked write
--   transaction is O(1) per write instead of one full COUNT/SUM aggregate scan
--   per write. The aggregate scans previously held the tenant advisory lock for
--   O(tenant rows) on every document, drive object, and import write, which
--   serializes tenant ingestion on scan time as tenants grow.
-- reversible: false
-- rollback: forward-fix (drop triggers, recompute counters from base tables)
-- transactional: true
-- lock: medium
-- lock_timeout: 2s
-- statement_timeout: 2min

BEGIN;

SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '2min';

CREATE TABLE IF NOT EXISTS kb_tenant_quota_usage (
    tenant_id BIGINT NOT NULL,
    organization_id BIGINT NOT NULL DEFAULT 0,
    document_count BIGINT NOT NULL DEFAULT 0,
    storage_bytes BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_kb_tenant_quota_usage PRIMARY KEY (tenant_id, organization_id),
    CONSTRAINT ck_kb_tenant_quota_usage_non_negative CHECK (
        document_count >= 0 AND storage_bytes >= 0
    )
);

-- One-time (idempotent) backfill from the authoritative base tables. Runs before
-- RLS/trigger installation below so unscoped bootstrap sessions can never be
-- rejected by the fail-closed policy; ON CONFLICT DO NOTHING keeps re-runs inert.
INSERT INTO kb_tenant_quota_usage (tenant_id, organization_id, document_count, storage_bytes)
SELECT pairs.tenant_id,
       pairs.organization_id,
       (
           SELECT COUNT(*)
           FROM kb_document d
           WHERE d.tenant_id = pairs.tenant_id
             AND d.organization_id = pairs.organization_id
             AND d.status = 1
       ),
       (
           SELECT COALESCE(SUM(r.size_bytes), 0)
           FROM kb_drive_object_ref r
           WHERE r.tenant_id = pairs.tenant_id
             AND r.organization_id = pairs.organization_id
             AND r.status = 1
       )
FROM (
    SELECT tenant_id, organization_id FROM kb_document
    UNION
    SELECT tenant_id, organization_id FROM kb_drive_object_ref
) AS pairs
ON CONFLICT (tenant_id, organization_id) DO NOTHING;

CREATE OR REPLACE FUNCTION kb_quota_usage_apply_document_delta()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    delta bigint;
    scope_tenant_id bigint;
    scope_organization_id bigint;
BEGIN
    IF TG_OP = 'INSERT' THEN
        scope_tenant_id := NEW.tenant_id;
        scope_organization_id := NEW.organization_id;
        delta := CASE WHEN NEW.status = 1 THEN 1 ELSE 0 END;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.tenant_id IS DISTINCT FROM NEW.tenant_id
            OR OLD.organization_id IS DISTINCT FROM NEW.organization_id THEN
            RAISE EXCEPTION 'kb_document tenant/organization scope columns are immutable'
                USING ERRCODE = 'restrict_violation';
        END IF;
        scope_tenant_id := NEW.tenant_id;
        scope_organization_id := NEW.organization_id;
        delta := (CASE WHEN NEW.status = 1 THEN 1 ELSE 0 END)
               - (CASE WHEN OLD.status = 1 THEN 1 ELSE 0 END);
    ELSE
        scope_tenant_id := OLD.tenant_id;
        scope_organization_id := OLD.organization_id;
        delta := CASE WHEN OLD.status = 1 THEN -1 ELSE 0 END;
    END IF;

    IF delta = 0 THEN
        RETURN NULL;
    END IF;

    -- The proposed insert row must satisfy the non-negative CHECK before the
    -- conflict branch is considered, so the VALUES clause only ever carries the
    -- non-negative part of the delta; the full delta (including negative
    -- transitions such as soft deletes) is applied in DO UPDATE arithmetic.
    -- A negative delta always implies the counter row already exists (a scope's
    -- first event is an active insert, and the migration backfills every
    -- existing scope), so no delta is lost on the insert path.
    INSERT INTO kb_tenant_quota_usage AS usage
        (tenant_id, organization_id, document_count, storage_bytes)
    VALUES (scope_tenant_id, scope_organization_id, GREATEST(delta, 0), 0)
    ON CONFLICT (tenant_id, organization_id)
    DO UPDATE SET
        document_count = usage.document_count + delta,
        updated_at = CURRENT_TIMESTAMP;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION kb_quota_usage_apply_storage_delta()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    delta bigint;
    scope_tenant_id bigint;
    scope_organization_id bigint;
BEGIN
    IF TG_OP = 'INSERT' THEN
        scope_tenant_id := NEW.tenant_id;
        scope_organization_id := NEW.organization_id;
        delta := CASE WHEN NEW.status = 1 THEN NEW.size_bytes ELSE 0 END;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.tenant_id IS DISTINCT FROM NEW.tenant_id
            OR OLD.organization_id IS DISTINCT FROM NEW.organization_id THEN
            RAISE EXCEPTION 'kb_drive_object_ref tenant/organization scope columns are immutable'
                USING ERRCODE = 'restrict_violation';
        END IF;
        scope_tenant_id := NEW.tenant_id;
        scope_organization_id := NEW.organization_id;
        delta := (CASE WHEN NEW.status = 1 THEN NEW.size_bytes ELSE 0 END)
               - (CASE WHEN OLD.status = 1 THEN OLD.size_bytes ELSE 0 END);
    ELSE
        scope_tenant_id := OLD.tenant_id;
        scope_organization_id := OLD.organization_id;
        delta := CASE WHEN OLD.status = 1 THEN -OLD.size_bytes ELSE 0 END;
    END IF;

    IF delta = 0 THEN
        RETURN NULL;
    END IF;

    -- Same non-negative-VALUES discipline as the document trigger: the CHECK
    -- is evaluated on the proposed insert row before conflict handling, so the
    -- VALUES clause carries GREATEST(delta, 0) and the full delta (including
    -- removals) is applied in DO UPDATE arithmetic.
    INSERT INTO kb_tenant_quota_usage AS usage
        (tenant_id, organization_id, document_count, storage_bytes)
    VALUES (scope_tenant_id, scope_organization_id, 0, GREATEST(delta, 0))
    ON CONFLICT (tenant_id, organization_id)
    DO UPDATE SET
        storage_bytes = usage.storage_bytes + delta,
        updated_at = CURRENT_TIMESTAMP;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS trg_kb_document_quota_usage ON kb_document;
CREATE TRIGGER trg_kb_document_quota_usage
    AFTER INSERT OR UPDATE OR DELETE ON kb_document
    FOR EACH ROW
    EXECUTE FUNCTION kb_quota_usage_apply_document_delta();

DROP TRIGGER IF EXISTS trg_kb_drive_object_ref_quota_usage ON kb_drive_object_ref;
CREATE TRIGGER trg_kb_drive_object_ref_quota_usage
    AFTER INSERT OR UPDATE OR DELETE ON kb_drive_object_ref
    FOR EACH ROW
    EXECUTE FUNCTION kb_quota_usage_apply_storage_delta();

-- Fallback scans (cold counter rows and drift repair) and other tenant-scoped
-- status queries stay index-only as tenants grow.
CREATE INDEX IF NOT EXISTS idx_kb_document_tenant_org_status
    ON kb_document (tenant_id, organization_id, status);
CREATE INDEX IF NOT EXISTS idx_kb_drive_object_ref_tenant_org_status_size
    ON kb_drive_object_ref (tenant_id, organization_id, status, size_bytes);

-- Installed last so the backfill above never fights the fail-closed policy in
-- unscoped bootstrap sessions. Business sessions always carry the tenant scope
-- variables, so trigger-maintained counter writes pass WITH CHECK.
DO $$
BEGIN
    EXECUTE 'ALTER TABLE kb_tenant_quota_usage ENABLE ROW LEVEL SECURITY';
    EXECUTE 'ALTER TABLE kb_tenant_quota_usage FORCE ROW LEVEL SECURITY';
    EXECUTE 'DROP POLICY IF EXISTS tenant_isolation ON kb_tenant_quota_usage';
    EXECUTE 'DROP POLICY IF EXISTS organization_isolation ON kb_tenant_quota_usage';
    EXECUTE 'CREATE POLICY organization_isolation ON kb_tenant_quota_usage AS PERMISSIVE FOR ALL TO PUBLIC USING (tenant_id = NULLIF(current_setting(''app.current_tenant_id'', true), '''')::bigint AND organization_id = NULLIF(current_setting(''app.current_organization_id'', true), '''')::bigint) WITH CHECK (tenant_id = NULLIF(current_setting(''app.current_tenant_id'', true), '''')::bigint AND organization_id = NULLIF(current_setting(''app.current_organization_id'', true), '''')::bigint)';
END $$;

COMMIT;
