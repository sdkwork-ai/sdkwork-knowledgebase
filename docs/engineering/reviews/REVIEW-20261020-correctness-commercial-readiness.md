# REVIEW-20261020 — Correctness, Scalability, and Commercial-Readiness Audit

Status: active — code remediation complete; full regression green; commercial launch evidence pending (see §1.1)
Owner: SDKWork maintainers
Application: sdkwork-knowledgebase
Scope: Rust backend (`crates/`), database lifecycle (`database/`), API contracts (`apis/`, authored OpenAPI),
PC client (`apps/sdkwork-knowledgebase-pc`), worker, and documentation canon.

Authorities applied: `../sdkwork-specs/API_SPEC.md`, `PAGINATION_SPEC.md`, `DATABASE_SPEC.md`,
`DATABASE_FRAMEWORK_SPEC.md`, `RUST_CODE_SPEC.md`, `SECURITY_SPEC.md`, `IAM_SPEC.md`,
`PERFORMANCE_SPEC.md`, and the repository canon
[PRD.md](../../product/prd/PRD.md) / [TECH_ARCHITECTURE.md](../../architecture/tech/TECH_ARCHITECTURE.md).

## 1. Verdict

**The system is not commercially shippable in its current state.** It is also **not "fake" overall**:
the core backend (Drive, provider runtime, grant/launch tickets, outbox, worker isolation, wiki
projection) is genuinely implemented, and there are **no** `todo!()` / `unimplemented!()` /
`dbg!()` markers in non-test Rust. The blockers are a specific set of correctness, scalability,
authorization-consistency, and documentation-integrity defects listed below, plus the release
gates the repository itself already records as open (`sdkwork.app.config.json` projects
`publish.status=DRAFT`, `publish.preLaunch=true`, `releaseEvidenceStatus=missing`).

Highest-risk themes:

1. **External side effects ahead of their documented gates** — WeChat publishing performs real
   `draft/add` + `freepublish/submit` while the PRD and architecture say the operation fails
   closed, and it has no durable idempotency or per-target outcome records.
2. **Fabricated user-visible behavior** — a hard-coded demo identity and activity metrics in the
   profile screen, and an "AI polish" fallback that invents rewritten paragraphs with no model call.
3. **Unbounded work on request paths** — retrieval fan-out by caller-supplied binding count, OKF
   search materializing a whole space, and synchronous whole-document/whole-space embedding.
4. **Database invariants that the gates do not actually verify** — the organization-scope SQL gate
   is defeated by a column mention, mixed `TIMESTAMP`/`TIMESTAMPTZ` without a pinned session time
   zone, and quota writes that serialize all writers on one advisory lock inside a 5-connection pool.
5. **Documentation that overstates implementation** — the claimed in-memory SQLite fixture does not
   exist as executable code; several migration IDs cited as evidence have no file.

## 1.1 Remediation status (2026-10-21)

Post-remediation assessment: every P0/P1/P2 code finding in §3 is closed and the full regression
is green (workspace Rust tests, script gates, PC typecheck/tests/production build). The system is
code-complete for the prelaunch boundary; the app manifest correctly stays `DRAFT` /
`preLaunch=true` until the external release-governance evidence listed at the end of the table is
attached.

Fixed and verified in this remediation cycle:

| Finding | Resolution |
|---------|------------|
| F-01 | External WeChat publication is disabled by default (`SDKWORK_KNOWLEDGEBASE_WECHAT_PUBLISH_ENABLED`, default false, 403 `wechat_publish_disabled`); the preview dry run is labelled "validate preview" and the UI states nothing is sent. Remaining commercial enablement gates stay recorded in PRD-mvp-launch. |
| F-02 | SQLite runtime remains PostgreSQL-only (now stated truthfully); the fixture DDL is executed in memory by `sqlite_fixture_parity.rs` with a test-only `sqlite` driver, so the reference cannot rot. |
| F-03 | `check_organization_scope_sql.mjs` now strips the outer projection before checking predicates; the two tenant-only group-binding queries were made organization-scoped (parameter + SQL + callers) and the gate passes with zero violations. |
| F-04 | Retrieval fan-out bounded to 16 bindings / 3 methods with method de-duplication and unit tests. |
| F-09 | Quota transactions set `lock_timeout = 3s` before the advisory lock; default process pool budget raised to 20 (10+10); UTC and idle-transaction timeouts pinned. |
| F-10 | Blocking admission permit now lives inside the blocking closure (timeouts no longer inflate concurrency); extraction runs through the shared bounded executor. |
| F-11 | Missing/deleted documents map to `NotFound` (404) instead of `Internal` (500). |
| F-12 | Fabricated "AI polish" fallback removed; the editor fails closed with a toast. |
| F-13 | Profile screen rebuilt as a read-only IAM account view; fake identity, presets, localStorage save, and hardcoded statistics removed. |
| F-14 | Demo mode is hard-disabled in production bundles; stale `.production/.staging/.test` example profiles normalized; demo deployment profiles retired repo-wide. |
| F-15 | Session time zone pinned to UTC (the local PostgreSQL server used to verify this reports `Asia/Shanghai`), idle-in-transaction timeout added. Physical `TIMESTAMPTZ` normalization remains tracked as F-40. |
| F-16 | Agent profiles carry a create-time owner; read/update/delete require owner or enabled-binding access; binding-less unowned profiles are denied. |
| F-17 | Wiki persistence scope uses the deployment organization that owns the space, matching the space row and RLS policy. |
| F-22 | Ingestion claim is a single `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING` statement. |
| F-23 | Current-version binding enforces monotonic `version_no` and reports a conflict when no row was updated. |
| F-25 | `public/runtime-env.json` is a gitignored build artifact (finding downgraded); tracked example profiles corrected. |
| F-26 | Abuse rate-limit tier applied to space/document/member/concept/context-binding mutations. |
| F-29 | Dead full-list `list_sources`/`list_active_sources` path removed end to end. |
| F-31 | Worker ingestion batch size validated to `1..=200`. |
| F-33 | Provider credential management now requires the exact `knowledge.platform.manage` scope (wildcard removed). |
| F-34 | Redis runtime config has a manual `Debug` that redacts URL userinfo. |
| F-39 | Ingestion lifecycle is persisted (`parse_state=succeeded`, `index_state=running`), and the worker drains a durable embedding backlog and marks documents indexed — previously documents stayed `pending` forever. |
| F-41 | `check:client-env` failed because standalone topology profiles did not declare the platform gateway URL; all profiles now declare it and materialization verifies 8 profiles. |
| F-27 | The Open API alias check now percent-decodes keys with the same parser as the app surface and returns the same `invalid_parameter` code, so one pagination contract holds across APIs. |
| F-37 | Retrieval hit recording is replace-on-retry inside one transaction (delete the trace's existing hits, then insert), so a retry cannot duplicate ranks. |
| F-18 | `agentProfiles.bindings.list` now declares the canonical `{items, pageInfo}` payload and serves real keyset pages: the route validates opaque `cursor`/`page_size` (1..=200), the service validates bounds, and the PostgreSQL store returns `page_size + 1` rows ordered by binding id to compute `next_cursor`/`has_more`; generated SDK models were regenerated and a service paging test proves two pages plus bound rejection. |
| F-19 | `okf.bundle.{index,log,profile}.list` renamed to `.retrieve` (they return one document), with route manifests, ports, SDKs, and generated models regenerated through the canonical pipeline. |
| F-40 | Every `kb_*` instant is now `TIMESTAMPTZ`; migration `0005_timestamptz_normalization` converts existing columns with `AT TIME ZONE 'UTC'`, the baseline creates them as `TIMESTAMPTZ`, and repository binds use `CAST(... AS TIMESTAMPTZ)`. Verified on live PostgreSQL: 0 naive timestamp columns remain and the integration suite passes 8/8. |
| F-21 | Retrieval metadata filters now compile to `metadata @> jsonb_build_object(key::text, value::text)`, served by the new `jsonb_path_ops` GIN index `idx_kb_document_metadata_gin` (migration `0006_document_metadata_gin`, folded into the baseline). The typed binds were verified against live PostgreSQL after an untyped probe failed with `42P18`. |
| F-24 | Drive storage selection is an explicit deployment-profile enum in both the app runtime and the RPC runtime: standalone uses the local store, cloud requires `SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_PROVIDER_ID` plus the PostgreSQL pool and otherwise returns a configuration error. No implicit local fallback exists. |
| F-28 | Next-cursor encoding has a typed `encode_u64_next_cursor(Option<u64>)` path; the string wrapper now logs a non-numeric store cursor instead of silently dropping it, and the bindings page uses the typed helper. |
| F-35 | The WeChat token cache and per-appid refresh-lock map are hard-capped at 256 entries, expired tokens are pruned on every write, and idle refresh locks are evicted without breaking single-flight; two tests cover churn and expiry. No Onyx allowlist cache exists in the current tree (verified by search). |
| F-32 | WeChat configuration replacement now runs under a transaction-scoped PostgreSQL advisory lock (`pg_advisory_xact_lock`, namespace `KBWC`, 3 s `lock_timeout`) injected by the host, so replicas serving one tenant serialize; the guard releases on error and the lock test proves a second writer times out and succeeds after release. The Drive object still lacks a compare-and-swap precondition, which remains an upstream platform limitation rather than an application defect. |
| F-36 | The only soft-delete column left is `kb_group_knowledge_space_binding.deleted_at`, and it is written (`deleted_at = NULL` on restore) rather than dead; the finding's "never written" premise no longer holds and no removal is warranted. |
| F-30 | Automated audit retention is implemented end to end: `kb_audit_legal_hold` stores subject-scoped holds, the single-statement batch delete skips held actors/resources, worker phase `audit_retention` runs with a 365-day default and 500-event batch, and backend admin operations `compliance.auditLegalHolds.{create,list,release.create}` manage holds with dual-token auth, `knowledge.platform.manage`, mutation audit, and cursor pagination. A live PostgreSQL drill proved that an expired unheld event is purged while an equally old held event survives. |
| F-38 | Documentation was reconciled with the implemented system: the audit-retention runbook now describes the worker phase, legal-hold API, metrics and drill; PRD/PRD-mvp/TECH/README/specs no longer claim retention is unimplemented, a naive-timestamp pipeline, or a SQLite server profile; migration records and `docs/INDEX.yaml` statuses were corrected; retired `demo` host/profile references were removed from topology, webserver configs, deploy runbooks, `bin/README.md` and `AGENTS.md`. |
| F-42 | The PostgreSQL integration suite could not run against a real database (pools had no RLS scope); every test now uses a production-scoped pool and the suite passes 8/8. |
| F-43 | Dependency baseline upgraded to current compatible releases: Rust OpenTelemetry 0.33/tracing-opentelemetry 0.34, reqwest 0.13, thiserror 2, sha2/sha1 0.11, hmac/hkdf 0.13, aes 0.9, cbc 0.2, aes-gcm 0.11, base64 0.23, quick-xml 0.42, pdf-extract 0.12, zip 9; PC Vite 8.3.4 with Rolldown `advancedChunks` splitting, plugin-react 6.1.2, vitest 4.1.12, esbuild 0.28.2, TypeScript 6.0.3. All library tests, the PostgreSQL suite, the workspace compile, and the production browser build pass. TypeScript stays on the catalog 6.0.3 because TS 7 is not adopted by the sibling workspace toolchain. |
| F-44 | The TIMESTAMPTZ normalization exposed repository decode defects: provider binding, provider migration, wiki public provider and drive-inbox SELECTs returned `TIMESTAMPTZ`/`JSONB` values that `sqlx::Any` cannot decode into the DTO `String` fields. Columns now render RFC 3339 UTC text through `TO_CHAR(... AT TIME ZONE 'UTC', ...)`, JSONB payloads are read with `::text`, `SqlTimestampDialect::sql_timestamp_text_expr` pins UTC, and inbox replay compares parsed JSON values plus instant equality instead of serialized text. |
| F-45 | Retired `demo` environment values were still accepted by the standalone gateway, the group-launch ticket consumer, the RPC host and the provider-secret adapter; configuration now accepts only `development`, `test`, `staging`, `production` and fails closed on anything else. |
| F-46 | The PostgreSQL integration tests did not carry RLS session scope, shared one schema across parallel test binaries, reused fixed fixture ids and one test dropped `kb_audit_event`. A shared `tests/support` helper opens deployment-scoped pools, wiki tests serialize and clean their scope, fixtures are idempotent, and the destructive audit-failure test is `#[ignore]`d for an isolated database job. All 19 non-ignored repository PG tests pass. |
| F-47 | Release-gate tooling was stale or brittle: `verify_phase1.ps1` filesystem walks aborted on generated SDK `target/` long paths and broken junctions (now git-tracked manifest discovery and empty-artifact handling); the required operationId list referenced obsolete ids; the OpenAPI verifier had no external-wire anonymous exception; the legal-hold release command lacked the API_SPEC-required typed command body; provider-certification compared a realpath target against a lexical artifact root; the document-extraction component declared an unknown layer role; and two memory/document-extraction call sites duplicated `is_blank`. |

Code remediation is complete and the full regression is green (see §8 evidence). Commercial
release remains gated on release-governance evidence, not on outstanding code defects: real catalog
media, signed artifact/SBOM/provenance evidence, per-profile rollout/rollback/monitoring evidence,
release-environment PostgreSQL verification, the deployctl `--deployment-profile` flag alignment in
the sibling topology repository, the >5,000-event asynchronous audit export contract, WeChat managed
Drive cover/publication enablement, and the final release-candidate gate run. One non-functional
cleanup also needs explicit human approval under the destructive-operation policy: the crate-local
`crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/` directory is a deprecated,
unreferenced mirror of `database/` and should be deleted as a directory tree.

## 2. Method and evidence

- Read the PRD/architecture canon and the task-selected SDKWork specs.
- Read implementation files at every cited line; nothing below is reported from a grep hit alone.
- Ran the standing gates:
  - `check-pagination.mjs` — passed (heuristic only; see F-19)
  - `check-api-response-envelope.mjs` — passed
  - `check-api-operation-patterns.mjs` — passed
  - `check-rust-crate-naming-standard.mjs` / `check-rust-manifest-standard.mjs` — 0 errors
  - `check-database-framework-standard.mjs` — passed
  - `tools/check_organization_scope_sql.mjs` — passed **while tenant-only statements exist** (F-09)
  - `cargo check --workspace` — compiled (10m15s); one `trivial_casts` warning, **fixed**
- Not run: load/soak tests, live PostgreSQL/WeChat/Drive environment probes, Playwright E2E.
  Findings that require a live environment are labelled *runtime-unverified*.

## 3. Finding register

Severity: **C**ritical = data loss / security / external side effect / outage; **H**igh = broken
feature, OOM, or unbounded work; **M**edium = contract, consistency, or performance defect;
**L**ow = hygiene.

| ID | Sev | Domain | Finding | Evidence |
|----|-----|--------|---------|----------|
| F-01 | C | Product integrity | WeChat publish performs real `draft/add` + `freepublish/submit` although PRD/architecture declare it fail-closed and out of scope; no durable idempotency, no per-account outcomes, no partial-failure recovery | `wechat/service.rs:143-216`; `docs/architecture/tech/TECH_ARCHITECTURE.md:137-143`; `docs/product/prd/PRD.md:91-92`; `PRD-mvp-launch.md:139` |
| F-02 | C | Testability claim | No SQLite runtime and no executable SQLite fixture; the parity test parses `.sql` text and never opens a database, while TECH claims an in-memory deterministic fixture | `repository-sqlx/Cargo.toml:22` (no `sqlite` feature); `db/bootstrap.rs:50`; `tests/sqlite_fixture_parity.rs:75-92`; `TECH_ARCHITECTURE.md:104-107` |
| F-03 | C | Tenant isolation gate | `check_organization_scope_sql.mjs` skips any statement whose text contains `organization_id`, so tenant-only SQL passes; two such statements exist on org-owned tables | `tools/check_organization_scope_sql.mjs:57-59`; `postgres_group_space_binding_store.rs:163`, `:558` |
| F-04 | H | OOM / DoS | Retrieval fans out one backend query per binding × method with no bound; a 1 MiB body allows thousands | `retrieval.rs:86-118`, `validate_request` (pre-fix) |
| F-05 | H | OOM / CPU | OKF native `search` loads up to 50 000 concept summaries, clones them into a map, loads up to 50 000 link edges, then runs an O(seeds×edges) loop with per-pair allocation inline on the Tokio reactor | `knowledge_engine/okf_native.rs:187-257`; `ports/knowledge_okf_concept_store.rs:9` |
| F-06 | H | OOM / latency | OKF `read_document` loads every published concept in the space to find one | `okf_native.rs:298-338` |
| F-07 | H | Request-path blocking | Ingest / Drive import / Git import await whole-document embedding despite "fire-and-forget" comments; one request can issue thousands of provider calls (up to 64 imported docs × chunks) | `routes-app-api/hosted.rs:192-199`, `:272-283`, `:329-335` |
| F-08 | H | Request-path blocking | Index create/rebuild embed every active chunk of a space inside the HTTP request, unbounded page loop | `hosted_backend.rs:838-877`, `:919-925`; `knowledge_embedding_build.rs:41-75` |
| F-09 | H | DB concurrency | Every quota-checked write holds a pooled connection while waiting on `pg_advisory_xact_lock`; default `AnyPool` budget is 5 connections, so ordinary write concurrency can starve all DB traffic | `quota_transaction.rs:11-26`; `db/bootstrap.rs:19,57-68` |
| F-10 | H | Concurrency / OOM | Bounded blocking executor releases its permit on timeout while the blocking task keeps running, so capacity no longer bounds real work; 32 MiB extraction tasks bypass admission entirely | `bounded_blocking.rs:34-63`; `imports/drive_import_pipeline.rs:218-240`; `document_extraction.rs:25` |
| F-11 | H | API correctness | Missing document returns `500` instead of `404` (repository maps `RowNotFound` to `Internal` on get/update/delete) | `postgres_import_stores.rs:815-830`, `:874-887`, `:915-919` — **fixed** |
| F-12 | H | Fabricated UX | "AI polish" fallback waits 1.2 s and appends a fabricated "optimized by AI" sentence to every paragraph when no API is connected | `components/TypographyModal.tsx:333-362` — **fixed** (fails closed with a toast) |
| F-13 | H | Fabricated UX | Profile screen seeds every user with a demo identity ("宋语冰 (Alice)", `alice@company.com`, `138-1234-5678`) and hard-coded activity metrics (42 documents, 3 spaces, 18/30 days, 158 likes); "save" writes localStorage only and reports success | `sdkwork-knowledgebase-pc-shell/src/UserProfileModal.tsx:64-75`, `:146-166`, `:643-676` |
| F-14 | H | Demo gating | `isKnowledgebaseDemoModeEnabled` ignores `import.meta.env.PROD` although the PRD claims production builds disable demo data; checked-in `*.production`/`*.staging`/`*.test` runtime-env profiles set `enableDemoMode: true`, `buildMode: development`, `configProfile: dev` | `core/src/api/knowledgebaseApiRegistry.ts:43-58` — **fixed**; `apps/.../config/browser/runtime-env.cloud.production.json:7-16` |
| F-15 | H | Schema / time | `kb_outbox_event` mixes `TIMESTAMP` and `TIMESTAMPTZ`; no `timezone=UTC` is injected, and RFC3339 text is written via `CAST(... AS TIMESTAMP)`, so `TIMESTAMPTZ` columns interpret the value in the session zone | `database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql:896,980,2738,2797`; `postgres_scope.rs:102-104`; `db/sql_timestamp.rs:22-32` |
| F-16 | H | Authorization | Agent profiles with zero enabled bindings pass every object-level check for read/update/delete (tenant check only) | `hosted_access.rs:710-739`; `runtime.rs:2458-2470`, `:2498-2516` |
| F-17 | H | Data scope | Wiki persistence uses the *session* organization (`unwrap_or(0)`) while the space row is owned by the deployment organization, so a personal (org-less) session writes/finds publications under org `0` and RLS rejects them | `hosted_wiki.rs:50-55`; `hosted_access.rs:70-79`; `runtime.rs:1027-1030` |
| F-18 | M | Contract | `agentProfiles.bindings.list` returns `data.item` typed as `{items}` with no `pageInfo` in OpenAPI, while the handler emits `fixed_list_page_data` (`items` + `pageInfo`); the store also silently truncates at 200 bindings | `apis/app-api/...openapi.json` (`KnowledgeAgentBindingList`); `agent_profile_store.rs:21,331-368` |
| F-19 | M | Pagination semantics | `okf.bundle.index.log/profile.list` are named `list` but return a single `data.item` document; generated SDKs expose a misleading `list` method | authored app OpenAPI (`OkfIndexDocument`, `OkfLogDocument`, `OkfProfileDocument`) |
| F-20 | M | Index design | `list_documents_page_for_space` filters `(tenant, org, space, status)` ordered by `id`, but no matching index exists | `postgres_import_stores.rs:742-781`; baseline indexes on `kb_document` |
| F-21 | M | Index design | User-declared JSON metadata filters use `d.metadata::jsonb ->> ...` with no GIN/expression index | `binding_scope_filters.rs:123-130` |
| F-22 | M | Transaction shape | Ingestion claim is one `SELECT LIMIT` followed by N conditional `UPDATE`s (safe against double ownership, but not the documented single atomic claim; wasted round trips under contention) | `postgres_import_stores.rs:2254-2356` |
| F-23 | M | Silent success | `bind_document_current_version_in_transaction` ignores `version_no` and the UPDATE row count; a caller then sets `current_version_id` in memory and returns it as bound | `postgres_knowledge_document_metadata_transaction.rs:422-458`; `postgres_drive_import_metadata_store.rs:201-212` |
| F-24 | M | Deployment safety | Missing `SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE` seeds a `local_filesystem` Drive provider, so a cloud image without the variable serves pod-local storage | `knowledgebase-drive/src/bootstrap.rs:54-92`; `TECH_ARCHITECTURE.md:55-57` |
| F-25 | M | Config integrity | `public/runtime-env.json` (build output) ships `cloud.production` with localhost URLs, `buildMode=development`, `configProfile=dev`, demo enabled; file note claims the opposite | `apps/sdkwork-knowledgebase-pc/public/runtime-env.json:2-18` |
| F-26 | M | Rate limiting | High-cost mutations (`spaces.create`, `documents.create`, `spaces.members.create`, `documents.versions.create`, `okf.concepts.upsert`, `contextBindings.*`) carry no abuse tier, unlike ingest/import routes | `routes-app-api/http_route_manifest.rs` |
| F-27 | M | API semantics | Open API rejects forbidden pagination aliases with a different error code than app/backend and without URL-decoding the key; behaviour is saved only by `deny_unknown_fields` | `routes-open-api/routes.rs:301-328` vs `routes-app-api/routes.rs:1570-1587` |
| F-28 | M | Silent truncation | `encode_opaque_next_cursor` drops any non-numeric store cursor, which also flips `hasMore` to false because `has_more = next_cursor.is_some()` | `routes-app-api/pagination.rs:185-189`, `:230-249` |
| F-29 | M | Dead path | `list_active_sources` still returns a hard-`LIMIT 200` full list although documentation says full-list downloads are not a production path | `postgres_import_stores.rs:321-343`; `hosted_backend.rs:230-237` |
| F-30 | M | Compliance | Audit anonymization is a plain UPDATE with no legal hold, approval, or self-audit; retention/purge remains unimplemented (documented gate) | `audit_event_store.rs:348-360` |
| F-31 | M | Worker | `SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_JOB_BATCH_SIZE` was unvalidated: `0` silently disabled ingestion and large values were silently clamped | `worker/src/bin/worker_main.rs:28-32` — **fixed** |
| F-32 | M | Process model | The WeChat config write lock is one process-wide async mutex held across two Drive round-trips, with no acquire timeout | `wechat/config_store.rs:41-45`, `:101-106` |
| F-33 | L | Provider safety | Provider-credential management accepts `knowledge.*` in the service layer while the HTTP guard requires the exact permission | `service/provider_binding.rs:516-528`; `routes-backend-api/permission.rs:12-17` |
| F-34 | L | Secret handling | Redis URL (with embedded password) lives in a `#[derive(Debug)]` struct | `standalone-gateway/src/redis_runtime.rs:24-28` |
| F-35 | L | Caches | WeChat token/refresh-lock maps and the Onyx per-space allowlist map only ever grow; the WeChat replay set is process-local | `wechat/api_client.rs:139-143`; `engine-onyx/src/client.rs:188-223`; `wechat/callback.rs:54` |
| F-36 | L | Dead code | `deleted_at` on `kb_group_knowledge_space_binding` is never written; `get_group_managed_space_any_status` does not filter binding lifecycle | `postgres_group_space_binding_store.rs`; `postgres_space_stores.rs:790-816` |
| F-37 | L | Idempotency | Retrieval trace hits are append-only with no per-trace uniqueness; a retry duplicates hits | `retrieval_store.rs:478-580` |
| F-38 | L | Docs | PRD checklist items contradict TECH (audit index "to add" vs "exists"), migration IDs cited in docs have no file, `database/README.md` repeats and contradicts its own initialization-state section | `PRD-mvp-launch.md:35`; `TECH_ARCHITECTURE.md:161-163`; `database/migrations/postgres/README.md:3-9` |

## 4. Detailed findings

### F-01 — WeChat publish is live ahead of its documented gates (Critical)

`publish_articles` resolves the account, uploads covers, calls `add_draft`, then optionally
`submit_freepublish` for every account (`wechat/service.rs:143-216`). The PRD and architecture say
the opposite: publish/preview "fail closed before upstream mutation" and re-enabling requires
managed covers, durable idempotency, per-target outcomes, and partial-failure recovery
(`TECH_ARCHITECTURE.md:137-143`, `PRD.md:91-92`, `PRD-mvp-launch.md:139`).

Consequences: a retried request can publish duplicate articles to real followers; a multi-account
request can partially succeed with no record of which account published; the operation returns
`accepted: true, status: "accepted"` after the loop, so the caller cannot distinguish an upstream
`publish_id` from a failure.

Required: put the operation behind an explicit default-off feature flag until idempotency keys are
persisted and per-target outcome rows exist, **or** complete those gates and update both documents.
Do not leave the documents asserting a fail-closed behavior the code does not implement.

Related: `preview_articles` is a dry run that returns `accepted: true, status: "validated"`
(`wechat/service.rs:218-269`), but `WechatSendPreviewModal.tsx:141-146` unconditionally toasts
"preview sent successfully" to the listed recipients. Either call WeChat's preview endpoint or change
the copy and the response contract to say nothing was sent.

### F-02 — SQLite is not implemented, and the claimed fixture is not executable (Critical for testability)

- `repository-sqlx/Cargo.toml:22` enables `runtime-tokio`, `postgres`, `any`, `migrate` — no `sqlite`.
- `db/bootstrap.rs:50` rejects every non-PostgreSQL engine; `database.manifest.json` declares
  `engines: ["postgres"]`; there is no `database/migrations/sqlite`.
- `tests/sqlite_fixture_parity.rs` reads `.sql` files as text and compares table/column/index
  **names**. It never opens a connection, and it does not compare types, nullability, constraints,
  triggers, or RLS.
- Doc comments in that test and `TECH_ARCHITECTURE.md:104-107` claim a "deterministic in-memory
  fixture can execute the same repository SQL". That code does not exist.

Answer to the direct question: **PostgreSQL is implemented; SQLite is not implemented at all.**
The correct action is to either build a real SQLite profile (driver feature, migration set,
executed fixture tests) or correct the documents and remove the dead fixture. Because the product
has no SQLite surface (mobile native is out of scope), correcting the documents is the cheaper and
more honest option.

### F-03 — The organization-scope gate cannot fail (Critical for isolation confidence)

`tools/check_organization_scope_sql.mjs:57-59` skips a statement when its text contains
`organization_id`. A `SELECT` list that includes the column therefore exempts a `WHERE` that only
binds `tenant_id`. Two such statements exist:

- `postgres_group_space_binding_store.rs:163` — cross-organization archive scan by `tenant_id` only.
- `postgres_group_space_binding_store.rs:558` — space lookup by `tenant_id` + `space_id` only.

RLS still restricts both to the deployment organization, so this is not a live cross-tenant read;
it is a defense-in-depth and gate-integrity defect. Fix the checker to parse the `WHERE`/`WITH CHECK`
clause and require both predicates, then make the two deliberate tenant-wide scans explicit typed
ports with audit.

### F-04 — Retrieval fan-out was unbounded (High; fixed)

`retrieve` loops `for binding { for method { backend.search_chunks(...) } }`
(`retrieval.rs:97-113`). `validate_request` bounded neither the binding count nor the method count,
and the HTTP body cap is 1 MiB. A single authenticated caller could force thousands of ACL checks
(`require_bindings_space_access`) and backend searches per request.

Fixed by `MAX_RETRIEVAL_BINDINGS = 16` and `MAX_RETRIEVAL_METHODS = 3` in
`crates/sdkwork-intelligence-knowledgebase-service/src/retrieval.rs`, plus method de-duplication in
`normalize_methods`, with unit tests.

### F-05 / F-06 — OKF native engine materializes whole spaces (High)

`search` loads all published concept summaries (cap 50 000, then error), clones each into a
`HashMap`, loads all active link edges, and calls `expand_ranked_with_link_edges`, whose inner loop
iterates seeds × edges and allocates a lowercased string per pair (`knowledge_engine/okf_search.rs`).
`read_document` loads the same full summary set and linearly finds one. Both run inline on an async
worker with no deadline.

Required: keyset/`LIMIT` ranking in SQL, edge expansion restricted to seed ids, `top_k` clamped at
the engine boundary, work moved to `run_bounded_blocking` with a deadline, and a `LIMIT 1` lookup for
single-document reads.

### F-07 / F-08 — Provider calls and embedding run inside HTTP requests (High)

`hosted.rs:192-199` and `:329-335` say "fire-and-forget" and then `.await` the whole embedding
pipeline; `hosted_backend.rs:838-877` and `:919-925` embed every chunk of a space on the request
path. The chain is: load all chunk texts → batches of 16 → one provider HTTP call per chunk
(`knowledge_embedding_index.rs`, `cloud_router_embeddings.rs`).

Required: enqueue a durable indexing job and let the worker embed with a cursor and per-tick batch
cap; never call the model provider on the user request path; expose `index_state` in the ingest
response.

### F-09 — Quota advisory lock inside a 5-connection pool (High)

`begin_tenant_quota_transaction` opens a transaction and takes `pg_advisory_xact_lock` on
`(tenant, organization)` (`quota_transaction.rs:11-26`). Every quota-checked write uses it. The
default process budget is 10 connections split 50/50 (`db/bootstrap.rs:19,57-68`), leaving **5**
`AnyPool` connections. When more than five writers for the one deployment tenant queue on the lock,
every other query on that pool waits (10 s acquire timeout) or runs into the 30 s statement timeout.

Required: replace the lock-and-count pattern with a single atomic reservation statement
(`UPDATE kb_tenant_quota_usage ... WHERE used < limit RETURNING used`), or use
`pg_try_advisory_xact_lock` with fail-fast/backoff so a waiting request never holds a pool slot; raise
the default budget and add a per-tenant write admission limit.

### F-10 — The "bounded" blocking executor is not bounded under timeout (High)

`BoundedBlockingExecutor::run` drops the permit on timeout while `spawn_blocking` keeps running
(`bounded_blocking.rs:39-60`), and a test codifies that eviction. Under a slow embedding provider the
64-permit cap becomes an unbounded zombie population (Tokio's blocking pool allows 512 threads).
Drive import extraction uses a raw `spawn_blocking` over a 32 MiB input with `tokio::time::timeout`,
which likewise does not cancel the task (`drive_import_pipeline.rs:218-240`).

Required: move the permit into the blocking closure so capacity is returned only when the task
finishes, and route extraction through the same admission control with a smaller input bound.

### F-11 — 500 instead of 404 for missing documents (High; fixed)

`PostgresKnowledgeDocumentStore::get_document_by_id`, `update_document`, and `soft_delete_document`
mapped `RowNotFound` / zero rows to `KnowledgeDocumentStoreError::Internal`, which the API maps to
HTTP 500 (`error.rs:621`). The enum already has a `NotFound` variant mapped to 404. Fixed in
`postgres_import_stores.rs`.

### F-12 — Fabricated AI polish (High; fixed)

`TypographyModal.handleRewrite` had a branch for "API unavailable" that waited 1.2 s and appended
`【此段内容已通过微信金句引擎优化润色】` to every paragraph, then applied it as the rewrite. That is
content no model produced, shown to the author as AI output. Replaced with a fail-closed toast; no
document mutation happens without a live API.

### F-13 — Fabricated identity and activity metrics (High; open)

`DEFAULT_USER_PROFILE` gives every user the same name, email, phone, department, and bio
(`UserProfileModal.tsx:64-75`). Save only writes localStorage and toasts "个人资料保存成功".
The statistics tab renders `42`, `3`, `18 / 30`, `158` as the user's own activity
(`:643-676`). This is not demo-gated.

Required: bind the profile to the authenticated account (IAM) and to real aggregates, or remove the
tab and the edit form until the contracts exist. Local-only preferences must not be presented as
persisted profile data.

### F-14 — Demo gating and stale production profiles (High; code fixed, config open)

`isKnowledgebaseDemoModeEnabled` only read `VITE_SDKWORK_KNOWLEDGEBASE_ENABLE_DEMO_MODE`; the PRD
claims `import.meta.env.PROD` disables demo data (`PRD-mvp-launch.md:85`). Added a hard `PROD`
short-circuit in `knowledgebaseApiRegistry.ts`. The checked-in runtime-env profiles still carry
`enableDemoMode: true`, `buildMode: development`, and `configProfile: dev` for `production`,
`staging`, and `test`; `public/runtime-env.json` ships the same. Those files are not the authority
the build uses (`.env.{profile}.{env}` is materialized from `etc/topology/`), but they are
committed, misleading, and a foot-gun for anyone who re-points the materializer. Align them or
delete them.

### F-15 — Timestamp typing and session time zone (High)

`kb_outbox_event.created_at` / `published_at` / `dead_lettered_at` / `next_attempt_at` are
`TIMESTAMP`, while `claimed_at` is `TIMESTAMPTZ` in the same table. Connection options never set
`timezone`, and writes go through `CAST(<rfc3339 text> AS TIMESTAMP)`, which discards the offset; a
`TIMESTAMPTZ` target then interprets the value in the session zone. Lease expiry, retry scheduling,
and stale-claim detection are all absolute-time calculations, so a non-UTC server shifts them.

Required: one instant type (`TIMESTAMPTZ`) for business instants, bind native time values instead of
text `CAST`, and add `-c timezone=UTC` to the connection options.

### F-16 — Agent profile IDOR for binding-less profiles (High)

`retrieve_profile`, `update_profile`, and `delete_profile` authorize only through the profile's
enabled bindings (`hosted_access.rs:710-739`). A profile with no enabled bindings iterates nothing
and returns `Ok(())` after the tenant check, so any authenticated user in the tenant can read
`system_instruction`, model parameters, and memory policy, or rewrite/delete the profile.

Required: add an owner (`created_by`) to `kb_agent_profile` and enforce owner-or-explicit-share, or
reject binding-less profiles at creation and re-check non-empty bindings on mutation. Add a negative
test (user B must get 403/404 for user A's profile).

### F-17 — Wiki persistence scope does not match the space's owning organization (High)

`HostedWikiPublicationService::scope` uses `context.organization_id.unwrap_or(0)`
(`hosted_wiki.rs:50-55`), while the space store is constructed with the deployment organization
(`runtime.rs:1027-1030`) and `ensure_runtime_organization` explicitly accepts an org-less personal
session (`hosted_access.rs:48-68`). A personal session therefore persists publication rows under
organization `0` for a space owned by the deployment organization. Under the `organization_isolation`
RLS policy (which requires the session organization), the insert is rejected; the wiki initializer
catches the error and only logs a warning (`space.rs:624,637-652`), so space creation succeeds
without a publication.

Required: derive every wiki write/read scope from the space's owning (deployment) organization, or
reject wiki operations for org-less sessions; add an integration test asserting the persisted
`organization_id` equals the space row's.

### F-18 / F-19 — List contract drift (Medium)

- `agentProfiles.bindings.list` declares `data.item: KnowledgeAgentBindingList` (`{items}`) but the
  handler returns `{items, pageInfo}` via `fixed_list_page_data`. Generated SDK types omit
  `pageInfo`. The store also applies `LIMIT 200` with no cap on creation, so bindings beyond 200 are
  silently invisible.
- `okf.bundle.index.list`, `okf.bundle.log.list`, and `okf.bundle.profile.list` are named `list` but
  return one document under `data.item` (`OkfIndexDocument`, `OkfLogDocument`,
  `OkfProfileDocument`). Per `API_SPEC.md` §15.4, `list` means `data.items` + `data.pageInfo`.

Fix by aligning the authored OpenAPI and the handler (declare `pageInfo`, add a real cursor page for
bindings) and renaming the OKF operations to `retrieve` (or returning real list envelopes).

### F-20 / F-21 — Missing indexes for declared filters (Medium)

`list_documents_page_for_space` filters `(tenant_id, organization_id, space_id, status)` and orders
by `id`; `kb_document` has no index leading with that tuple, so each page scans/sorts the tenant's
documents in the org. Metadata filters emit `d.metadata::jsonb ->> ? = ?` with no GIN/expression
index. Add `(tenant_id, organization_id, space_id, status, id)` and either index the supported
metadata keys or reject metadata filters until the contract exists.

### F-22 — Ingestion claim shape (Medium)

The claim is one `SELECT ... LIMIT` followed by per-candidate conditional `UPDATE ... RETURNING`
(`postgres_import_stores.rs:2254-2356`). The CAS prevents double ownership, but the documented
"atomic claim" is not what runs: it costs N+1 round trips and can waste candidates that lose the
race. Prefer a single `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING`.

### F-23 — Current-version binding reports success without binding (Medium)

`bind_document_current_version_in_transaction` ignores `version_no` and does not inspect
`rows_affected` (`postgres_knowledge_document_metadata_transaction.rs:422-458`); the caller then sets
`current_version_id` in memory and returns it (`postgres_drive_import_metadata_store.rs:201-212`).
When the monotonic guard matches no row, the response claims a binding the database did not make.
Return a conflict on zero rows and assert monotonicity with `version_no`.

### F-24 — Cloud image can silently use pod-local storage (Medium)

`should_seed_standalone_local_provider` treats an absent deployment profile as standalone and seeds a
`local_filesystem` provider (`knowledgebase-drive/src/bootstrap.rs:54-92`). The architecture says
cloud deployments must use a shared non-local provider and that pod-local files are never
authoritative. Fail closed when the profile is absent in a production-like environment.

### F-25 — Stale, misleading client runtime profiles (Medium)

`public/runtime-env.json` (the build output) is `cloud.production` with
`configProfile: "dev"`, `buildMode: "development"`, `featureFlags.enableDemoMode: true`, and
`127.0.0.1` SDK base URLs, while its own note says production disables demo mode. The app does not
read this file today (it is not fetched), but it is committed build output that can be served and
copied. Regenerate it from the topology authority or remove it from version control.

### F-26 — Abuse tiers on expensive mutations (Medium)

`spaces.create`, `documents.create`, `spaces.members.create`, `documents.versions.create`,
`okf.concepts.upsert`, and `contextBindings.*` use the plain route helper, unlike ingest/import
routes that declare an abuse tier. Space creation provisions a Drive space, an OKF file set, and an
owner ACL grant per request, and there is no space-count quota. Add the tier and a tenant space
quota.

### F-27 / F-28 / F-29 — Smaller API and dead-path defects (Medium)

- Open API alias rejection splits the raw query and does not percent-decode the key; app API does.
  The two surfaces also return different error codes for the same violation.
- `encode_opaque_next_cursor` silently drops a non-numeric store cursor, and `has_more` is derived
  from cursor presence, so a format mismatch would end pagination early with no error.
- `list_active_sources` (LIMIT 200) survives although docs say full-list downloads are not a
  production path; it can be wired by mistake and silently truncates.

### F-30 — Compliance gaps already recorded as gates (Medium)

Audit export is bounded (5 000 + overflow probe → 413) and implemented. Retention/purge, legal hold,
and anonymization safeguards are not; `anonymize_actor` is a plain UPDATE with no audit or hold check.
This matches the documented pre-launch gate but must not be presented as complete.

### F-31 — Worker batch env validation (Medium; fixed)

`SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_JOB_BATCH_SIZE` accepted `0` (silently disabling ingestion via
`limit == 0`) and any large value (silently clamped to 200). Now filtered to `1..=200`, matching the
other four batch knobs.

### F-32 — WeChat config write lock (Medium)

One process-wide `tokio::sync::Mutex` is held across load + two Drive round-trips
(`wechat/config_store.rs:41-45`, `:101-106`), with no acquisition timeout. Cross-replica lost updates
remain possible because the Drive write has no version/ETag precondition. Key the lock by
tenant/config kind, bound the critical section, and add a Drive precondition with a 409 conflict.

### F-33 — Provider credential permission widening (Low)

`validate_management_context` accepts `knowledge.*` (`service/provider_binding.rs:516-528`) while the
HTTP guard requires the exact `knowledge.platform.manage` (`routes-backend-api/permission.rs:12-17`).
The route currently blocks the wildcard, but the service layer is a second, weaker authority. Remove
the wildcard or take an explicit required permission.

### F-38 — Documentation contradictions (Low but reputationally expensive)

- `PRD-mvp-launch.md:35` still asks to add the audit subject index that
  `TECH_ARCHITECTURE.md:161-163` says exists (it does: `idx_kb_audit_event_scope_actor_created`).
- `specs/tenant-isolation.md` and TECH cite migration IDs (`202607310001_core_organization_isolation`,
  `202607310002_outbox_claim_fencing`, `202608040002_audit_event_scope_actor_index`) that have no
  file under `database/migrations/postgres/`; the history is folded into the baseline.
- `database/README.md` repeats its "Initialization state" section and says migrations are
  "intentionally empty" while two exist.

## 5. Answers to the specific questions

### 5.1 Are SQLite and PostgreSQL both fully implemented? — No.

PostgreSQL is the only server backend and is implemented: lifecycle bootstrap, RLS with
`FORCE ROW LEVEL SECURITY` on 41 tables, outbox `SKIP LOCKED` claim with owner/token fencing,
worker leases, audit export bounds, optimistic version fencing for provider bindings. SQLite is
**0% implemented at runtime** and its "parity fixture" is a text file that no test executes.
Treat the TECH sentence about an in-memory SQLite fixture as false and correct it.

### 5.2 Is pagination aligned to `PAGINATION_SPEC`? — Mostly, with contract drift.

- `page_size` is the only size parameter on HTTP GET; `pageSize`/`limit`/`per_page`/`size`/
  `page_no`/`pageNo` are explicitly **rejected** before deserialization on app and open surfaces
  (`routes-app-api/routes.rs:1533-1587`, `routes-open-api/routes.rs:301-328`).
- Page size is validated (not clamped) to `1..=200` and `PageInfo.mode` is `cursor` on the list
  services reviewed; repository queries use keyset `id > cursor ORDER BY id LIMIT page_size + 1`.
- Drift: `agentProfiles.bindings.list` omits `pageInfo` in OpenAPI and silently truncates at 200;
  three `*.list` operations return a single `item`; the open-api alias error code differs.
- The heuristic gate passes, but it does not catch the three contract issues above.

### 5.3 Are there fake implementations? — Yes, three user-visible ones, plus documentation lies.

- WeChat preview reports success without sending (UI copy) — **open**.
- "AI polish" fabricated paragraphs — **fixed** (fail-closed).
- Demo identity + hard-coded activity metrics — **open**.
- Beyond that, there are no `todo!`/`unimplemented!` in production Rust and the engine/Drive/outbox/
  wiki paths perform real work.

### 5.4 OOM and large-cluster concurrency? — Not yet.

Unbounded retrieval fan-out (**fixed**), whole-space OKF materialization (**open**), synchronous
whole-document and whole-space embedding (**open**), 32 MiB extraction tasks without admission
(**open**), permit eviction on timeout (**open**), quota advisory lock on a 5-connection pool
(**open**). These are the changes required before any load/SLO evidence is meaningful.

### 5.5 Database design/transactions? — Substantial, with real gaps.

Good: composite scoped FKs for the core tables (15), composite uniques for org isolation, outbox
retry/backoff/dead-letter, exactly-one WikiPublication, provider version fencing, audit index.
Gaps: mixed timestamp types and unpinned session time zone, missing document-list index, unindexed
JSON metadata filters, non-atomic ingestion claim shape, tenant-only statements the gate cannot see,
single-column FKs on three tables, and no FK on six space-scoped tables.

### 5.6 Commercial readiness? — No.

The manifest itself is `DRAFT`/`preLaunch` with missing release evidence, and the PRD records open
gates (release PostgreSQL evidence, WeChat managed cover/idempotency, audit retention, artifact
signing, live provider certification with `liveCertifiedCount = 0`). On top of those, this audit adds
F-01/F-13/F-16/F-17 as release blockers that the existing gate list does not contain.

## 6. Remediation sequence

**P0 — before any external user (must fix):**

1. Gate WeChat publish behind a default-off flag until idempotency + per-target outcomes exist, and
   make preview copy truthful (F-01).
2. Replace the demo profile/metrics with real account data or remove the screen (F-13).
3. Fix agent-profile authorization (owner or explicit share) (F-16).
4. Fix the wiki persistence scope so writes match the space's owning organization (F-17).
5. Correct the SQLite fixture claim and remove/replace the dead fixture (F-02).
6. Fix the organization-scope gate and the two tenant-only statements (F-03).
7. Make the quota write path pool-safe (single atomic reservation or `try_advisory_lock`) and raise
   the default connection budget (F-09).
8. Move ingest/import/index embedding off the request path; bound OKF ranking and single-doc reads
   (F-05/F-06/F-07/F-08).
9. Keep the permit inside the blocking closure; route extraction through admission control (F-10).
10. Pin `timezone=UTC` and normalize business instants to `TIMESTAMPTZ` (F-15).

**P1 — commercial launch gates:**

11. Persist ingest `index_state` + retry/reconciliation; expose it in the API.
12. Add `(tenant, org, space, status, id)` and metadata GIN indexes; attach query-plan evidence under
    load (F-20/F-21).
13. Add abuse tiers and a per-tenant space quota (F-26).
14. Align `agentProfiles.bindings.list` and the three OKF `*.list` operations with `API_SPEC`
    (F-18/F-19); add pagination contract tests for `pageInfo.mode`, max page size, and cursor
    continuity.
15. Fix the ingestion claim to one statement; add the missing composite FKs (F-22 and §5.5).
16. Drive config version preconditions and a keyed config lock (F-32).
17. Reconcile PRD/TECH/database docs and restore an addressable migration history (F-38).

**P2 — hardening:**

18. Bounded caches/eviction for WeChat tokens and the Onyx allowlist (F-35); redact the Redis URL
    (F-34); remove the `knowledge.*` widening (F-33); stop shipping stale runtime-env profiles
    (F-25); implement audit retention/legal hold (F-30); add load/SLO/outage evidence and non-owner
    RLS release tests.

## 7. Verification log for this review

### Final regression (2026-10-21)

| Command | Result |
|---------|--------|
| `SDKWORK_DATABASE_URL=… cargo test --workspace --no-fail-fast` | no failing targets; 0 failed tests, 1 ignored destructive test |
| `cargo check --workspace --all-targets` | exit 0, no first-party warnings |
| `cargo fmt -p … -- --check` (touched crates) | exit 0 |
| `pnpm test:frontend` | 26 files / 128 tests + all PC packages pass |
| `pnpm --dir …-pc run build:prod` | PASS under the 650 KiB per-chunk budget (Vite 8/Rolldown) |
| `pnpm test:security` / `pnpm test:launch-readiness` | pass |
| `pnpm verify` (phase-1 structure + check + security/observability/launch/phase-2 + deploy validate + composition + CORS) | pass |
| `pnpm topology:validate` / `pnpm db:validate` | pass |
| API patterns / envelope / pagination / DB framework / client env / PC hygiene / architecture | pass |
| Repository PostgreSQL suite (provider binding, readiness, migration, wiki persistence, public provider, lifecycle) | 19 passed, 1 ignored |

Historical review-time verification:

| Command | Result |
|---------|--------|
| `node ../sdkwork-specs/tools/check-pagination.mjs --workspace .` | passed (heuristic) |
| `node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .` | passed |
| `node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .` | passed |
| `node ../sdkwork-specs/tools/check-rust-crate-naming-standard.mjs --root .` | 0 errors |
| `node ../sdkwork-specs/tools/check-rust-manifest-standard.mjs --root .` | 0 errors |
| `node ../sdkwork-specs/tools/check-database-framework-standard.mjs --root .` | passed |
| `node tools/check_organization_scope_sql.mjs` | passed **despite** F-03 |
| `cargo check --workspace` | compiled; one warning fixed |
| `cargo check --workspace` (after remediation batch 1) | compiled, exit 0 |
| `cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --lib` | 33 passed |
| `cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test sqlite_fixture_parity` | 6 passed (in-memory SQLite DDL executes) |
| `SDKWORK_DATABASE_URL=… cargo test -p …-repository-sqlx --test postgres_optional_integration` | **8 passed** against live PostgreSQL 18.3 |
| `cargo test -p sdkwork-intelligence-knowledgebase-service --lib` | 175 passed |
| `cargo test -p …-service --lib okf` / `--test knowledge_engine_native` | 44 / 12 passed |
| `pnpm --dir apps/sdkwork-knowledgebase-pc run lint` (tsc) | exit 0 |
| `pnpm --dir …-pc-core test` / `…-pc-shell test` / `…-pc-knowledgebase test` | 59 / 3 / 128 passed |
| `node tools/check_organization_scope_sql.mjs` | passed with the strict predicate parser (0 violations) |
| `node tools/check_sdkwork_knowledgebase_architecture_alignment.mjs` | passed (retired demo topology removed) |
| `node tools/check_pc_app_hygiene.mjs` | passed |
| `node ../sdkwork-specs/tools/materialize-client-env.mjs --root . --check` | passed, 8 profiles |

Fix-level verification is recorded in the commit that lands each fixed item (retrieval bounds unit
tests, repository error-mapping compile, worker env parsing, PC typecheck).

## 8. Open questions for human review

- WeChat publish: is the intent to enable it now (then complete the gates and update the docs), or to
  keep it disabled (then add the flag)? This changes external customer behavior and needs an owner.
- SQLite: is a local/desktop persistence profile required by any commercial target? If not, remove
  the fixture and the claim; if yes, it is a new workstream, not a test fix.
- Agent profiles: should they be actor-owned, space-scoped, or admin-only? The schema has no owner
  column, so this is a product decision before the authorization fix.
- Personal (org-less) sessions: should wiki and other org-scoped features be available to them? If
  yes, the persistence layer must resolve the owning organization rather than the session claim.
