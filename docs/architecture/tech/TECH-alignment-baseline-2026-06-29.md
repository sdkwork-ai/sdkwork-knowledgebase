# SDKWork Knowledgebase Standards Alignment Baseline

Status: active  
Owner: SDKWork maintainers  
Updated: 2026-07-04

## Purpose

Pre-launch alignment baseline against `../sdkwork-specs/`. Native composition authority (ADR-20260629) replaces the retired parallel `dependency.composition.json` model.

## Framework Integration

| Framework | Status | Evidence |
|-----------|--------|----------|
| sdkwork-specs | Aligned | `AGENTS.md`, `pnpm check`, `verify-repo.mjs` |
| sdkwork-web-framework | Aligned | Route crates + `web_bootstrap.rs` + envelope mapping |
| sdkwork-database | Aligned | `database/` lifecycle + `pnpm db:*` |
| sdkwork-utils | Aligned | `sdkwork-utils-rust` / `@sdkwork/utils` + strict `check:utils-integration` (no `.trim()` blank bypass) |
| sdkwork-drive | Aligned | `sdkwork-knowledgebase-drive` + `@sdkwork/drive-app-sdk` PC uploads |
| sdkwork-discovery | Deferred | HTTP-only; enable when RPC services ship |

## Native Composition (APP_COMPOSITION_SPEC)

| Authority | Location |
|-----------|----------|
| Workspace dependency graph | Repository root `pnpm-workspace.yaml` (no nested app workspace) |
| Frontend SDK inventory | `apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-core/specs/component.spec.json#contracts.sdkDependencies` (app + platform workspaces with `surface` / `credentialMode`) |
| Resolved composition output | `generated/composition.resolved.json` via `node ../sdkwork-specs/tools/resolve-composition.mjs --root .` |
| Runtime SDK base URLs | `sdkwork-knowledgebase-pc-core/src/composition/dependency-runtime.ts` |
| PC app config workspace pointer | `apps/sdkwork-knowledgebase-pc/sdkwork.app.config.json#packages.workspace` → `../../pnpm-workspace.yaml` |
| Core public exports | `sdkwork-knowledgebase-pc-core/package.json#exports` (`.`, `./sdk`, `./modules`, `./host`, `./session`, `./composition`) |
| SDK contract types (capability packages) | `sdkwork-knowledgebase-pc-core/src/sdk/sdkContractTypes.ts` |

Gate: `pnpm check:app-composition` → `node ../sdkwork-specs/tools/verify-repo.mjs --root .`

## Verification Commands

```powershell
pnpm check
pnpm check:app-composition
node ../sdkwork-specs/tools/resolve-composition.mjs --root .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
pnpm api:materialize:check
pnpm sdk:generate:check
pnpm db:validate
pnpm verify
pnpm test
```

## Pre-Launch Policy

- Root `README.md` declares `repository-kind: application` per `SDKWORK_WORKSPACE_SPEC.md`.
- Blank/trim helpers must import `@sdkwork/utils` directly (no `pc-commons/stringUtils` re-export shim).
- No `specs/dependency.composition.json` or `contracts.dependencyComposition` pointers.
- No raw HTTP in product UI services; use generated app SDK or composed facades.
- No persistent file bytes outside `sdkwork-drive`.
- No local filesystem bundle discovery in production Rust libraries (test fixtures under `tests/support/` only).
- Demo/offline UX is development-only via `shouldUseKnowledgebaseDemoFallback()`; production builds fail closed.

## Verification Status (2026-07-04)

Re-run before release cutover:

| Gate | Result |
|------|--------|
| `pnpm check` | required |
| `pnpm verify` | required |
| `verify-repo.mjs` | required |
| API envelope checker | required |
| Phase 1 launch readiness | required |
| Phase 2 commercial readiness | required |
| Upload session space ACL (`require_space_access`) | enforced |
| Space ACL fail-closed without `drive_space_id` | enforced |
| PC WeChat editor HTML sanitization | enforced |
| Production demo/synthetic media gating | enforced (`shouldUseKnowledgebaseDemoFallback` across WeChat, search, asset library, music player) |
| Tenant-scoped dynamic rate limit policy (`framework_rate_limit_policy`) | wired on app/backend/open HTTP surfaces |
| Backend `tenants.current.list` implementation | wired in `HostedBackendApi` |
| PC admin console (`/admin`) | tenant status, spaces, members, sources, indexes, retrieval traces, provider health via backend SDK |
| Backend `spaces.list` / `spaces.members.list` | admin operator APIs on `/backend/v3/api/knowledge/spaces` |
| Web policy bootstrap | idempotent `framework_rate_limit_policy` + `framework_tenant_runtime_profile` seeds on web store connect |
