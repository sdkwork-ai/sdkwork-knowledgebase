# SDKWork Knowledgebase - MVP Launch Acceptance

Status: prelaunch-gated
Owner: SDKWork maintainers
Application: sdkwork-knowledgebase
Updated: 2026-07-31
Parent: [PRD.md](PRD.md)

## Purpose

Phase 0.1 exit criteria and Phase 1.0 launch acceptance checklist for SDKWork Knowledgebase. This document records repository readiness gates and remaining release blockers; it is not a production release evidence record.

## Commercialization Readiness Decision

Decision: SDKWork Knowledgebase remains prelaunch and must not be treated as a production/commercial release until release-governance evidence is attached. The app manifest remains a `DRAFT` with `publish.preLaunch=true`, `release.defaultChannel=DEV`, disabled prelaunch packages, and disabled placeholder media projection.

- [x] Align manifest launch state: `sdkwork.app.config.json` projects `publish.status=DRAFT`, `publish.preLaunch=true`, `release.defaultChannel=DEV`, `release.latest.DEV=0.1.0`, and `publish.metadata.releaseStatus=prelaunch-gated`.
- [ ] Replace placeholder catalog media: icon, screenshot, and preview entries are disabled with `generatedPlaceholder=true` and `releaseStatus=prelaunch-placeholder`; production listing requires Drive-backed, real product media assets.
- [ ] Attach `web-universal-cloud-browser-zip` artifact evidence: checksum value, signing evidence, SBOM, provenance/attestation, immutable artifact URL or digest, and build workflow run.
- [ ] Record rollout, rollback, monitoring, and smoke-test evidence for each runtime target and deployment profile.
- [x] Reproducible frontend dependency installation restored: `pnpm install --frozen-lockfile`
  passes with the updated `pnpm-lock.yaml` (workspace `@sdkwork/utils` links added for the
  `sdkwork-base-data-backend-sdk` and IAM SDK families; reviewed supply-chain diff).
- [x] Full Rust workspace compilation restored: `cargo check --workspace` passes for the
  Knowledgebase workspace.
- [ ] Align profile-specific deploy argument forwarding between sibling `sdkwork-app-topology` and
  `sdkwork-specs/deployctl`: the generated `deploy:validate:cloud` facade passes
  `--deployment-profile cloud`, while `deployctl` currently accepts only `--profile`. The generic
  `deploy:validate` path successfully validates the declared `cloud.production` default, but that
  does not prove the profile-specific lifecycle contract is repaired.
- [ ] Run and record release-environment PostgreSQL verification with `SDKWORK_DATABASE_URL` pointing at the target PostgreSQL service; local fixture and contract gates are not enough for a commercial cutover claim.
- [x] Automated audit retention with legal hold: `kb_audit_legal_hold` holds by actor/resource, the batched PostgreSQL delete skips held events, worker phase `audit_retention` defaults to 365 days / 500 events and records `knowledgebase_audit_events_purged_total`; backend admin operations `compliance.auditLegalHolds.{create,list,release.create}` manage holds (dual-token, `knowledge.platform.manage`, mutation-audited), and a live drill proved unheld events are purged and held events survive.
- [ ] Add a cursor-based or asynchronous audit export contract and generated SDK support for subjects above 5,000 events. The current synchronous export fails explicitly with HTTP `413` instead of returning a partial result.
- [x] PostgreSQL audit subject composite index `idx_kb_audit_event_scope_actor_created`
  `(tenant_id, organization_id, actor_id, created_at DESC, id DESC)` exists in the baseline and in
  migration `202608040002_audit_event_scope_actor_index` (folded section). Release-environment
  query-plan and concurrent export/anonymization load evidence remains a cutover item.
- [ ] Replace the WeChat article `cover` URL field with a managed Drive object reference, resolve bounded cover bytes through the Drive server-side boundary, and attach real WeChat cover publish/preview evidence. External publication is disabled by default (`SDKWORK_KNOWLEDGEBASE_WECHAT_PUBLISH_ENABLED`); preview is a local dry run that sends nothing. URL-only and placeholder covers are forbidden.
- [ ] Add durable idempotency, per-account/per-recipient outcome records, retry/reconciliation, and partial-failure recovery before enabling multi-account WeChat publishing or multi-recipient preview (the default-off publication gate is the current control).
- [x] Serialize WeChat configuration replacement across writers: a transaction-scoped PostgreSQL advisory lock (namespace `KBWC`, 3 s `lock_timeout`) wraps the read-modify-write, so a second writer receives a conflict instead of silently losing an update. The Drive object still has no compare-and-swap precondition (upstream limitation).
- [ ] Add an end-to-end managed Drive object-reference contract before offering custom Knowledgebase,
  Official Account, or Applet avatar images. Browser Data URLs and client-local registry values are
  not valid commercial media identity; the current product surface deliberately offers bounded
  icon/emoji selection only.
- [ ] Run and archive final launch gates on the release candidate artifact: `pnpm verify`, `pnpm test`, `pnpm test:e2e:playwright`, and live smoke probes with configured app/backend/open API URLs.

## Phase 0.1 Exit Criteria

### Security

- [x] Public ingress does not expose `/metrics`; Prometheus scrapes via ServiceMonitor only
- [x] Backend OpenAPI declares `x-sdkwork-permission: knowledge.platform.manage` on all protected operations
- [x] Upload session space ACL enforced (`require_space_access` on create/complete)
- [x] Space ACL fail-closed when drive binding missing
- [x] WeChat editor HTML sanitized before insertion
- [x] Production demo/synthetic UI data gated by `shouldUseKnowledgebaseDemoFallback()` (WeChat insert, applet modal, settings icon choices, music player, widget templates without external URLs)
- [x] Tenant-scoped dynamic rate limit policy wired from web store (`SqlxDynamicPolicyBundle`) on all HTTP surfaces
- [x] External/catalog adapter engines return `Unsupported` for `list_documents` instead of silent empty lists
- [x] User-facing mutation errors surfaced via `toastKnowledgebaseError` in core KB flows
- [x] `pnpm test:security` passes (tenant isolation, RBAC, audit, demo gating)
- [x] WeChat credentials encrypted at rest; `encrypt_secret` fails closed when `SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY` is unset (no plaintext fallback)

### API & SDK

- [x] `pnpm api:materialize:check` passes (auth-mode, permissions, authority sync)
- [x] `pnpm sdk:generate:check` passes
- [x] Open API included in `verify_openapi_operation_ids.ps1` and phase1 generated SDK roots
- [x] `specs/component.spec.json` indexes all three HTTP surfaces and SDK clients
- [x] Browser list uses `KnowledgeBrowserListData`; OKF `view=files` resolves to original files under `sources/raw`, `view=okf_bundle` resolves to generated bundle files under `okf`, and root create/upload clients use response `data.parentId`.

### Reliability

- [x] Worker `/readyz` probes database connectivity (requires connection pool, not business queries)
- [x] App API `/readyz` simplified to dependency connectivity checks only (see Phase 0.4)
- [x] `list_browser` enforces `ensure_runtime_tenant` like other hosted app routes
- [x] Repository hot paths bounded: chunk load cap, drive ref prefix limit, OKF link list limits
- [x] Agent provider `block_on_async` reuses a dedicated bridge thread/runtime (no per-call OS thread spawn)
- [x] Worker resource-based HPA, Service, and ServiceMonitor configured; custom backlog metrics remain disabled until a Prometheus Adapter rule exists
- [x] K8s manifests include PDB, NetworkPolicy, and `securityContext`
- [x] Ingest pipelines log failures when `mark_failed` cannot persist state
- [x] Production topology documents mandatory Outbox webhook configuration
- [x] WeChat tenant configuration reads and writes are bounded to 1 MiB, reject duplicate/oversized account and applet sets, and preserve existing encrypted configuration when Drive reads fail

### Frontend

- [x] No `@packages/` deep imports; package boundary imports only
- [x] Demo/mock fallbacks disabled in production builds (`import.meta.env.PROD`)
- [x] Offline import modals (chat file/dialog, notes) show honest empty states when IM connector is not wired; batch import remains gated by `assertKnowledgebasePreviewFeature`
- [x] WeChat save-as-draft persists via `WechatService.publishArticles` (WeChat draft box API)
- [x] AI assistant uses backend agent when `isKnowledgebaseApiAvailable()`; local MCP agent is demo-only
- [x] Image viewer AI tools hidden outside `shouldUseKnowledgebaseDemoFallback()`
- [x] Asset library scan capped (`MAX_ASSET_LIBRARY_ITEMS` / `MAX_ASSET_SCAN_NODES`) with truncation banner
- [x] Asset library modal uses cursor pagination (`listAssetLibraryItemsPage`) with Load more; no synthetic third-party demo assets (API-backed or empty state only)
- [x] Knowledge space members settings use paginated first page; full baseline fetched on save only when members changed
- [x] Partial member sync preserves unloaded baseline members (`buildPartialMemberSyncPayload`); baseline fetched on save only when members changed
- [x] Auto-save and editor uploads surface i18n errors via `toastKnowledgebaseError`; numeric ProblemDetail `60002` maps to tenant quota message
- [x] Editor demo upload uses blob URLs only under `shouldUseKnowledgebaseDemoFallback()`
- [x] Permissions modal uses paginated member count (`20+` when truncated)
- [x] Cloud drive import modal uses cursor pagination (`listBrowserItemsPage`, `listStarredItemsPage`, `listRecentItemsPage`, `listSharedItemsPage`) with Load more across my-drive, starred, recent, and shared tabs
- [x] Cloud drive interactive browse never prefetches multi-page collections or caps an in-memory aggregate; each tab requests the next Drive SDK cursor page on demand
- [x] Cloud drive collection tabs do not synthesize Drive metadata; when the Drive SDK page lacks `updatedAt`, the UI renders an explicit `--` placeholder instead of a generated timestamp
- [x] Drive import pipeline enforces `MAX_MARKDOWN_PAYLOAD_BYTES` before chunking
- [x] WeChat typography preview uses article author and current date instead of hardcoded demo metadata
- [x] WeChat publish/upload/AI stream API failures use `toastKnowledgebaseError` (quota/offline/network aware)
- [x] Network offline fail-closed: `AppShell` wires `setKnowledgebaseNetworkOnline`; mutations call `requireKnowledgebaseNetworkOnline`
- [x] i18n keys for `network.offline` and `feature.previewOnly` error surfaces
- [x] Unimplemented split-view menu removed (no false success toast)
- [x] Document export path re-sanitizes HTML before `innerHTML` assignment; native PDF output is
  metadata-checked and read through a 64 MiB plus one-byte overflow probe before it enters memory
- [x] Unsupported browser Data URL avatar uploads were removed from Knowledgebase, Official Account,
  and Applet configuration. Those surfaces expose icon/emoji selection until a managed Drive media
  contract exists; they do not persist `File`, object URL, Data URL, or transient download URL as
  business media identity.
- [x] Official Account and Applet domain-verification TXT files are rejected before `FileReader`
  when they exceed 64 KiB; invalid extensions, read failures, and repeated selection of the same
  file are handled explicitly by resetting the input.
- [x] `pnpm lint` (`tsc --noEmit`) passes on the current checkout with the repaired dependency
  graph.
- [x] Ad-hoc root migration scripts removed; `pnpm check:pc-app-hygiene` passes
- [x] Browser/desktop staging and production config examples present
- [x] OKF file list is an original-source file surface: the PC file list calls browser `view=files`, does not show `okf/`, `output/`, `.sdkwork/`, or Drive root system folders, and OKF concept copy/move tooling explicitly calls `view=okf_bundle`.

### Verification

```bash
pnpm verify
pnpm test
pnpm lint
```

## Phase 1.0 Launch Acceptance

### Functional

- [x] Author scenario: login -> create note -> edit document -> auto-save (`e2e/author.flow.spec.ts`, Playwright CI)
- [x] Search scenario: RAG answer with citations navigates to source document (`e2e/search.flow.spec.ts`, Playwright CI)
- [x] Admin scenario contract: backend source listing requires `knowledge.platform.manage` (`scripts/smoke-knowledgebase-admin-ingest.test.mjs`; live probe optional via `SDKWORK_KNOWLEDGEBASE_SMOKE_BACKEND_URL`)
- [x] Open API scenario contract: api-key `context_packs` and `retrievals` (`scripts/smoke-knowledgebase-open-api.test.mjs`; live probe optional via `SDKWORK_KNOWLEDGEBASE_SMOKE_OPEN_URL`)
- [x] WeChat publish modal uses API-backed account selection and fan tag groups load from WeChat `tags/get` through `wechat.officialAccounts.fanTags.list`; article/account/recipient request sizes are bounded.
- [x] WeChat publish path blocks demo fallback in production builds (`shouldUseKnowledgebaseDemoFallback`; hosted API smoke optional before cutover)
- [ ] WeChat publish and preview are enabled end to end only after the managed Drive cover contract, generated SDKs, durable idempotency, partial-failure recovery, configuration concurrency control, and live upstream evidence exist. External publication is currently disabled by default and rejects with `403 wechat_publish_disabled`; preview validates the payload locally and sends nothing.

### Operations

- [x] PostgreSQL schema, migration ordering, and RLS contracts are covered by repository/static gates;
  non-owner execution, upgrade/recovery, and target release-environment PostgreSQL evidence remain
  cutover blockers above.
- [x] Backup/restore runbook documented (`deployments/runbooks/backup-restore.md`) and referenced by launch runbook
- [x] Application public-ingress smoke script: public `SDKWORK_KNOWLEDGEBASE_SMOKE_BASE_URL=... pnpm test:smoke` probes `/livez` and `/readyz`; optional internal `SDKWORK_KNOWLEDGEBASE_SMOKE_METRICS_URLS=... pnpm test:smoke` probes `/metrics` through in-cluster Service URLs only.
- [x] JSON logging enabled in production topology; OTEL documented when collector is available (`SDKWORK_KNOWLEDGEBASE_LOG_FORMAT=json`)
- [x] Audit retention is enforced automatically: worker phase `audit_retention` purges expired `kb_audit_event` rows in bounded batches, active legal holds are skipped by the delete predicate, `knowledgebase_audit_events_purged_total` is exported, and a PostgreSQL drill proved purge-plus-hold behavior.
- [ ] GDPR audit export supports more than 5,000 events through a bounded paginated or asynchronous workflow; the synchronous endpoint must never be treated as an unbounded archive download.

### Release

- [ ] Web bundle `web-universal-cloud-browser-zip` release evidence attached: checksum, signature, SBOM, provenance/attestation, immutable artifact reference, and workflow run. The manifest declares these controls as required and keeps the package disabled until evidence exists.
- [x] Four SDK families (app, backend, internal, and open) indexed for governed consumption (`specs/component.spec.json`)
- [x] Desktop packaging workflow explicitly prelaunch-disabled until desktop CI targets ship (`sdkwork.app.config.json` metadata)

Launch orchestration runbook: [deployments/runbooks/production-launch.md](../../../deployments/runbooks/production-launch.md)

Automated launch gate:

```bash
pnpm test:launch-readiness
pnpm test:e2e:playwright
```

## Success Metrics (from PRD)

| Metric | Target |
|--------|--------|
| API availability (per tenant deployment) | 99.5% monthly |
| P95 retrieval latency (warm index) | < 2s |
| Authz failures return 403 without data leak | 100% in integration tests |
| PC shell smoke (login + load) | Pass in CI Playwright |
| PC author/search launch flows | Pass in CI Playwright |
| Document save success when online | > 99% |

## Out of Scope for 1.0

- Shared request-scoped tenant pooling; commercial release gates are tracked in
  [PRD-phase2-commercial-saas.md](PRD-phase2-commercial-saas.md)
- Real-time collaborative editing
- Mobile native clients
- SOC2 program (platform-level)

## Database migration authority

Canonical lifecycle assets live under `database/` (PostgreSQL-only baseline plus ordered
migrations). The repository crate's `migrations/` directory is a deprecated, unreferenced mirror of
historical migrations and is scheduled for removal under the destructive-change policy; do not add
files there. New schema changes go to `database/migrations/postgres/` and the baseline, then
`pnpm db:validate`.
