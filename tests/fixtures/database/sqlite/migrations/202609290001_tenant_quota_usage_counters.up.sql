-- sdkwork:migration
-- id: 202609290001_tenant_quota_usage_counters
-- engine: sqlite
-- module: knowledgebase
-- purpose: Test-fixture mirror of the PostgreSQL tenant quota usage counters
--          (database/migrations/postgres/0002_tenant_quota_usage_counters.up.sql,
--          folded into the PostgreSQL baseline): exact document-count and
--          storage-bytes counters maintained by row triggers so the fixture
--          executes the same quota SQL with the same semantics.
-- reversible: false
-- rollback: forward-fix (drop triggers, recompute counters from base tables)
-- transactional: true

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
WHERE true
ON CONFLICT (tenant_id, organization_id) DO NOTHING;

CREATE TRIGGER IF NOT EXISTS trg_kb_document_quota_usage_insert
AFTER INSERT ON kb_document
FOR EACH ROW
WHEN NEW.status = 1
BEGIN
    INSERT INTO kb_tenant_quota_usage (tenant_id, organization_id, document_count, storage_bytes)
    VALUES (NEW.tenant_id, NEW.organization_id, 1, 0)
    ON CONFLICT (tenant_id, organization_id) DO UPDATE SET
        document_count = document_count + 1;
END;

CREATE TRIGGER IF NOT EXISTS trg_kb_document_quota_usage_delete
AFTER DELETE ON kb_document
FOR EACH ROW
WHEN OLD.status = 1
BEGIN
    UPDATE kb_tenant_quota_usage
    SET document_count = document_count - 1
    WHERE tenant_id = OLD.tenant_id AND organization_id = OLD.organization_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_kb_document_quota_usage_update
AFTER UPDATE OF status ON kb_document
FOR EACH ROW
WHEN OLD.status <> NEW.status
BEGIN
    -- The proposed insert row must satisfy the non-negative CHECK before the
    -- conflict branch is considered, so VALUES carries MAX(delta, 0) and the
    -- full delta (including soft deletes) is applied in DO UPDATE arithmetic.
    -- A negative delta always implies the counter row already exists (a
    -- scope's first event is an active insert).
    INSERT INTO kb_tenant_quota_usage (tenant_id, organization_id, document_count, storage_bytes)
    VALUES (
        NEW.tenant_id,
        NEW.organization_id,
        MAX((NEW.status = 1) - (OLD.status = 1), 0),
        0
    )
    ON CONFLICT (tenant_id, organization_id) DO UPDATE SET
        document_count = document_count + ((NEW.status = 1) - (OLD.status = 1));
END;

CREATE TRIGGER IF NOT EXISTS trg_kb_drive_object_ref_quota_usage_insert
AFTER INSERT ON kb_drive_object_ref
FOR EACH ROW
WHEN NEW.status = 1
BEGIN
    INSERT INTO kb_tenant_quota_usage (tenant_id, organization_id, document_count, storage_bytes)
    VALUES (NEW.tenant_id, NEW.organization_id, 0, NEW.size_bytes)
    ON CONFLICT (tenant_id, organization_id) DO UPDATE SET
        storage_bytes = storage_bytes + NEW.size_bytes;
END;

CREATE TRIGGER IF NOT EXISTS trg_kb_drive_object_ref_quota_usage_delete
AFTER DELETE ON kb_drive_object_ref
FOR EACH ROW
WHEN OLD.status = 1
BEGIN
    UPDATE kb_tenant_quota_usage
    SET storage_bytes = storage_bytes - OLD.size_bytes
    WHERE tenant_id = OLD.tenant_id AND organization_id = OLD.organization_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_kb_drive_object_ref_quota_usage_update
AFTER UPDATE ON kb_drive_object_ref
FOR EACH ROW
WHEN OLD.status <> NEW.status OR OLD.size_bytes <> NEW.size_bytes
BEGIN
    -- Same non-negative-VALUES discipline as the document trigger: the CHECK
    -- is evaluated on the proposed insert row before conflict handling.
    INSERT INTO kb_tenant_quota_usage (tenant_id, organization_id, document_count, storage_bytes)
    VALUES (
        NEW.tenant_id,
        NEW.organization_id,
        0,
        MAX(
            (CASE WHEN NEW.status = 1 THEN NEW.size_bytes ELSE 0 END)
            - (CASE WHEN OLD.status = 1 THEN OLD.size_bytes ELSE 0 END),
            0
        )
    )
    ON CONFLICT (tenant_id, organization_id) DO UPDATE SET
        storage_bytes = storage_bytes
            + (CASE WHEN NEW.status = 1 THEN NEW.size_bytes ELSE 0 END)
            - (CASE WHEN OLD.status = 1 THEN OLD.size_bytes ELSE 0 END);
END;

CREATE INDEX IF NOT EXISTS idx_kb_document_tenant_org_status
    ON kb_document (tenant_id, organization_id, status);
CREATE INDEX IF NOT EXISTS idx_kb_drive_object_ref_tenant_org_status_size
    ON kb_drive_object_ref (tenant_id, organization_id, status, size_bytes);
