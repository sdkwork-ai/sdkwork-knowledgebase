# MIG-2026-0720 Knowledge Engine Provider Binding

```yaml
id: MIG-2026-0720
owner: SDKWork Knowledgebase maintainers
status: active
requirement: REQ-2026-0720
decision: ADR-20260720
type: mixed
scope:
  producers:
    - sdkwork-knowledgebase-provider-runtime
    - sdkwork-intelligence-knowledgebase-service
    - sdkwork-intelligence-knowledgebase-repository-sqlx
    - sdkwork-routes-knowledgebase-backend-api
  consumers:
    - sdkwork-routes-knowledgebase-app-api
    - sdkwork-knowledgebase-worker
    - sdkwork-knowledgebase-pc
    - sdkwork-knowledgebase-app-sdk
    - sdkwork-knowledgebase-backend-sdk
compatibility_window:
  starts_at: 2026-07-20
  ends_at: before-v1.0.0
strategy: no-compatibility-approved
```

Specs: MIGRATION_SPEC.md, DATABASE_SPEC.md, API_SPEC.md, SDK_SPEC.md, SECURITY_SPEC.md,
PRIVACY_SPEC.md, TEST_SPEC.md, RELEASE_SPEC.md

## Authority And Compatibility

The application has not been released. The owner approved direct prelaunch cleanup on 2026-07-20.
There are no production consumers entitled to source-order Provider inference, raw environment
credential selection, or non-standard management contracts. The migration must not introduce
deprecated aliases, dual-write debt, a legacy resolver, or a compatibility feature flag.

Database changes remain forward-safe and reversible because local/staging data can exist before
release. Generated SDK output changes only through authored OpenAPI and the standard generator.

## Cutover

1. Introduce the shared Provider Runtime and migrate all adapters off direct HTTP construction.
2. Add provider binding, credential reference, migration operation, audit, indexes, and PostgreSQL
   RLS through canonical database authority and materialization.
3. Do not synthesize bindings from source rows. Report every external space without an active
   binding as bounded, actionable prelaunch data work; an administrator must create, test, and
   activate the binding explicitly. The implemented readiness command and paging procedure are
   documented in `docs/runbooks/RUNBOOK-provider-binding-readiness.md`.
4. Add SDKWork v3 backend management operations, regenerate composed SDKs, and migrate the PC
   management surface.
5. Resolve external mode only through the tenant/organization/space active binding. Source-based
   engine selection is removed in the same change; `kb_source.provider` remains non-authoritative
   source association metadata only. Adapter code cannot read `KnowledgeSourceStore` or parse
   `connector_metadata_json` into Provider remote-resource configuration.
6. Run Provider-to-Provider changes through the space-scoped migration operation. The target remote
   resource must already exist and have a successful Binding test. The Worker claims one phase with
   an expiring owner/token lease, revalidates versions/capabilities, atomically switches Bindings,
   defers observation completion until its deadline, and retains the predecessor for rollback.
7. Run PostgreSQL isolation, migration, API/SDK, provider certification, quality, load,
   outage, cutover, and rollback gates before release.

## Rollback

Rollback is supported before release and during every later Provider-to-Provider migration:

1. Stop new migration operations and allow in-flight leased work to reach a checkpoint or expire.
2. Atomically reactivate the retained predecessor binding when a cutover has occurred.
3. Roll back application artifacts and SDK consumers together when contract compatibility requires
   it; database tables remain additive until the forward-fix is verified.
4. Never delete predecessor binding records, credential references, remote data, or audit evidence
   as part of automatic rollback.
5. Restore the database from the approved backup only for migration corruption, then rerun the
   idempotent migration and reconciliation checks.

## Verification

```bash
cargo test -p sdkwork-knowledgebase-provider-runtime
cargo test -p sdkwork-intelligence-knowledgebase-service knowledge_engine
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx provider_binding
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test provider_binding_readiness_store
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test provider_binding_readiness_postgres_optional
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test provider_migration_store
cargo test -p sdkwork-knowledgebase-worker
cargo test -p sdkwork-routes-knowledgebase-app-api --test hosted_runtime_routes hosted_provider_migration_is_scoped_recoverable_and_reversible -- --exact
node tools/check_external_knowledge_engine_catalog.mjs
node tools/check_knowledge_engine_spi_standard.mjs
node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
pnpm api:materialize:check
pnpm sdk:generate:check
pnpm verify
```

Release completion additionally requires release-environment PostgreSQL, real Provider versions,
load, outage, backup/restore, cutover, rollback, security/privacy, licensing, SBOM, provenance, and
immutable artifact evidence.
