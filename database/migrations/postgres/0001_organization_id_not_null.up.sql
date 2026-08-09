-- sdkwork:migration
-- id: 0001_organization_id_not_null
-- engine: postgres
-- module: sdkwork-knowledgebase
-- purpose: Enforce organization_id NOT NULL DEFAULT on all tables in the
--   consolidated baseline. NULL rows (pre-standard data anomalies) are
--   backfilled with the platform sentinel before NOT NULL is set, and
--   NOT NULL columns without an explicit default receive the sentinel
--   default, keeping existing deployments consistent with fresh baseline
--   installs.
-- reversible: false
-- rollback: forward-fix (sentinel backfill is the canonical fix; NULL
--   organization rows are data anomalies)
-- transactional: true
-- lock: lightweight
-- lock_timeout: 2s
-- statement_timeout: 30s

BEGIN;

UPDATE kb_space SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_space ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_space ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_group_knowledge_space_binding SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_group_knowledge_space_binding ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_group_knowledge_space_binding ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_group_knowledge_space_member SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_group_knowledge_space_member ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_group_knowledge_space_member ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_group_knowledge_space_event_inbox SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_group_knowledge_space_event_inbox ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_group_knowledge_space_event_inbox ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_group_knowledge_space_membership_projection SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_group_knowledge_space_membership_projection ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_group_knowledge_space_membership_projection ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_site_publication SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_site_publication ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_site_publication ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_source_file_projection SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_source_file_projection ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_source_file_projection ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_source_file_rendition SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_source_file_rendition ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_source_file_rendition ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_drive_source_checkpoint SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_drive_source_checkpoint ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_drive_source_checkpoint ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_drive_event_inbox SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_drive_event_inbox ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_drive_event_inbox ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_provider_credential_reference SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_provider_credential_reference ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_provider_credential_reference ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_provider_binding SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_provider_binding ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_provider_binding ALTER COLUMN organization_id SET NOT NULL;

UPDATE kb_provider_migration_operation SET organization_id = 0 WHERE organization_id IS NULL;
ALTER TABLE kb_provider_migration_operation ALTER COLUMN organization_id SET DEFAULT 0;
ALTER TABLE kb_provider_migration_operation ALTER COLUMN organization_id SET NOT NULL;

COMMIT;
