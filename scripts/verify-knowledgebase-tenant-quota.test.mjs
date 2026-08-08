import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');

function readRepoFile(relativePath) {
  return readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function readOpenApi() {
  return JSON.parse(
    readRepoFile('sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json'),
  );
}

describe('knowledgebase tenant quota and GDPR compliance alignment', () => {
  it('defines KnowledgeTenantQuotaStatus and quota on KnowledgeTenantStatus in backend OpenAPI', () => {
    const spec = readOpenApi();
    assert.ok(spec.components.schemas.KnowledgeTenantQuotaStatus);
    assert.ok(spec.components.schemas.KnowledgeTenantStatus.properties.quota);
  });

  it('exposes GDPR compliance audit export and anonymize operations in backend OpenAPI', () => {
    const spec = readOpenApi();
    for (const operationId of [
      'compliance.auditEvents.export.create',
      'compliance.auditEvents.anonymizeActor.create',
    ]) {
      assert.ok(
        Object.values(spec.paths).some((methods) =>
          Object.values(methods).some((operation) => operation.operationId === operationId),
        ),
        `missing operation ${operationId}`,
      );
    }
    for (const schemaName of [
      'ExportKnowledgeAuditEventsRequest',
      'KnowledgeAuditEventExport',
      'AnonymizeKnowledgeAuditSubjectRequest',
      'AnonymizeKnowledgeAuditSubjectResult',
    ]) {
      assert.ok(spec.components.schemas[schemaName], `missing schema ${schemaName}`);
    }

    const exportOperation =
      spec.paths['/backend/v3/api/knowledge/compliance/audit_events/export'].post;
    assert.ok(exportOperation.responses['413']);
    assert.match(exportOperation.description, /5,000/);
    assert.equal(
      spec.components.schemas.ExportKnowledgeAuditEventsRequest.properties.actorId.minLength,
      1,
    );
    assert.equal(
      spec.components.schemas.ExportKnowledgeAuditEventsRequest.properties.actorId.maxLength,
      128,
    );
    assert.equal(
      spec.components.schemas.AnonymizeKnowledgeAuditSubjectRequest.properties.actorId.maxLength,
      128,
    );
    assert.equal(
      spec.components.schemas.KnowledgeAuditEventExport.properties.items.maxItems,
      5_000,
    );
    assert.equal(
      spec.components.schemas.KnowledgeAuditEventItem.properties.createdAt.format,
      'date-time',
    );
    assert.equal(
      spec.components.schemas.KnowledgeAuditEventItem.properties.actorId.maxLength,
      128,
    );
  });

  it('ships backend SDK types and compliance client for quota and GDPR workflows', () => {
    const sdkApi = readRepoFile(
      'sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi/src/api/knowledge.ts',
    );
    const tenantStatus = readRepoFile(
      'sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi/src/types/knowledge-tenant-status.ts',
    );
    assert.match(sdkApi, /KnowledgeComplianceAuditEventsApi/);
    assert.match(sdkApi, /compliance\/audit_events\/export/);
    assert.match(sdkApi, /compliance\/audit_events\/anonymize_actor/);
    assert.match(tenantStatus, /KnowledgeTenantQuotaStatus/);
  });

  it('enforces tenant business quotas at app, service, and repository boundaries', () => {
    const enforcement = readRepoFile(
      'crates/sdkwork-routes-knowledgebase-app-api/src/tenant_quota_enforcement.rs',
    );
    const hosted = readRepoFile('crates/sdkwork-routes-knowledgebase-app-api/src/hosted.rs');
    const runtime = readRepoFile('crates/sdkwork-routes-knowledgebase-app-api/src/runtime.rs');
    const objectRefStore = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/drive_object_ref_store.rs',
    );
    const appError = readRepoFile('crates/sdkwork-routes-knowledgebase-app-api/src/error.rs');
    const observability = readRepoFile('crates/sdkwork-knowledgebase-observability/src/tenant_quota.rs');
    const tenantContract = readRepoFile('crates/sdkwork-knowledgebase-contract/src/tenant.rs');
    assert.match(appError, /knowledge_tenant_quota_exceeded/);
    assert.match(enforcement, /ensure_document_capacity/);
    const ingestionStore = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/postgres_import_stores.rs',
    );
    assert.match(ingestionStore, /create_or_get_job_with_quota|ensure_ingest_quota_on/);
    assert.match(hosted, /ensure_tenant_can_create_document/);
    assert.match(runtime, /with_quota_limits\(quota_limits\)/);
    assert.match(objectRefStore, /ensure_storage_quota_on/);
    assert.match(objectRefStore, /ensure_storage_capacity\(total, additional_bytes, &limits\)/);
    assert.match(objectRefStore, /tenant_id = \$1 AND organization_id = \$2/);
    // Quota enforcement is transactional on the PostgreSQL authoritative path: the storage
    // capacity check and the object-ref insert share one database transaction, so a concurrent
    // over-quota insert cannot partially persist (replaces the removed SQLite fixture test).
    assert.match(objectRefStore, /begin_tenant_quota_transaction/);
    assert.match(objectRefStore, /ensure_storage_quota_on\(&mut transaction/);
    assert.match(objectRefStore, /transaction\.commit/);
    assert.match(observability, /SDKWORK_KNOWLEDGEBASE_TENANT_MAX_DOCUMENTS/);
    assert.match(observability, /SDKWORK_KNOWLEDGEBASE_TENANT_MAX_CONCURRENT_INGEST_JOBS/);
    assert.match(observability, /SDKWORK_KNOWLEDGEBASE_TENANT_MAX_RETRIEVALS_PER_MINUTE/);
    assert.match(observability, /SDKWORK_KNOWLEDGEBASE_TENANT_MAX_STORAGE_BYTES/);
    assert.match(tenantContract, /max_storage_bytes/);
    assert.match(tenantContract, /storage_bytes_used/);
  });

  it('documents tenant quota env vars and admin quota visibility', () => {
    const tenantIsolation = readRepoFile('specs/tenant-isolation.md');
    const adminConsole = readRepoFile(
      'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-shell/src/KnowledgebaseAdminConsole.tsx',
    );
    const tenantStatusSdk = readRepoFile(
      'sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi/src/types/knowledge-tenant-quota-status.ts',
    );
    assert.match(tenantIsolation, /SDKWORK_KNOWLEDGEBASE_TENANT_MAX_DOCUMENTS/);
    assert.match(tenantIsolation, /SDKWORK_KNOWLEDGEBASE_TENANT_MAX_STORAGE_BYTES/);
    assert.match(adminConsole, /adminConsoleQuotaTitle/);
    assert.match(adminConsole, /status\.quota/);
    assert.match(adminConsole, /formatBytes/);
    assert.match(tenantStatusSdk, /maxStorageBytes/);
    assert.match(tenantStatusSdk, /storageBytesUsed/);
  });

  it('documents GDPR compliance API in audit retention runbook', () => {
    const runbook = readRepoFile('docs/runbooks/audit-retention.md');
    assert.match(runbook, /compliance\.auditEvents\.export\.create/);
    assert.match(runbook, /compliance\.auditEvents\.anonymizeActor\.create/);
    assert.match(runbook, /audit_export_limit_exceeded/);
    assert.match(runbook, /POST \/backend\/v3\/api\/knowledge\/compliance/);
  });
});
