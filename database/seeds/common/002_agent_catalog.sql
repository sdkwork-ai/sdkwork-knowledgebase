-- SDKWork official agent catalog: knowledge-space identities (neutral base).

-- Catalog revision: 1.2.1.

BEGIN;

SELECT set_config('app.current_tenant_id', '100001', true);

SELECT set_config('app.current_organization_id', '0', true);

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000001, '294668ef-f835-4142-88d4-00a517abc494', 100001, 0,
  'knowledge.base.programmer-fullstack', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000002, '707c653e-677e-42f4-8814-eabe0eb09a40', 100001, 0,
  'knowledge.base.programmer-frontend', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000003, 'bb787e5c-c848-4c81-8d0d-d025652149dd', 100001, 0,
  'knowledge.base.edu-k12-math', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000004, 'b1190d8d-17e6-4349-8fe5-5c820baf8e10', 100001, 0,
  'knowledge.base.edu-uni-cs', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000005, 'a8409f37-0fd0-4c50-8023-3bc636e4dd23', 100001, 0,
  'knowledge.base.doctor-general', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000006, 'f2c04e92-fbb1-4e62-80c9-1cbb99ecd149', 100001, 0,
  'knowledge.base.lawyer-civil', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000007, '92a572af-0dc4-427f-8721-558b0291a044', 100001, 0,
  'knowledge.base.travel-planner', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000008, 'c2c85baa-6457-4115-8c94-da35f2e0faed', 100001, 0,
  'knowledge.base.ecommerce-operations', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000009, '3444424f-2288-44e8-8874-3ca2eaad3093', 100001, 0,
  'knowledge.base.realestate-buyer', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000010, '5a18ff2b-a439-47c9-8695-ce0e3fd8cc76', 100001, 0,
  'knowledge.base.edu-k12-chinese', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000011, '6d8649ed-71e7-4fcd-806f-73999b2e7ce3', 100001, 0,
  'knowledge.base.edu-k12-english', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000012, 'e5abac5f-0e1f-4e1b-82ce-be2b5c6c67f2', 100001, 0,
  'knowledge.base.edu-k12-physics', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000013, 'f8faa808-32a1-4a6c-8c20-9e97b9458a66', 100001, 0,
  'knowledge.base.edu-k12-chemistry', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000014, '6cc98dcd-d1dd-41ba-80c7-c284b5ff78d8', 100001, 0,
  'knowledge.base.edu-k12-biology', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000015, 'ea77bf9e-6e8b-49dc-8425-d71e41e2b7ae', 100001, 0,
  'knowledge.base.edu-k12-history', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000016, '9e3f0d49-ecb9-40d6-8dcb-94177647d64c', 100001, 0,
  'knowledge.base.edu-k12-geography', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000017, '105dc2a2-76d5-4923-8c33-3517ead11752', 100001, 0,
  'knowledge.base.edu-k12-politics', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000018, '3f4b5663-67c9-45f3-8357-a8919df8937f', 100001, 0,
  'knowledge.base.programmer-backend', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000019, '9919ffd1-c681-4d57-8d1e-ff285d3d19a6', 100001, 0,
  'knowledge.base.programmer-architect', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000020, '07013d99-93e4-47f7-84e1-8be10d245031', 100001, 0,
  'knowledge.base.programmer-devops', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000021, '9e484de0-4e10-4ff8-8149-d9eed06497f7', 100001, 0,
  'knowledge.base.programmer-qa', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000022, 'bc8e0c7d-7d13-43b5-839c-1d012ef769df', 100001, 0,
  'knowledge.base.programmer-data-ai', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000023, '8558d267-33d4-4786-8a4d-dc4878090da4', 100001, 0,
  'knowledge.base.edu-uni-math', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000024, 'dcd23963-d9c5-4499-81c2-45a0e95a6313', 100001, 0,
  'knowledge.base.edu-uni-physics', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000025, '08b20ef9-1df3-4552-8443-1827f8193fc6', 100001, 0,
  'knowledge.base.edu-uni-economics', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000026, 'db1db5ca-9d10-482c-84fd-0889cf1da7f7', 100001, 0,
  'knowledge.base.edu-uni-law', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000027, '00016096-502e-4b79-8660-4e6069b4c2f1', 100001, 0,
  'knowledge.base.edu-uni-medicine', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000028, '32c474d4-a2af-4889-8f42-d3d932630057', 100001, 0,
  'knowledge.base.edu-uni-engineering', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000029, '05459a1b-46d0-424f-83ab-9f1db934286d', 100001, 0,
  'knowledge.base.doctor-pediatrics', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000030, 'd9f2e057-fed8-489d-8f1a-46b8c8019610', 100001, 0,
  'knowledge.base.doctor-cardiology', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000031, 'fcba7a69-bdbf-4955-8c7d-a84207ad8cea', 100001, 0,
  'knowledge.base.doctor-mental-health', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000032, '6ab70774-b278-4c39-82f7-26538eab3676', 100001, 0,
  'knowledge.base.lawyer-labor', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000033, '0c842372-e724-4aba-8166-8661a313c09a', 100001, 0,
  'knowledge.base.lawyer-criminal', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000034, 'b71e40b8-4993-4140-8029-3e1c09378c88', 100001, 0,
  'knowledge.base.lawyer-ip', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000035, '726fb0d2-0545-4430-8cf2-082cfc40bbda', 100001, 0,
  'knowledge.base.lawyer-family', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000036, '9bdfe7c5-5df8-4ba1-84bb-c3f6f9c32087', 100001, 0,
  'knowledge.base.travel-guide', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000037, 'b5111452-cae3-4bb2-8298-cd1790def904', 100001, 0,
  'knowledge.base.travel-visa', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000038, '46d94c43-6032-4fb3-820c-8f1c16de1e8d', 100001, 0,
  'knowledge.base.ecommerce-product', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000039, '65b638e0-9467-4755-8b3a-0d0bb0be149f', 100001, 0,
  'knowledge.base.ecommerce-customer-service', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000040, '13a9c5ee-b7ee-4cf2-8c39-53a44708a452', 100001, 0,
  'knowledge.base.ecommerce-ads', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000041, '6b16a128-3915-4f33-8899-0390d3c27761', 100001, 0,
  'knowledge.base.realestate-rental', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000042, '18ba71ce-3a63-4093-8074-01c7023befb2', 100001, 0,
  'knowledge.base.realestate-valuation', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000043, '351a90d4-11da-4ed4-8743-488a949c4561', 100001, 0,
  'knowledge.base.restaurant-operations', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000044, 'f595a91a-9944-4ce7-8536-c72f59f24a2f', 100001, 0,
  'knowledge.base.restaurant-menu-development', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000045, '8a8d9cab-d2bb-41b6-80ec-26bda43ca4e9', 100001, 0,
  'knowledge.base.restaurant-food-safety', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000046, '46f48654-deef-41be-858f-ac2069e71865', 100001, 0,
  'knowledge.base.restaurant-marketing', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000047, '37fc86da-c361-4905-8341-93fda4b9f488', 100001, 0,
  'knowledge.base.enterprise-consulting', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000048, '62ec9871-08d1-47bf-81e9-acd231d44a9a', 100001, 0,
  'knowledge.base.enterprise-hr', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000049, 'f892dc97-1c74-43ce-882c-7098f8ff1672', 100001, 0,
  'knowledge.base.enterprise-tax', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000050, '9b20b88a-c9f5-4d9c-8eac-170cfd2f3fe7', 100001, 0,
  'knowledge.base.enterprise-b2b-sales', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000051, '2f2aeff7-cd2f-43f8-879c-d5f6f991732e', 100001, 0,
  'knowledge.base.government-service-guide', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000052, 'c6cfcc8f-07ee-4a68-8426-14804d63c00b', 100001, 0,
  'knowledge.base.government-policy-interpretation', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000053, 'd6b93e51-48d2-4e23-8760-92eaa6c9a829', 100001, 0,
  'knowledge.base.government-document', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000054, '596f3a4a-3693-4867-8181-bfdca738bde9', 100001, 0,
  'knowledge.base.government-digital', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000055, 'e5c50dc5-26aa-442c-8f10-edeecdec050e', 100001, 0,
  'knowledge.base.hotel-operations', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000056, '4f0c5971-c9ff-4b80-8b8c-878f9da4f480', 100001, 0,
  'knowledge.base.hotel-revenue', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000057, '681af1d8-0b5b-4212-8447-e742ffadb883', 100001, 0,
  'knowledge.base.hotel-guest-service', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

INSERT INTO kb_space (
  id, uuid, tenant_id, organization_id, name, description, status, knowledge_mode,
  okf_bundle_initialized, okf_log_sequence_counter, created_at, updated_at, version, created_by
) VALUES (
  533000000000000058, '2a56558f-9f21-49f7-8470-4f33ec67cf43', 100001, 0,
  'knowledge.base.hotel-marketing', NULL,
  1, 'okf_bundle', 0, 0, TIMESTAMPTZ '2026-11-03T00:00:00+08:00', TIMESTAMPTZ '2026-11-03T00:00:00+08:00', 0, 1
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  status = EXCLUDED.status,
  knowledge_mode = EXCLUDED.knowledge_mode,
  updated_at = EXCLUDED.updated_at
;

COMMIT;
