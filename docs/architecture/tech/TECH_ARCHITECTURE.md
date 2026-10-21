# SDKWork Knowledgebase Technical Architecture

Status: active  
Owner: SDKWork maintainers  
Updated: 2026-07-31<br>
Specs: ARCHITECTURE_DECISION_SPEC.md, DOCUMENTATION_SPEC.md

## Document Map


- [TECH-alignment-baseline-2026-06-29.md](TECH-alignment-baseline-2026-06-29.md)
- [TECH-2026-06-11-sdkwork-structure-standardization-design.md](TECH-2026-06-11-sdkwork-structure-standardization-design.md)
- [TECH-2026-06-11-sdkwork-structure-standardization-implementation.md](TECH-2026-06-11-sdkwork-structure-standardization-implementation.md)
- [TECH-2026-06-19-okf-knowledge-bundle-design.md](TECH-2026-06-19-okf-knowledge-bundle-design.md)
- [TECH-okf-knowledge-bundle.md](TECH-okf-knowledge-bundle.md)
- [TECH-live-wiki-resource-provider.md](TECH-live-wiki-resource-provider.md) (active)
- [TECH-2026-06-01-knowledgebase-backend-design.md](TECH-2026-06-01-knowledgebase-backend-design.md)
- [TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md](TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md)
- [TECH-2026-06-09-knowledgebase-agent-rag-design.md](TECH-2026-06-09-knowledgebase-agent-rag-design.md)
- [TECH-2026-06-09-knowledgebase-agent-rag-implementation.md](TECH-2026-06-09-knowledgebase-agent-rag-implementation.md)
- [TECH-2026-06-12-knowledgebase-open-api-design.md](TECH-2026-06-12-knowledgebase-open-api-design.md)
- [TECH-2026-06-12-knowledgebase-open-api-implementation.md](TECH-2026-06-12-knowledgebase-open-api-implementation.md)
- [TECH-topology-standard.md](TECH-topology-standard.md)
- [PRD-mvp-launch.md](../../product/prd/PRD-mvp-launch.md)
- [ADR-20260731-dedicated-tenant-organization-runtime.md](../decisions/ADR-20260731-dedicated-tenant-organization-runtime.md) (proposed; human approval required)
- [ADR-20260713-group-knowledgebase-binding-and-launch.md](../decisions/ADR-20260713-group-knowledgebase-binding-and-launch.md)
- [ADR-20260720-knowledge-engine-provider-binding-spi-v2.md](../decisions/ADR-20260720-knowledge-engine-provider-binding-spi-v2.md) (accepted)

## 1. Architecture Overview

SDKWork Knowledgebase is a Rust backend with a horizontally scalable application public-ingress process, a separately scalable worker process, and a PC React client (browser + optional Tauri desktop). The public ingress mounts app-api, backend-api, and open-api route surfaces on one application-plane listener. Each production deployment binds **one tenant and one non-zero organization** across every ingress/worker replica with fail-closed guards.

| Surface | Prefix | SDK family | Auth |
|---------|--------|------------|------|
| App API | `/app/v3/api` | `sdkwork-knowledgebase-app-sdk` | dual-token |
| Backend API | `/backend/v3/api` | `sdkwork-knowledgebase-backend-sdk` | dual-token + `knowledge.platform.manage` |
| Open API | `/knowledge/v3/api` | `sdkwork-knowledgebase-sdk` | API key |
| Worker | — | — | internal |

App, backend, and open API authority documents are maintained under their owning SDK family and
materialized into `apis/` by `pnpm api:materialize`. The internal Wiki provider API is authored in
`apis/internal-api/knowledgebase/`, mirrored into its SDK family by the canonical generation
workflow, and consumed through generated TypeScript and Rust transports. The direction is explicit
per surface; generated transports are never API authority.
For the app surface, the authored source is
`sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json`; materialization
derives `apis/app-api/knowledgebase-app-api.openapi.json`, and the generator derives only
`sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi`.
Derived copies are never edited as competing contract authorities.

## 2. Technology Choices

- **Backend**: Rust, Axum, SQLx, `sdkwork-web-framework`, PostgreSQL authoritative server storage.
  SQLite is not a server profile: no server crate compiles a SQLite driver,
  `database/database.manifest.json` declares `postgres` only, and the SQLite schema fixture under
  `tests/fixtures/database/sqlite/` is a reference artifact whose DDL is executed in memory by the
  schema-parity test — never a second runtime backend.
- **Storage**: `sdkwork-drive` via `sdkwork-knowledgebase-drive` only. Standalone may use the Drive
  local provider; cloud requires an active non-local Drive provider resolved from Drive metadata
  and credential references, and startup verifies the configured bucket.
- **OKF browser views**: PC file lists use `spaces.browser.list?view=files`, which resolves OKF spaces to `sources/raw` original files. OKF bundle inspection uses `view=okf_bundle`; generated outputs use `view=outputs`.
- **Memory**: `sdkwork-memory` via `sdkwork-knowledgebase-memory` port only
- **Frontend**: React 19, Vite, TipTap, IAM app SDK, generated knowledgebase app SDK, `@sdkwork/drive-app-sdk` for persistent uploads
- **Client pagination**: PC Cloud Drive browse/import uses generated Knowledgebase SDK and Drive SDK cursor page methods; interactive my-drive, starred, recent, and shared tabs load one page at a time and never prefetch multi-page aggregates
- **Client composition**: native authority per `APP_COMPOSITION_SPEC.md` — root `pnpm-workspace.yaml`, pc-core `sdkDependencies`, and capability packages import SDK types only via `sdkwork-knowledgebase-pc-core/sdk`
- **Observability**: Prometheus `/metrics` (in-cluster only), structured audit logs, optional OTLP
- **External knowledge Providers**: `kb_provider_binding` is the sole tenant/organization/space
  selection authority. `sdkwork-knowledgebase-provider-runtime` owns outbound target policy,
  deadlines, bounded retries, `Retry-After`, circuit breaking, bulkheads, response limits, trace
  propagation, redaction, and bounded-cardinality Provider metrics. Provider origin hosts are
  DNS-resolved, validated as public (private/loopback/link-local/metadata ranges fail closed), and
  pinned on the HTTP client, so a rebinding DNS record cannot redirect provider traffic into an
  internal network. Source rows never select a
  Provider. `KnowledgeEngineSpaceResolver` returns a `KnowledgeEngineExecutionHandle`; search,
  read, and list validate immutable request-derived identity, scope, binding, trace, and deadline
  before engine execution. Adapters revalidate request tenant/space before HTTP, and external sync
  requires the same explicit context. Credential-free infrastructure probes may use the bounded
  system-health context; external Binding health always carries an authenticated management
  context and resolves the Binding credential after authorization.

## 3. System Boundaries

- Business logic: `sdkwork-knowledgebase-service`
- Persistence: `sdkwork-knowledgebase-repository-sqlx` + `database/` lifecycle
- HTTP boundaries: `sdkwork-routes-knowledgebase-{app,backend,open}-api`
- Background work: `sdkwork-knowledgebase-worker` (outbox, ingest, group archive, and Provider
  migration maintenance)
- Ingestion workers claim Drive jobs with owner/token leases in one
  `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING` statement, renew leases
  during processing, reclaim expired work after crashes, and fence stale workers from success or
  failure commits. Jobs whose lease expired at/over
  `SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_MAX_ATTEMPTS` claims are failed permanently instead of
  being reclaimed forever. Chunk replacement, job completion, and outbox append remain one database
  transaction. After chunk replacement the worker persists the ingestion lifecycle
  (`kb_document_version.parse_state = succeeded`, `kb_document.index_state = running`), and the same
  maintenance phase drains a durable post-ingest embedding backlog
  (`kb_document.index_state IN (pending, running)` with chunks and no active embeddings). API
  ingest, Drive import, and Git import only persist content; they never serialize model-provider
  calls, and an ingestion that completes leaves the document eligible for worker embedding rather
  than reporting a synchronous success.
- The worker maintenance loop drives each domain (outbox, ingestion, Provider migration, group
  archive, wiki backfill/relay/events/sources/delivery renewal) under its own time budget with
  panic and error isolation: one slow webhook or panicking phase can delay only its own domain,
  never starve the others. A phase that exceeds its budget (`SDKWORK_KNOWLEDGEBASE_WORKER_PHASE_TIMEOUT_SECONDS`,
  default 60 s) keeps running in the background and is reaped by later ticks (at most one instance
  per phase). Missed ticks are skipped, never burst-caught-up. Wiki domains advance independent
  in-memory cursors: a backfill candidate that fails is retried with exponential cooldown
  (30 s doubling to a 1 h cap) instead of stalling the rest of the domain, and a Drive event
  checkpoint with retried head events parks the consumer cursor until its stream catches up or
  the event dead-letters.
- Outbox delivery failures are scheduled with exponential backoff
  (30 s doubling to a 1 h cap, stored as `next_attempt_at`) so dead webhooks are not hammered every
  poll interval; events that exhaust `SDKWORK_KNOWLEDGEBASE_OUTBOX_MAX_RETRIES` move to the
  dead-letter status with an error-level log and a `knowledgebase_outbox_events_total{status="dead_lettered"}`
  metric. A stale/fenced completion never aborts the rest of the dispatch batch.
- Production Snowflake generators obtain fenced node IDs from `sdkwork_node_registry`. Lease loss disables ID generation and fails runtime readiness; Kubernetes supplies only the pod UID identity, never a hashed node ID.
- PostgreSQL is authoritative for server runtimes. Every process pool is created with fixed
  `app.current_tenant_id` and `app.current_organization_id` connection settings. Business RLS
  policies require both columns, use `FORCE ROW LEVEL SECURITY`, and are release-tested through a
  non-owner role. Server repository bootstrap rejects every non-PostgreSQL engine; no server binary
  compiles a SQLite driver. The SQLite fixture is exercised only by the repository test target
  (test-only `sqlx` `sqlite` dev-dependency), which applies its baseline and migrations to an
  in-memory database so the reference cannot rot.
- PostgreSQL migrations execute atomically (scripts without an explicit transaction boundary are
  wrapped in BEGIN/COMMIT by `sdkwork-database-lifecycle`), so a mid-script failure never leaves a
  partially migrated schema. The `202607310002_outbox_claim_fencing` migration resets only stale
  in-flight outbox claims (claimed longer than the five-minute stale window), preserving
  mid-delivery workers from silent duplicate delivery; the SQLite fixture migration carries the
  same guarded reset.
- Gateway, worker, and internal RPC processes enable the SDKWork process-shared database pool before
  module bootstrap. `SDKWORK_DATABASE_MAX_CONNECTIONS` is one combined process budget split between
  one scoped typed `PgPool` and one scoped `AnyPool` compatibility pool; odd budgets favor the typed
  pool and values below two fail startup. The default budget is 20 (10 typed + 10 compatibility) so
  ordinary write concurrency cannot exhaust the business pool. Every Knowledgebase pool pins
  `timezone=UTC`, a 30 s `statement_timeout`, and a 120 s
  `idle_in_transaction_session_timeout`; every `kb_*` business instant is stored as
  `TIMESTAMPTZ` (migration `0005_timestamptz_normalization` converts existing columns with
  `AT TIME ZONE 'UTC'`), and repository binds use `CAST(<rfc3339> AS TIMESTAMPTZ)`, so instants are
  independent of the host/container time zone. Drive, pgvector, and cloud
  provider resolution reuse the typed handle. In processes without a process-shared pool
  (application public ingress), the one-shot migration pool is closed right after bootstrap so the
  budget is held only by the business `AnyPool` and the Drive pool; the `AnyPool` applies the
  resolved `ssl_mode` to its connection URL and shares the typed pool's 10 s acquire timeout, so TLS
  policy cannot silently downgrade to plaintext. Quota-enforcing transactions additionally set a 3 s
  `lock_timeout` before taking the per-tenant advisory lock, so a contended writer fails its own
  request quickly instead of occupying a pooled connection until the statement timeout. The
  remaining `AnyPool` compatibility driver must be removed before the first commercial production
  release; see
  [ADR-20260730-knowledgebase-process-shared-database-pool.md](../decisions/ADR-20260730-knowledgebase-process-shared-database-pool.md).
- Media tasks consume the generated `cloudrouter-open-sdk` through the existing credential-resolving provider boundary. Image requests require URL output to keep base64 image payloads out of process memory; transcription accepts bounded HTTPS references and rejects local/private hosts.
- Audit retention is automated in the worker: one `audit_retention` phase deletes at most
  `SDKWORK_KNOWLEDGEBASE_AUDIT_RETENTION_BATCH_SIZE` (default 500, max 1000) `kb_audit_event`
  rows older than `SDKWORK_KNOWLEDGEBASE_AUDIT_RETENTION_DAYS` (default 365, 30..3650). The delete
  is a single statement over the `(tenant_id, organization_id, created_at, id)` index and skips
  events whose actor/resource has an active `kb_audit_legal_hold` row; the phase is bounded,
  idempotent, and reports `knowledgebase_audit_events_purged_total`. Legal holds are managed by
  backend admin operations `compliance.auditLegalHolds.{create,list,release.create}` (dual-token,
  `knowledge.platform.manage`, mutation-audited), which list with opaque cursor pagination.
- WeChat account/applet configuration is a tenant-scoped Drive object with a 1 MiB read/write
  boundary, encrypted secret fields, bounded unique entries, and fail-closed storage errors. A
  replacement contains at most 100 resources and each domain array at most 50 values. IDs/names are
  capped at 128 characters, avatar icons at 32, App IDs at 64, descriptions at 2,048,
  secrets/tokens at 256, AES keys at 43, verification filenames at 255, verification content
  at 64 KiB UTF-8, Official Account domains at 255, and Applet endpoints/paths at 2,048/1,024.
  Unknown fields, unsupported enums, embedded media identities, and overflow fail with canonical
  Problem Details before Drive I/O. Secret replacement merges each write-only field independently,
  so rotating an App secret does not clear an omitted token or AES key. See
  [REQ-2026-0731](../../product/requirements/REQ-2026-0731-wechat-config-input-boundaries.md). The
  read-modify-write replacement runs under a transaction-scoped PostgreSQL advisory lock
  (`pg_advisory_xact_lock` with a 3 s `lock_timeout`, namespace `KBWC`), so replicas serving the
  same tenant serialize instead of losing one side's update; the host injects the lock and the
  drive document itself still has no compare-and-swap precondition. External article publication
  is **disabled by default** and fails closed
  with `403 wechat_publish_disabled` unless
  `SDKWORK_KNOWLEDGEBASE_WECHAT_PUBLISH_ENABLED=true`. The switch is the release control while the
  managed Drive cover reference, durable idempotency, per-target outcome records,
  retry/reconciliation, partial-failure recovery, and live WeChat evidence remain open
  requirements; enabling it in a production-like environment emits an explicit warning. The preview
  operation is a local dry run that returns `status: "validated"` and performs no upstream call; the
  client labels it "validate preview" and states that nothing is sent. The previous embedded
  fallback thumbnail has been removed; a cover is either managed media data or an already-uploaded
  app-scoped WeChat media id, and remote cover URLs are rejected.
- Wiki publication projects Drive nodes under the fixed `sources/raw` root into per-file source,
  publication, visibility, route, render, and index state. Wiki persistence scope is the deployment
  organization that owns the space, never a personal session's organization claim, so every
  publication row shares the space row's `(tenant_id, organization_id)` ownership and passes the
  organization RLS policy. The bounded worker validates pinned
  Drive versions, canonicalizes native routes, sanitizes Markdown/HTML/text pages, and executes
  eligible owner-approved auto-public commands. Eligible native pages and passive static assets
  resolve live through the typed Knowledgebase Wiki provider; ordinary content changes advance
  live provider generations and create no immutable site artifact, Deploy Release, Deployment, or
  SiteRevision. Deploy owns Site/domain/Variant/TLS/runtime configuration and Web Server owns
  public HTTP/TLS/cache execution. See
  [ADR-20260721-live-mounted-wiki-publication.md](../decisions/ADR-20260721-live-mounted-wiki-publication.md)
  and [TECH-live-wiki-resource-provider.md](TECH-live-wiki-resource-provider.md).
- Backend administrative list handlers use cursor page contracts and push ordering, filtering, and limits into database queries; full-list downloads are not a production path. `page_size` is validated (rejected, not clamped) identically across app and backend surfaces, and all three business API surfaces enforce an explicit 1 MiB transport-level request body limit so oversized payloads fail before JSON deserialization.
- The compliance audit export is a deliberately bounded synchronous exception: it loads at most
  5,000 events plus one overflow probe and returns `413 audit_export_limit_exceeded` when the result
  cannot be complete. It never returns a silently truncated archive. Larger data-subject exports
  require a future cursor-based or asynchronous contract with generated SDK support. Automated
  retention and legal-hold-aware batch purge are implemented by the worker `audit_retention` phase
  (bounded batch, 30..3650-day policy, hold-aware single-statement delete) with
  `knowledgebase_audit_events_purged_total` exported; production-like purge evidence remains a
  release gate. The matching `(tenant_id, organization_id, actor_id, created_at DESC, id DESC)`
  composite index and the purge index `(tenant_id, organization_id, created_at, id)` exist on
  PostgreSQL; query-plan and concurrency evidence under load remains a commercial activation gate.
- The persistence contract stores only write-only Provider credential references, never plaintext
  credentials. Binding lifecycle and Provider-to-Provider migration checkpoints are
  tenant/organization scoped, version-fenced, RLS protected, and retain predecessors for rollback.
  After execution-handle authorization, runtime resolution loads the current Binding reference
  through an injected port and passes a one-time redacted/zeroized credential to the adapter.
  Adapter startup config and route crates never read secrets. The dedicated Provider secret adapter
  requires a tenant/organization/space/Binding/actor/operation/trace/deadline access context.
  Development and test allow only namespaced
  `env://SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_<PROVIDER_CODE>_*` values and bounded regular
  `file://` values whose canonical path remains under
  `SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRETS_DIR/<provider-code>/`. Cross-Provider credential locators
  fail closed. Staging and production
  allow only `secret://knowledgebase/provider/...` through an injected Kernel `SecretProvider`;
  default production construction fails closed when no approved resolver is provided. Resolution
  requires a managed audit record identifier, caps values at 64 KiB, and applies one five-second-or-
  shorter request budget across admission and backend execution. A 32-call default bulkhead retains
  permits for timed-out synchronous calls until the backend returns, so stuck backends cannot create
  unbounded blocking work; file buffers and intermediate plaintext are zeroized on every exit path.
  Telemetry emits no locator, secret id, or value and uses fixed outcome categories. Management
  writes validate the active environment policy before persistence
  without loading secret material. No secret cache exists, so rotation/revocation cannot leave
  stale process-local credentials. The backend
  management authority and generated SDKs provide credential-reference create/list/retrieve/rotate/
  revoke, space-scoped Binding create/list/retrieve/update/test/activate/disable, and migration
  create/list/retrieve/rollback with optimistic versions and resource-aware mutation audit. The
  migration Worker uses fenced owner/token leases, one phase per claim, durable checkpoints,
  observation deferral, and transactional Binding cutover/rollback. Audit payloads are field-whitelisted to resource
  type/id, URL space, expected/result version, and result status; credential locators, fingerprints,
  remote resource IDs, raw requests, and secret values cannot enter the audit event. Provider
  Certification v2 executes a versioned six-dimension contract suite for all ten adapters and
  fingerprints its local evidence. Live promotion is a separate release gate requiring a pinned
  upstream version and SHA-256-bound quality, contract, load/SLO, outage, licensing, and
  security/privacy evidence with adapter-commit, workflow, reviewer, expiry, and approval bindings.
  Quality evidence additionally requires a reviewed production-domain dataset, bounded minimum
  coverage, exact query/run cardinality, raw result and report provenance, and deterministic metric
  recomputation; contract samples cannot satisfy this gate.
  The dedicated PC backend-admin Provider package exposes credential-reference, Binding, and
  migration workflows through admin-core and the composed backend SDK with cursor pagination,
  optimistic versions, capability/lifecycle guards, write-only locator inputs, and sanitized states.
  A separate one-shot Worker operations entrypoint produces the prelaunch active-external-space
  readiness report through a service read-model port and SQLx adapter. It requires explicit tenant
  and organization scope, uses opaque keyset pagination at the store, and cannot read sources,
  credentials, or remote Provider resource identifiers. It is informational only and cannot create
  or infer Bindings.
  Current `liveCertifiedCount` is zero. A concrete production secret-manager/KMS backend with
  bounded TLS transport, durable audit retention and rotation/revocation drill evidence,
  authenticated operator UI acceptance, release PostgreSQL evidence, and live certification remain
  prelaunch gates.
- Provider health probes native infrastructure without credentials and probes external Providers
  only through active Bindings using the authenticated backend Operator and trace. External probes
  stream paginated Bindings with a fixed concurrency bound of eight and the request deadline. The
  health path never binds an external Provider at startup or fabricates a system business context.
- PC client: `apps/sdkwork-knowledgebase-pc/`

## 4. Security Model

- Production/staging boot is fail-closed on the standalone gateway: distributed Redis rate
  limiting, idempotency, and concurrent-admission stores (via `SDKWORK_KNOWLEDGEBASE_REDIS_*`), a
  JWT issuer/audience claim policy for `sdkwork-knowledgebase-pc`, IAM audit/security-event
  emitters, and PostgreSQL are all required; `SDKWORK_KNOWLEDGEBASE_ENVIRONMENT` is the single
  environment authority and is exported as `SDKWORK_ENVIRONMENT` so the web framework and
  knowledgebase-local checks can never disagree
- Backend OpenAPI declares `x-sdkwork-permission: knowledge.platform.manage` on all protected operations
- Public ingress exposes API paths only; `/metrics` is scraped via ServiceMonitor inside the cluster
- PC production builds disable demo/mock API fallbacks
- Space and browser read paths are fail-closed: without an injected access control the operation is
  rejected (never degraded to tenant-scoped visibility), and an anonymous browser identity is
  rejected instead of falling back to an `anonymous` actor. Grant/revoke paths enforce the same
  requirement symmetrically.
- Knowledgebase, Official Account, and Applet configuration surfaces expose icon/emoji selection,
  not browser Data URL avatar persistence. Custom media remains unavailable until an app-api/Drive
  contract can store a stable managed object reference and resolve bounded delivery URLs. WeChat
  domain-verification TXT selection enforces a 64 KiB metadata limit before `FileReader`, reports
  read failure, and resets the input after every accepted or rejected selection.
- Desktop native resource and export bridges enforce component-declared byte and concurrency limits.
  Generated PDF files are metadata-checked, then read through a 64 MiB plus one-byte overflow probe,
  so oversized output is rejected during bounded IO instead of after an unbounded allocation.
- Managed group spaces use `kb_group_knowledge_space_binding` instead of generic context binding.
  The binding is scoped by tenant, organization, and IM Conversation id; group spaces are hidden
  from generic resource routes and resolved only through the specialized launch path.
- Agent profiles carry a create-time owner (`kb_agent_profile.created_by`). Read/update/delete
  require the owner or access through the profile's enabled space bindings; a binding-less profile
  with no recorded owner is denied (`403 agent_profile_access_denied`) instead of being reachable by
  every authenticated user in the tenant. The owner is taken from the authenticated actor and is
  never accepted from the request body.
- The group resolver requires both a synchronized IM role snapshot and direct Drive authorization.
  Current-Owner initialization and active-content access are separate: only the current IM Owner
  may initialize or retry failed provisioning. Once active, Owner maps to Owner, Admin to Writer,
  Member to Reader, and Guest to no access; left, removed, and non-member actors are also denied.
  ACL projection failure is fail-closed, and `active` binding state requires an active ACL
  projection.
- IM launch tickets are opaque, hash-stored, one-time, short-lived capabilities bound to verified
  actor/session scope, binding version, and membership epoch. Browser tickets are fragment-only;
  desktop tickets are transient deep-link data and never persistent host state.

## 4.1 Managed Group Knowledgebase Boundary

IM owns the Conversation roster and lifecycle. Knowledgebase owns the one-to-one managed binding,
space/Drive lifecycle, ACL projection, and final content enforcement. Trusted IM service calls use
the generated SDK/RPC boundary; the authenticated Knowledgebase App API consumes a ticket and
resolves the exact binding target. IM alone applies current-Owner initialization and retry
authorization before it requests provisioning; Knowledgebase never treats a browser-supplied role
as authority. It accepts launch tickets only after the binding is active and the interactive caller
is a joined non-Guest Owner, Admin, or Member. The browser opens the standalone `/group-launch`
route under its configured public base path. The desktop handoff uses the independent Knowledgebase
Tauri process, not an IM-owned iframe or Webview. See
[ADR-20260713-group-knowledgebase-binding-and-launch.md](../decisions/ADR-20260713-group-knowledgebase-binding-and-launch.md).

## 5. Deployment Topology

Production uses `cloud.production`; process decomposition remains an implementation detail inside
that profile. Kubernetes runs one replicated `application.public-ingress` Deployment for all
application HTTP route surfaces and one replicated worker Deployment. Every replica uses the same
deployment-managed tenant/organization identity, PostgreSQL pool scope, and shared Drive object-storage provider selected by
`SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_PROVIDER_ID`; pod-local files are not authoritative cloud
content. The platform cloud gateway preserves distinct app/backend/open authorities while routing
them to the same bounded public-ingress Service. See `deployments/README.md` and `etc/topology/`.
Scope switching inside a process is unsupported. Separate tenant/organization deployments may
share the PostgreSQL cluster and schema only because repository predicates and RLS bind both ids.

## 6. Verification

```bash
pnpm check
pnpm check:app-composition
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-process-shared-database-pool.mjs --root .
pnpm verify
pnpm test
```

Gates include architecture alignment, `verify-repo` native composition, PC app hygiene (SDK boundary), utils integration, API envelope, SDK generation, database contract, isolation, and commercial release-readiness scripts.

The repository-level deploy validator currently resolves the declared `cloud.production` default.
Full `pnpm verify` and profile-specific deploy facade evidence remain prelaunch-gated by the
cross-repository blockers recorded in [PRD-mvp-launch.md](../../product/prd/PRD-mvp-launch.md); a
successful generic deploy validation must not be represented as complete release verification.

Launch acceptance: [PRD-mvp-launch.md](../../product/prd/PRD-mvp-launch.md).
Commercial readiness: [PRD-phase2-commercial-saas.md](../../product/prd/PRD-phase2-commercial-saas.md).
