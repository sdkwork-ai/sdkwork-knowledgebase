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

describe('knowledgebase Phase 2 commercial readiness alignment', () => {
  it('documents the proposed dedicated tenant and organization isolation decision', () => {
    const adr = readRepoFile(
      'docs/architecture/decisions/ADR-20260731-dedicated-tenant-organization-runtime.md',
    );
    assert.match(adr, /Status: proposed/);
    assert.match(adr, /app\.current_tenant_id/);
    assert.match(adr, /app\.current_organization_id/);
    assert.match(adr, /FORCE ROW LEVEL SECURITY/);
    assert.match(adr, /Shared request-scoped tenant\/organization pooling is unsupported/);
  });

  it('indexes the dedicated runtime ADR through the standard architecture decisions directory', () => {
    const activeDocuments = [
      'docs/README.md',
      'docs/INDEX.yaml',
      'docs/runbooks/tenant-isolation.md',
      'docs/product/prd/PRD.md',
      'docs/product/prd/PRD-phase2-commercial-saas.md',
      'specs/tenant-isolation.md',
    ];

    for (const relativePath of activeDocuments) {
      const source = readRepoFile(relativePath);
      assert.match(
        source,
        /docs\/architecture\/decisions\/ADR-20260731-dedicated-tenant-organization-runtime\.md|architecture\/decisions\/ADR-20260731-dedicated-tenant-organization-runtime\.md|decisions\/ADR-20260731-dedicated-tenant-organization-runtime\.md|\.\.\/architecture\/decisions\/ADR-20260731-dedicated-tenant-organization-runtime\.md/,
        `${relativePath} must point to the standard ADR path`,
      );
      assert.doesNotMatch(
        source,
        /docs\/adr\/ADR-2026-06-24-phase2-postgres-rls-multi-tenant\.md|\.\.?\/adr\/ADR-2026-06-24-phase2-postgres-rls-multi-tenant\.md/,
        `${relativePath} must not point to retired docs/adr layout`,
      );
    }
  });

  it('ships Postgres RLS migration for tenant-scoped kb_* tables', () => {
    const baseline = readRepoFile('database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql');
    const crateMigration = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/postgres/V202606260001__knowledgebase_postgres_rls.sql',
    );
    // Post-launch organization isolation migrations are folded into the
    // consolidated pre-launch baseline (database/migrations/postgres/README.md).
    const organizationMigration = readRepoFile(
      'database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql',
    );
    assert.match(baseline, /ENABLE ROW LEVEL SECURITY/);
    assert.match(baseline, /tenant_isolation/);
    assert.match(baseline, /kb_audit_event/);
    assert.match(crateMigration, /app\.current_tenant_id/);
    assert.match(organizationMigration, /app\.current_organization_id/);
    assert.match(organizationMigration, /FORCE ROW LEVEL SECURITY/);
    assert.match(organizationMigration, /CREATE POLICY organization_isolation/);
  });

  it('keeps Postgres scope deployment-bound and rejects request-scoped checkout', () => {
    const bootstrap = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/db/bootstrap.rs',
    );
    const tenantSession = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/db/postgres_tenant_session.rs',
    );
    // The RLS session keys and the scope-injection helper live in the shared database host
    // (`postgres_scope.rs`); the repository bootstrap reuses them via `pub use`.
    const postgresScope = readRepoFile(
      'crates/sdkwork-knowledgebase-database-host/src/postgres_scope.rs',
    );
    const phase2Prd = readRepoFile('docs/product/prd/PRD-phase2-commercial-saas.md');
    assert.match(postgresScope, /POSTGRES_TENANT_SESSION_KEY/);
    assert.match(postgresScope, /postgres_url_with_deployment_scope/);
    assert.match(postgresScope, /require_postgres_rls_organization_id/);
    assert.match(bootstrap, /postgres_url_with_deployment_scope/);
    assert.match(bootstrap, /require_postgres_rls_organization_id/);
    assert.match(bootstrap, /create_pool_from_config/);
    assert.doesNotMatch(bootstrap, /PgPoolOptions|after_connect/);
    assert.match(tenantSession, /deployment-bound tenant id/);
    assert.match(phase2Prd, /One dedicated API\/worker deployment serves one tenant and one non-zero organization/);
    assert.match(phase2Prd, /does not authorize or\s+promise shared request-scoped multi-tenancy/);
    assert.doesNotMatch(phase2Prd, /Shared request checkout uses a transaction-scoped `SET LOCAL/);
    assert.match(tenantSession, /require_postgres_rls_tenant_id/);
    const processPoolSpec = readRepoFile('specs/process-database-pool.spec.json');
    assert.match(processPoolSpec, /one-tenant-per-process|one tenant per process/i);
    const webAudit = readRepoFile('crates/sdkwork-routes-knowledgebase-backend-api/src/web_audit_store.rs');
    assert.match(webAudit, /connect_and_bootstrap_webstore_database_from_env/);
    assert.match(webAudit, /shared_audit_emitter_pg/);
  });

  it('exports billable usage counters from observability crate', () => {
    const billingModule = readRepoFile(
      'crates/sdkwork-knowledgebase-observability/src/billing_metrics.rs',
    );
    const observabilityLib = readRepoFile('crates/sdkwork-knowledgebase-observability/src/lib.rs');
    assert.match(billingModule, /knowledge_retrievals_total/);
    assert.match(billingModule, /knowledge_context_packs_total/);
    assert.match(billingModule, /billing_event = "knowledge\.retrieval\.completed"/);
    assert.match(observabilityLib, /billing_metrics::render_billing_prometheus_metrics/);
  });

  it('records retrieval and context pack billing events in service layer', () => {
    const retrievalService = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-service/src/retrieval.rs',
    );
    const ingestService = readRepoFile(
      'crates/sdkwork-intelligence-knowledgebase-service/src/ingest/service.rs',
    );
    assert.match(retrievalService, /record_retrieval_completed/);
    assert.match(retrievalService, /record_context_pack_completed/);
    assert.match(ingestService, /record_ingest_job_succeeded/);
    assert.match(ingestService, /record_ingest_job_failed/);
  });

  it('documents audit retention and GDPR operator procedures', () => {
    const runbook = readRepoFile('docs/runbooks/audit-retention.md');
    assert.match(runbook, /kb_audit_event/);
    assert.match(runbook, /GDPR/);
    assert.match(runbook, /365 days/);
    assert.match(runbook, /compliance\.auditEvents\.export\.create/);
    assert.match(runbook, /compliance\.auditEvents\.anonymizeActor\.create/);
    assert.match(runbook, /retention automation pending/);
  });

  it('indexes current commercial release criteria in the product PRD map', () => {
    const prd = readRepoFile('docs/product/prd/PRD.md');
    const phase2 = readRepoFile('docs/product/prd/PRD-phase2-commercial-saas.md');
    assert.match(prd, /PRD-phase2-commercial-saas\.md/);
    assert.match(phase2, /These foundations are implementation evidence, not commercial release approval/);
    assert.match(phase2, /Subscription, entitlement, suspension, and payment authority belong to the SDKWork platform/);
    assert.match(phase2, /ADR-20260731-dedicated-tenant-organization-runtime/);
    assert.doesNotMatch(phase2, /ADR-20260624-phase2-postgres-rls-multi-tenant/);
  });
});
