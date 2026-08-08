# Knowledgebase Changelog

## Unreleased

### Added

- Added `sdkwork-knowledgebase-provider-secret-adapter` and a typed Provider credential access
  context. Development/test sources are restricted to the Knowledgebase and implementation
  environment namespace or canonical files under a per-Provider root; staging/production use managed `secret://`
  references only. Executable tests cover unrelated and cross-Provider locator rejection, root and
  symlink escape, production fail-closed policy, complete context propagation, bounded results,
  one total time budget, a concurrency bulkhead that contains timed-out blocking calls, intermediate
  plaintext cleanup, sanitized errors, and immediate rotation/revocation without a cache.
- Added versioned Provider load/SLO and outage-recovery evidence schemas, templates, and a shared
  operational evidence policy. Live certification now recomputes results from digest-bound raw
  request samples and outage timelines and rejects policy weakening, unsafe fields, oversized or
  escaped artifacts, future/mismatched dates, threshold violations, retry storms, secret leaks, and
  cross-tenant violations. Quality, operational, and live evidence now reuse one bounded artifact
  reader; production quality datasets are also capped at 5,000 scored and 500 rejection queries.
  These controls do not create production evidence; all live Provider certifications remain pending.
- Added the `ADR-20260720` Provider Binding prelaunch readiness read model and one-shot Worker
  command. It is read-only, tenant/organization scoped, opaque-keyset paginated, secret-free, and
  does not infer Provider Bindings from historic source order. Operator procedure:
  `docs/runbooks/RUNBOOK-provider-binding-readiness.md`.
- Added shared PostgreSQL RLS scope helpers (`postgres_scope.rs`) as the single authority for
  tenant/organization session-key injection, plus SQLite/PG schema-parity fixture migrations and
  parity tests (`sqlite_fixture_parity.rs`) covering organization isolation columns, outbox claim
  fencing, retry backoff, and audit scope/actor indexes.
- Added opaque base64url cursor helpers (`encode_opaque_u64_cursor`/`parse_opaque_u64_cursor`) to
  both app-api and backend-api route crates; chunked document-version reads now use keyset loops
  instead of offset pagination.

### Changed

- Aligned every list endpoint with PAGINATION_SPEC v1.3: cursor-mode envelopes, default page size
  20 with a hard maximum of 200, and `40003` rejection (never clamping) for oversized page sizes.
  Facade/in-process pagination defaults were replaced by explicit `unsupported_operation` stubs.
- Renamed repeated operationIds to verb-form semantics (`documents.versions.create`,
  `agentProfiles.bindings.create`, `agentProfiles.retrievalPreview.create`,
  `agentProfiles.chat.create`, `spaces.contextBindings.create`, `spaces.members.create`) and
  normalized `account_id` to `accountId` across route manifests and OpenAPI sources; OpenAPI
  permission annotations are fully materialized (72/72).
- Retrieval trace logging now redacts query/actor payloads through a field-whitelist request
  record; WeChat API client caches tokens (7,000 s TTL) with bounded retry/backoff for retryable
  error codes.
- Demo mode requires an explicit environment flag everywhere; the PC app enables `strict`
  TypeScript and removes ~200 unused declarations (deletion-based cleanup), the tab window cache
  is bounded (50 tabs/KB, 20 cached KBs), tree browsing gained load-more pagination, and all
  timers are cancellation-safe.
- Dependency security alignment: `jspdf` upgraded to 4.2.1+, `react-router-dom` to 7.18.2+, and
  workspace overrides pin `dompurify`, `linkify-it`, `postcss`, `nanoid`, and `react-router` to
  patched releases; `pnpm audit --prod` reports zero known vulnerabilities.

- Removed raw Provider credential parsing from the app route crate. `KnowledgebaseRuntime` now
  injects the resolver, exposes an explicit managed-resolver construction path, and rejects default
  staging/production construction when no approved managed resolver is supplied. A concrete
  production Vault/KMS backend and live operational drills remain release gates.
- Reduced the readiness report to stable space identifiers and non-active Binding counts; space
  names, source metadata, remote-resource identifiers, and credential material are excluded.
- Replaced the Provider migration cutover store's positional parameter list with a typed command
  and private SQL transition objects. Atomic cutover, retained predecessor, lease fencing, and
  rollback behavior remain covered by the repository migration tests; no Clippy suppression is
  used for these transition paths.
- Replaced static-message `format!` calls in the AnythingLLM, Flowise, Haystack, Open WebUI, and
  Qdrant adapters. All ten executable Provider crates pass strict all-target Clippy with warnings
  denied.
