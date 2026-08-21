# MIG-2026-0721 Release To Live Wiki Publication

Status: implementation active; production evidence blocked
Requirement: REQ-2026-0721
Decision: ADR-20260721-live-mounted-wiki-publication
Owner: SDKWork Knowledgebase maintainers
Date: 2026-07-21
Machine contract: `specs/live-wiki-publication.spec.json`
Specs: MIGRATION_SPEC.md, DATABASE_SPEC.md, API_SPEC.md, SDK_SPEC.md, DRIVE_SPEC.md,
DEPLOYMENT_SPEC.md, RELEASE_SPEC.md, SECURITY_SPEC.md, TEST_SPEC.md, MEDIA_RESOURCE_SPEC.md,
SUPPLY_CHAIN_SECURITY_SPEC.md

Cross-repository runtime contracts:

- `sdkwork-deployments/docs/architecture/tech/TECH-cloud-site-publishing-control-plane.md`
- `sdkwork-webserver/docs/architecture/tech/TECH-cloud-site-delivery-data-plane.md`

## 1. Scope

Preserve the clean removal of the unlaunched immutable `kb_site` / `kb_site_release` /
`kb_site_host_binding` publication model, then add live `kb_site_publication`,
`kb_source_file_projection`, and rebuildable `kb_source_file_rendition` state after architecture and
schema approval. Site/domain/TLS/runtime configuration authority belongs to `sdkwork-deployments`;
Knowledgebase retains Wiki state, rendering, navigation, search, and public page/asset eligibility.

This migration governs the approved prelaunch realignment. It authorizes additive implementation work
under the stated review and verification gates; it does not authorize deleting user changes or
declaring incomplete capability production-ready.

## 2. Superseded Surfaces

- The obsolete REQ, implementation PLAN, and migration record were removed. The superseded ADR is
  retained only as the governance-required tombstone and is not an implementation authority.
- `kb_site_release` immutable artifact generation and current-release pointer were removed.
- Knowledgebase Site host-binding/domain/certificate ownership was removed in favor of Deploy stable
  Site/resource references.
- Release publish/rollback APIs, route manifests, generated SDK operations/types, PC release history,
  release builder, release artifact store, and release cleanup paths were removed.
- Knowledgebase owns no anonymous public router. Standalone and cloud delivery both enter the Web
  Server descriptor/Mount pipeline and use the same typed WIKI provider contract.

Generated SDK transports are changed only by editing owner OpenAPI/route sources and regenerating.

## 3. Target Additive Schema

Add `kb_site_publication`, `kb_source_file_projection`, and `kb_source_file_rendition` in PostgreSQL
and SQLite with the target contract in `TECH-live-wiki-resource-provider.md`. Add indexes,
constraints, RLS/tenant filtering, worker leases/checkpoints/outbox integration, schema manifest
coverage, and repository adapters. Rendition rows reference stable Drive-generated artifacts and
must not store object keys, presigned URLs, or process-local paths.

Use `provider_generation` for provider-wide eligibility/policy fencing and `page_public_version` for
per-route public content/revocation. Drive source checkpoint and navigation/search generations are
independent. Do not materialize one catch-all generation that flushes the whole Wiki for private
processing or a normal page-body update.

The additive migration must start from a baseline in which old tables, compatibility views, aliases,
and writers are absent. Static migration tests reject their reintroduction. Do not add a dual-writer
flag, compatibility view, fallback repository, or old-to-new runtime adapter.

## 4. Data Policy

The product owner confirmed the application is prelaunch, so no customer or production publication
data exists in the removed tables. If contrary evidence is discovered, stop this migration: do not
restore compatibility code or infer a mapping.
Create a separately reviewed data-recovery/migration record from retained backup evidence.

- Every eligible `kb_space` receives exactly one canonical DRAFT/PRIVATE WikiPublication from the
  current Space plus bound Drive `sources/raw` tree through an idempotent bounded backfill; no old
  release row is an authority.
- Domain, host, TLS, or runtime configuration is created through Deploy and never migrated into
  Knowledgebase tables.
- Do not copy any retained rendered release artifact into the source projection. If external backup
  evidence reveals such artifacts, handle cleanup under a separate retention/legal-hold review.
- Do not treat new PDF/page-image/text/thumbnail/poster/sheet/slide renditions as migrated
  SiteRelease artifacts. Rebuild them from pinned source versions through the approved processor
  registry and retain only stable Drive identities plus processor/checksum facts.
- Explicit source-file publication decisions must be established from approved rules/user review;
  do not infer public state from old artifact membership without owner approval.

## 5. Stages

### Stage 0 - Clean Baseline (Completed)

Release-oriented code, routes, OpenAPI, SDK types, migrations, tables, jobs, UI, permissions, and
configuration were removed without a compatibility layer. Keep static zero-presence checks and
obtain the required human confirmation that no customer/production publication data existed.

### Stage 1 - Add Live Model

Add schema/contracts/services and feature-disabled projection/rendition processing. Provision the
canonical publication for new Knowledgebases, backfill existing eligible Knowledgebases, bootstrap
`sources/raw` binding, and reconcile Drive events without anonymous delivery. Build format
classifier, processor sandbox, renderer/search/provider, and rendition cleanup/rebuild tests.
Before projection is enabled, remove the current logical-path overwrite rejection for
`sources/raw` and prove stable-node/new-immutable-version edit semantics without weakening
committed Drive version/blob immutability.

### Stage 2 - Non-Public Projection Qualification

Project representative Markdown, HTML, PDF, Office, presentation, spreadsheet, code, media,
archive, and unsupported-binary sources. Verify routes/renditions/render/assets/search/state; run
converter sandbox, sanitizer, isolation, load, cleanup, rebuild, and processor rollback tests; and
attach Deploy resources in a non-public environment. No old release serving path is restored for
comparison.

The shadow descriptor must keep Deploy `resourceUuid` distinct from Knowledgebase
`providerResourceUuid`, carry only stable provider Space/root/contract-version metadata, and reject
provider endpoints, tokens, storage topology, credentials, and presigned URLs.
The stage also requires Drive-owned input AsyncAPI, Knowledgebase-owned output AsyncAPI,
`sdkwork-knowledgebase-internal-api`, generated `sdkwork-knowledgebase-internal-sdk`, and standalone
typed-port parity. No raw HTTP fallback is permitted.

### Stage 3 - SDK And UI Cutover

Add owner OpenAPI/route manifests, regenerate SDKs, and update services/UI to Wiki publication and
file-state workflows. Static scans continue to reject removed release/host-binding types and methods,
compatibility aliases, and raw HTTP fallbacks.

### Stage 4 - Runtime Cutover

Activate Deploy `KNOWLEDGEBASE_WIKI` resources and the Web Server WIKI handler for pilot Sites in
standalone and cloud profiles. Verify content freshness, visibility revocation, domains, TLS, cache,
outage, traces, and the continued absence of any Knowledgebase anonymous public router.

Before public activation, run the shared `validateResource`, `resolveWikiRoute`, `openContent`,
`searchWiki`, and `subscribeResourceEvents` compatibility suite. Verify the complete cache identity,
move invalidation, durable event checkpoints, gap reconciliation, non-disclosing errors, and
last-known-good isolation. Prove that ordinary provider lifecycle changes create no Deploy Release,
Deployment, or `SiteRevision`.

### Stage 5 - Contract And Operations Closure

Complete database baseline, schema registry, docs, tests, changelog, runbooks, release evidence,
backup/rebuild drills, and zero-presence scans. Any later discovery of old external data or artifacts
is handled by a separate approved recovery/cleanup record, not by reintroducing old runtime code.

## 6. Rollback

Before Stage 4, disable the new feature while keeping the removed model absent; do not delete new
source decisions. During pilot, roll the Deploy Site to the previous runtime configuration or remove
the WIKI Mount and stop provider traffic while fixing forward. Do not restore an old Knowledgebase
public router, table, API, SDK, or UI as a rollback path. Once user publication decisions exist in
the new model, never roll the database back to a schema that cannot represent them; restore a
consistent backup only as a full incident-recovery action.

Rolling back a Deploy `SiteRevision` changes only Site composition/delivery configuration. It does
not roll back a Knowledgebase public content version. Content rollback remains a version-fenced
Knowledgebase/Drive operation and emits provider invalidation independently.

Content rollback uses Drive source version plus Knowledgebase publication/public-version state.
Domain/TLS/config rollback uses Deploy. Neither requires rebuilding a SiteRelease.

## 7. Verification

- dual-engine migration/contract/RLS/index/rollback-plan and backup/restore evidence;
- deterministic bootstrap/reconciliation and idempotent rerun;
- zero-presence enforcement for old tables, routes, operations, SDK types, permissions, config, and UI;
- owner OpenAPI regeneration and consumer import checks with no handwritten generated output;
- projection/event/rebuild, format classification, processor sandbox, rendition lifecycle,
  render/search/assets/routes, state/visibility, tenant/cache/index security;
- stable-node/new-immutable-version edits and exact Drive/Wiki event compatibility, replay,
  checkpoint, gap, and route-scoped generation behavior;
- Deploy resource/Web Server handler/domain/TLS/freshness/outage/rollback E2E;
- Deploy descriptor resource/provider identity split, secret-topology rejection, contract-version
  fencing, and SiteRevision trigger tests;
- Web Server unified provider-port/error mapping, complete cache tuple, event checkpoint/gap,
  last-known-good, and standalone/cloud parity tests;
- zero-presence scans continue to reject old APIs/types/routes/services/tables/config/permissions.

## 8. Human Review Gates

Approval is required for confirmation that no customer/production data existed in the removed
model, exact new tables/columns/enums/RLS, API/SDK breaking surface, Deploy binding creation,
production-like cutover, processor/converter selection and supply-chain policy, trusted-active-site
policy, and any separately discovered destructive data/artifact cleanup.
