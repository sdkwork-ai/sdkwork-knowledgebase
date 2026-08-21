# PLAN-2026-0721 Live Wiki Cloud Publication

Status: in progress
Requirement: REQ-2026-0721
Decision: ADR-20260721-live-mounted-wiki-publication (accepted)
Machine contract: `specs/live-wiki-publication.spec.json`
Owner: SDKWork Knowledgebase maintainers
Updated: 2026-07-21

## 1. Outcome

Deliver one commercial cloud publication system shared by Drive directory websites and
Knowledgebase live Wikis without reviving the removed SiteRelease model.

The target behavior is:

- A Drive Space is externally servable only when its Space type is `website`; a configured folder
  is the document root and its current directory tree is the website.
- A Knowledgebase is externally servable only when a `wiki` publication is active; its
  `sources/raw` tree is the live content authority and every file has explicit processing,
  publication, visibility, route, and public-version state.
- Deploy owns Site, resource connections, mounts, domains, device Variants, TLS, ACME, descriptor
  revisions, rollout, usage, and commercial policy.
- Web Server consumes a compiled `WebsiteRuntimeDescriptor` and serves either a Drive directory or
  a Knowledgebase Wiki through typed resource providers.
- Provider lifecycle changes advance the narrowest page public version, provider generation,
  navigation generation, or search generation and invalidate live representations. They do not
  create a SiteRelease, Deploy Release, Deployment, or
  SiteRevision. `deploy_release` remains available only for Git, package, image, or other frozen
  artifact flows.

## 2. Non-Negotiable Clean-Break Rules

1. Do not restore the removed Site, SiteRelease, SiteHostBinding, copied-artifact public router, or
   PC deployment modal.
2. Do not add compatibility endpoints, SDK aliases, table aliases, dual reads, dual writes, or
   migration fallbacks for the unreleased model.
3. Generated SDK output is changed only through the canonical `sdkgen` generator.
4. Cross-service calls use generated SDK families or approved typed provider ports. Raw HTTP,
   manual auth headers, shared database reads, and object-key coupling are forbidden.
5. Drive nodes, versions, and renditions use stable Drive UUIDs/URIs. Knowledgebase never stores
   provider bucket/object keys.
6. Domain, certificate, ACME, Variant, mount, and routing state is not duplicated in Knowledgebase
   or Drive.
7. Public eligibility is explicit and fail closed. Drive visibility, RAG index state, or file
   presence alone never makes content public.
8. Breaking API, schema migration, SDK ownership, certificate security, and privacy changes pass
   the repository human-review gates before merge.
9. Standalone and cloud public traffic both use Web Server's descriptor/Mount pipeline. Do not add a
   Knowledgebase-owned anonymous `/wiki/...` compatibility route.

## 3. Ownership And Dependency Direction

| Repository | Owns | Must not own |
| --- | --- | --- |
| `sdkwork-drive` | Website Space type, document-root folder identity, tree/version events, typed node reads, atomic directory sync | Domains, TLS, device routing, Wiki publication policy |
| `sdkwork-knowledgebase` | Wiki publication authority, per-file projection, processors/renditions, routes, navigation, search, visibility, public-version policy | Domain/certificate state, copied website snapshots, object keys |
| `sdkwork-deployments` | Site/resource/Variant/mount/binding/domain/TLS/ACME/descriptor/rollout/usage control plane | Source bytes, Wiki parse/index state, request byte streaming |
| `sdkwork-webserver` | Host/SNI match, descriptor cache, Variant selection, path normalization, provider dispatch, HTTP/cache/range/security semantics | Authoring state, certificate ordering, source indexing, deployment business workflow |

Dependency direction is browser/admin client -> generated app/backend SDK -> owning service API ->
application service -> port -> adapter. Web Server resource reads use typed provider contracts
bound by the descriptor and never call authoring UI handlers.

## 4. Phase 0: Clean Baseline

Status: implemented; focused validation required before merge.

- [x] Remove old Rust domain/service/store/artifact/public-router implementation.
- [x] Remove old PostgreSQL/SQLite baseline tables and migration registration.
- [x] Remove old app API operations, permissions, route manifests, and authored schemas.
- [x] Regenerate the app SDK and prune stale Site/SiteRelease/HostBinding files.
- [x] Remove old PC deployment modal, local registry fields, menu entry, and SDK service.
- [x] Delete obsolete REQ/PLAN/MIG work records and retain only the required superseded ADR
  tombstone.
- [x] Remove stale Knowledgebase site-deployment permissions and public-object-gateway configuration.
- [ ] Pass all verification gates listed in section 13.

Exit condition: the repository compiles and all old runtime/API/SDK/database/UI identifiers are
absent. The only permitted references describe removed objects in the superseded ADR, migration,
architecture review, or a zero-presence test.

## 5. Phase 1: Publication And File Projection Contract

Implement the Knowledgebase authority before any cloud UI is exposed.

- Add `contract::wiki` types for publication state, per-file source/publication/visibility/index
  state, file kind, update policy, navigation policy, public resource requests, and safe results.
- Add `kb_site_publication`, `kb_source_file_projection`, and `kb_source_file_rendition` exactly as
  defined by `TECH-live-wiki-resource-provider.md` and the machine contract.
- Provision exactly one canonical DRAFT/PRIVATE WikiPublication with every new Knowledgebase and
  backfill existing eligible Knowledgebases through an idempotent bounded job before feature
  activation. No special Knowledgebase type or conversion flow is introduced.
- Separate retained business decisions from rebuildable observations in repository interfaces and
  backup/rebuild tests.
- Add PostgreSQL RLS, tenant/organization predicates, optimistic versions, unique public routes,
  bounded JSON checks, retry indexes, audit events, and outbox events.
- Keep PostgreSQL and SQLite contracts behaviorally equivalent; do not run real migrations during
  implementation verification.
- Author app API commands for Wiki settings, activate/pause, file review/publish/unpublish,
  visibility, scheduling, route validation, reprocess, problem lists, and preview authorization.
- Author backend API operations for reconciliation, retry/dead-letter handling, policy rollout,
  tenant suspension, and operational inspection.
- Generate app/backend SDKs and integrate clients through composed facades.

Exit condition: one-to-one provisioning/backfill, multi-Site resource reuse, state-machine,
version-conflict, route-collision, tenant-isolation, audit, RLS, pagination, and SDK contract tests
pass with no Web Server or Deploy dependency.

## 6. Phase 2: Drive Event Projection

- Accept a Drive-owned AsyncAPI authority for `drive.node.version.committed.v1`,
  `drive.node.path.changed.v1`, `drive.node.eligibility.changed.v1`, and
  `drive.node.deleted.v1`; declare the producer/consumer event inventories in component specs.
- Consume these events with tenant/Space/root-binding/checkpoint fencing, at-least-once delivery,
  replay, dead letter, compatibility, and lag policy.
- Accept events only for the bound `knowledge_base` Space and descendants of `sources/raw`.
- Upsert projections by stable Drive node/version UUID and retain explicit publication decisions
  across rebuilds and moves.
- Replace logical-path immutability with stable-node/new-immutable-version semantics. Verify edit,
  replacement, concurrent write, rename/move, delete/restore, and rollback without mutating a
  committed Drive version/blob.
- Add a checkpointed reconciler that can rebuild projections from a bounded Drive tree walk and
  repair missed/out-of-order events.
- Classify source type by extension, declared MIME, detected signature, checksum, size, and scan
  state. Mismatch, quarantine, or policy failure revokes public eligibility.
- Use durable jobs with leases, fencing, bounded concurrency, deadlines, retry/jitter, dead-letter
  state, and idempotent writes.
- Generate safe renditions through isolated processors and `sdkwork-drive-uploader-service`.
  Verify output MIME, signature, checksum, size, and processor/policy identity before READY.
- Atomically publish the versioned `knowledgebase.wiki.*.v1` provider events from the machine
  contract, carrying provider UUID/generation, affected routes, page public versions, checkpoint,
  visibility transition, and revocation priority. Do not publish source bytes or secrets.

Exit condition: large Markdown batches become routable without whole-tree loading; duplicate,
late, move, delete, quarantine, processor failure, and full-rebuild tests are deterministic.

## 7. Phase 3: Typed Wiki Resource Provider

Create a delivery contract separate from authoring APIs.

- Author `sdkwork-knowledgebase-internal-api`, generate
  `sdkwork-knowledgebase-internal-sdk`, and declare it as the split-topology integration authority.
  Standalone uses an equivalent typed Rust service port and the same compatibility fixtures.

Required operations:

- `resolve_publication(publication_uuid)` returns eligibility and policy metadata.
- `resolve_route(publication_uuid, normalized_path, locale)` returns page, rendition, asset,
  redirect, download, or not-found without leaking private existence.
- `open_body(resource_version, range)` streams bounded bytes through Drive-backed readers.
- `resolve_navigation`, `search_public`, `resolve_sitemap`, and `resolve_robots` use the same public
  version and visibility fence.
- `subscribe_invalidations(checkpoint)` supports at-least-once delivery and replay.

Provider rules:

- Validate tenant, connected Deploy Site/resource identity, publication state, route, public file
  state, scan state, and exact public Drive version on every resolution.
- Serve sanitized HTML/renditions for active content; never execute uploaded JavaScript on the
  Wiki origin.
- Return stable ETag/content version, media type, length, range capability, cache policy, content
  disposition, language, canonical route, and security metadata.
- Revoke private/delete/quarantine transitions immediately. `KEEP_LAST_PUBLIC_UNTIL_READY` applies
  only to safe content updates and never overrides a revocation.
- Bound page/body/search/navigation sizes and execution time; stream large bytes with backpressure.

Exit condition: provider conformance, traversal, XSS, MIME confusion, stale-public-version,
revocation, range, cancellation, cache, and load tests pass.

## 8. Phase 4: Deploy Resource Integration

Implement the shared control-plane model in `sdkwork-deployments`.

- Add resource kinds `DRIVE_DIRECTORY` and `KNOWLEDGEBASE_WIKI` with typed owner, resource UUID,
  provider contract version, health, capability, and bounded diagnostic metadata. Provider endpoint
  and credentials remain runtime configuration and never enter a descriptor or business row.
- Site creation requires an eligible Drive Website Space or active/ready Wiki publication.
  Normal Drive Spaces and non-Wiki Knowledgebases are rejected.
- Mounts bind a URL prefix to one resource and optional Variant. The same resource may be mounted
  by multiple Sites; ownership/tenant checks are mandatory.
- Variants support PC, mobile, tablet, bot, TV, embedded, locale, and explicit client hints with
  deterministic priority/fallback. User-Agent-only matching is advisory and never an auth signal.
- Bind one or more verified domains to a Site, with canonical/redirect policy and host conflict
  uniqueness. Domain routing may select different Variants but not different tenants.
- Manage certificate orders, challenges, keys, chains, deployment state, renewal windows,
  revocation, failure backoff, and audit. Private keys remain in the approved secret/KMS boundary.
- Compile immutable descriptor revisions only for Deploy-owned configuration changes. Knowledgebase
  upload, processing, publication, visibility, route-content mapping, theme, renderer, navigation,
  search, quarantine, delete, and restore advance the narrowest provider/page/navigation/search
  version or generation and create no Deploy Release, Deployment, or SiteRevision.
- Publish signed/versioned descriptor snapshots and TLS snapshots; retain last-known-good rollback
  independently for each.

Exit condition: domain verification, SNI/certificate selection, ACME HTTP-01/DNS-01 renewal,
Variant precedence, mount conflicts, descriptor rollout, rollback, tenant isolation, and audit
tests pass.

## 9. Phase 5: Web Server Data Plane

- Load and validate `WebsiteRuntimeDescriptor` and TLS snapshots with monotonic revision fencing,
  signature/checksum verification, bounded caches, watch/poll recovery, and last-known-good state.
- Resolve request in this order: SNI/TLS -> normalized Host -> Site -> Variant -> longest-prefix
  mount -> provider -> normalized resource path -> representation -> HTTP response.
- Implement STATIC, SPA, and WIKI handlers behind one descriptor model. WIKI delegates to the
  typed provider; STATIC/SPA delegates to Drive directory or immutable artifact providers.
- Route both standalone and cloud traffic through that descriptor model. Standalone may use an
  in-process typed provider adapter but has no Knowledgebase-owned fixed public route.
- Enforce traversal/symlink policy, hidden/reserved files, directory index/listing policy, SPA
  fallback, MIME, compression, conditional requests, range, HEAD, redirects, cache controls,
  CSP/HSTS/nosniff/referrer policy, and uniform not-found behavior.
- Separate descriptor cache, route metadata cache, body cache, and negative cache. Security
  revocation invalidations bypass normal debounce and purge immediately.
- Emit bounded metrics/traces/logs for host, Site, Variant, mount, provider, status, cache outcome,
  bytes, latency, and safe error code without URL secrets or customer content.

Exit condition: desktop/mobile/device routing, custom domains, certificate rotation, live Wiki
updates, immediate unpublish, multi-range/conditional behavior, cache stampede, provider outage,
descriptor rollback, and horizontal consistency tests pass.

## 10. Phase 6: User And Admin Interfaces

Do not expose these screens before their generated SDK operations and permissions exist.

Knowledgebase user views:

- Wiki overview: status, public URL, connected Deploy Site, document root, freshness, problems, and
  pause/activate action.
- Publication settings: title, home page, locale, navigation, theme, search, robots, sitemap,
  publication mode, default visibility, and update policy.
- Source file table/tree: source state, publication state, visibility, canonical route, public
  version, index/rendition state, schedule, error, preview, and bulk review/publish/unpublish.
- Problem center: collisions, broken links, unsafe MIME/content, quarantine, processor/index
  errors, retry/reprocess, and redacted diagnostics.

Deploy user views:

- Site resource/mount editor with Drive Directory or Knowledgebase Wiki eligibility validation.
- Domain list, DNS verification instructions, canonical/redirect selection, certificate status,
  renewal timeline, and ACME challenge evidence.
- Variant rule editor with ordered predicates, explicit default/fallback, route tester, and
  desktop/mobile/tablet/bot simulations.
- Deployment health, descriptor revision, last-known-good state, request analytics, bandwidth,
  cache ratio, errors, and billing usage.

Admin views:

- Cross-tenant Site/domain/certificate inventory with scoped support access and audit.
- Expiring/failed certificate and ACME queue, challenge retry, issuer rate-limit, and revocation
  operations.
- Descriptor rollout/failure/rollback, provider health, orphan resource/mount reconciliation,
  invalidation lag, cache purge, abuse suspension, quota, and usage adjustment workflows.

Exit condition: permission, tenant boundary, destructive confirmation, loading/empty/error,
pagination/filtering, keyboard/accessibility, responsive layout, and end-to-end browser tests pass.

## 11. Phase 7: Commercial Launch Gates

- Security: threat model, SSRF/path/XSS/MIME/archive-bomb controls, secret/KMS review, least
  privilege, certificate key ceremony, abuse/rate limits, dependency and supply-chain scans.
- Reliability: SLOs for route resolution, first byte, invalidation, activation, and certificate
  renewal; capacity model; regional failure; backup/restore; reconciliation and disaster drills.
- Performance: directory/Wiki scale profiles, large trees, large Markdown batches, media range,
  cache hit ratio, stampede, p95/p99 latency, memory, CPU, and provider concurrency budgets.
- Operations: dashboards, alerts, on-call runbooks, renewal incident procedures, public-content
  revocation drills, descriptor rollback, tenant suspension, and safe support tooling.
- Commercial: plan entitlements, custom-domain/Variant/bandwidth/storage/build quotas, usage events,
  metering reconciliation, invoice evidence, trials, suspension/grace, and cost attribution.
- Compliance: retention/deletion, audit export, data residency, content takedown, privacy review,
  license attribution, and customer-domain ownership evidence.

Exit condition: commercial readiness review has no open P0/P1 findings and every external
dependency has production evidence rather than mock-only tests.

## 12. Recommended Delivery Slices

1. Slice A: Phase 0 merge as an independent debt-removal change.
2. Slice B: Phase 1 database/domain/app API with no cloud exposure.
3. Slice C: Phase 2 projection plus author-only preview.
4. Slice D: Phase 3 provider conformance with a local test harness.
5. Slice E: Phase 4 Deploy resource/domain/Variant/TLS control plane.
6. Slice F: Phase 5 Web Server canary for system domains, then custom domains.
7. Slice G: Phase 6 user/admin UI and Phase 7 commercial gates.

Each slice has its own reviewed API/schema boundary, generated SDK diff, migration plan, rollback,
observability, load evidence, and security tests. Do not combine all repositories into one opaque
release.

## 13. Verification Gates

Phase 0 minimum:

```powershell
cargo fmt --all -- --check
cargo check -p sdkwork-knowledgebase-contract
cargo check -p sdkwork-intelligence-knowledgebase-service
cargo check -p sdkwork-intelligence-knowledgebase-repository-sqlx
cargo check -p sdkwork-routes-knowledgebase-app-api
cargo check -p sdkwork-api-knowledgebase-assembly
pnpm api:materialize:check
pnpm sdk:generate:check
pnpm sdk:check
pnpm db:validate
pnpm check:source-hygiene
pnpm check:architecture-alignment
node ..\sdkwork-specs\tools\check-repository-docs-standard.mjs --root .
```

Every later phase adds focused contract/service/repository/provider/UI tests and the owning
repository's narrowest prescribed validation. Workspace-wide and production migration commands
are not default verification.

## 14. Approval Gates

Before Phase 1 starts:

- Accept `ADR-20260721-live-mounted-wiki-publication`.
- Human-review the database contract, public API, SDK generation, privacy/security model, and
  cross-repository ownership boundaries.
- Confirm the Deploy and Web Server descriptor/provider contracts are versioned authorities.
- Confirm no customer or production data exists in the removed prelaunch tables. If that
  assumption is false, stop and replace clean deletion with a separately reviewed data migration.
