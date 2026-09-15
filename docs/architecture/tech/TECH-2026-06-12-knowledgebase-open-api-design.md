> Owner: SDKWork maintainers

## Context

`sdkwork-knowledgebase` currently owns two SDKWork HTTP surfaces:

- App API: `crates/sdkwork-routes-knowledgebase-app-api`, `sdks/sdkwork-knowledgebase-app-sdk`, `/app/v3/api/knowledge`
- Backend API: `crates/sdkwork-routes-knowledgebase-backend-api`, `sdks/sdkwork-knowledgebase-backend-sdk`, `/backend/v3/api/knowledge`

There is no public open-api route crate, open SDK family, normalized open-api route manifest, or `sdkwork-api-cloud-gateway` surface for Knowledgebase.

Human approval was given on 2026-06-12 to implement the recommended public Knowledgebase open-api contract with prefix `/knowledge/v3/api`.

## Standards

- `../sdkwork-specs/SOUL.md`
- `../sdkwork-specs/API_SPEC.md`
- `../sdkwork-specs/SDK_SPEC.md`
- `../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md`
- `../sdkwork-specs/WEB_BACKEND_SPEC.md`
- `../sdkwork-specs/CODE_STYLE_SPEC.md`
- `../sdkwork-specs/NAMING_SPEC.md`
- `../sdkwork-specs/RUST_CODE_SPEC.md`
- `../sdkwork-specs/CONFIG_SPEC.md`
- `../sdkwork-specs/SECURITY_SPEC.md`
- `../sdkwork-specs/TEST_SPEC.md`

## Goals

- Add a distinct Knowledgebase open-api surface for external integrations.
- Keep public API behavior separate from app-api and backend-api.
- Generate the open SDK family from owner-only OpenAPI metadata.
- Integrate the new public surface into `sdkwork-api-cloud-gateway`.
- Avoid technical debt by keeping route crates, manifests, SDK metadata, and gateway config aligned.

## Non-Goals

- Do not expose login, registration, refresh, logout, current-session, or IAM session flows.
- Do not expose backend management operations such as provider health, source administration, wiki candidate approval, index rebuild, or trace listing.
- Do not duplicate Drive-owned upload/session APIs.
- Do not hand-edit generated SDK transport output under `generated/server-openapi`.
- Do not add database migrations or persistence schema changes.

## Public Prefix And Authority

The approved public prefix is:

```text
/knowledge/v3/api
```

The open-api surface uses these SDKWork identities:

| Artifact | Value |
| --- | --- |
| Route crate | `sdkwork-routes-knowledgebase-open-api` |
| Route crate path | `crates/sdkwork-routes-knowledgebase-open-api` |
| API authority | `sdkwork-knowledgebase.open` |
| SDK family | `sdkwork-knowledgebase-sdk` |
| TypeScript package | `@sdkwork/knowledgebase-sdk` |
| Gateway service id | `sdkwork-knowledgebase-open-api` |
| Gateway base URL key | `SDKWORK_KNOWLEDGEBASE_OPEN_API_BASE_URL` |

The authority string follows the existing Knowledgebase app/backend convention:

- `sdkwork-knowledgebase.app`
- `sdkwork-knowledgebase.backend`
- `sdkwork-knowledgebase.open`

## Public API Scope

The first public surface exposes common external integration flows only.

| Method | Path | Operation ID | Purpose |
| --- | --- | --- | --- |
| `POST` | `/knowledge/v3/api/retrievals` | `retrievals.create` | Run a knowledge retrieval request for external RAG/search consumers. |
| `GET` | `/knowledge/v3/api/retrievals/{retrievalId}` | `retrievals.retrieve` | Retrieve a completed retrieval result by server id. |
| `POST` | `/knowledge/v3/api/context_packs` | `contextPacks.create` | Build a context pack for external LLM/RAG consumers. |
| `POST` | `/knowledge/v3/api/ingests` | `ingests.create` | Submit a bounded external ingestion task. |
| `GET` | `/knowledge/v3/api/ingests/{ingestId}` | `ingests.retrieve` | Read ingestion task status. |
| `GET` | `/knowledge/v3/api/documents` | `documents.list` | List externally visible knowledge documents. |
| `GET` | `/knowledge/v3/api/documents/{documentId}` | `documents.retrieve` | Read an externally visible knowledge document. |
| `GET` | `/knowledge/v3/api/spaces/{spaceId}/browser` | `spaces.browser.list` | Browse a bounded knowledge browser view; OKF `view=files` lists original files under `sources/raw`, while `view=okf_bundle` lists generated bundle files under `okf`. |

This scope intentionally reuses app-api business DTOs where the existing contracts already model the external use case. It does not expose create/update/delete document administration or backend operator controls.

Browser list responses use `KnowledgeBrowserListData` (`items`, `pageInfo`, `spaceId`, `driveSpaceId`, resolved `parentId`, `view`, and `pageSize`). Clients must use the resolved `parentId` for root operations and must not infer Drive folder ids from logical paths.

## Authentication And Request Context

All initial open-api operations are protected with API key auth:

- OpenAPI security scheme: `ApiKey`
- Operation extension: `x-sdkwork-auth-mode: api-key`
- Route manifest auth mode: `api-key`

Handlers must consume typed context when the runtime provides it. They must not parse raw `X-API-Key`, `Authorization`, `Access-Token`, tenant, organization, user, permission, or request id headers. The current route crate test harness may use fake services, but production runtime integration must rely on the standard framework request-context chain.

## Rust Route Design

Add `crates/sdkwork-routes-knowledgebase-open-api` following the SDKWork route crate shape:

```text
crates/sdkwork-routes-knowledgebase-open-api/
  Cargo.toml
  README.md
  specs/
    README.md
    component.spec.json
  src/
    lib.rs
    manifest.rs
    paths.rs
    routes.rs
    handlers.rs
    error.rs
    ports.rs
    adapters.rs
  tests/
    open_api_routes.rs
```

The crate delegates to service traits and does not depend on SQLx repository crates or generated SDKs for the same authority. It may reuse focused traits or adapters from the existing app route crate only when doing so does not couple public API semantics to app-session behavior.

`src/lib.rs` remains limited to module declarations, re-exports, and lightweight docs/wiring.

## SDK Workspace Design

Add the open SDK family:

```text
sdks/
  _route-manifests/
    open-api/
      sdkwork-routes-knowledgebase-open-api.route-manifest.json
  sdkwork-knowledgebase-sdk/
    README.md
    sdk-manifest.json
    sdk-manifest.json
    specs/
      README.md
      component.spec.json
    openapi/
      knowledgebase-open-api.openapi.json
    sdkwork-knowledgebase-sdk-typescript/
      generated/server-openapi/
```

The materialization script `sdks/standardize-knowledgebase-sdk-family.mjs` becomes the single owner for SDK family metadata:

- Include the open family beside app and backend families.
- Write `sdk-manifest.json`, `sdk-manifest.json`, and `specs/component.spec.json`.
- Ensure `sdkDependencies` is explicit and `[]` for the open family unless a dependency SDK is required.
- Set owner metadata on the OpenAPI document and every operation.
- Keep generated transport output free of ownership overlays.

Generated TypeScript output must be produced through the canonical SDKWork generator if the local generator is available. If the generator is unavailable, the family metadata and OpenAPI authority still must be valid, and verification must report the generator gap explicitly instead of editing generated output manually.

## Gateway Design

Update `<workspace-root>/sdkwork-api-cloud-gateway` so the public surface is routable in split mode:

- Add service id constant `KNOWLEDGEBASE_OPEN_API_SERVICE_ID`.
- Add app manifest env var `SDKWORK_KNOWLEDGEBASE_OPEN_API_BASE_URL`.
- Add dependency surface for prefix `/knowledge/v3/api`.
- Add dependency metadata:
  - workspace `sdkwork-knowledgebase`
  - sdkFamily `sdkwork-knowledgebase-sdk`
  - apiAuthority `sdkwork-knowledgebase.open`
  - runtimeModes `["split"]`
  - coverage `knowledgebase-open-api-upstream-routes`
- Add config/runtime tests proving `/knowledge/v3/api/...` routes to `sdkwork-knowledgebase-open-api`.

The gateway remains a proxy boundary for this dependency surface. It must not implement Knowledgebase business handlers.

## Error Handling

All generated operations use RFC 9457 compatible problem details:

- `application/problem+json`
- stable error code/title values
- no stack traces, SQL, raw provider errors, tokens, API keys, or internal hostnames

Route handlers map service errors through the existing `ApiProblem` style used by app/backend route crates.

## Verification Design

Narrow verification:

- `cargo test -p sdkwork-routes-knowledgebase-open-api`
- `node sdks/standardize-knowledgebase-sdk-family.mjs --check`
- `node sdks/test/verify-sdk-ownership-boundaries.test.mjs`
- `powershell -ExecutionPolicy Bypass -File tools/verify_openapi_operation_ids.ps1` when present

Repository verification:

- `powershell -ExecutionPolicy Bypass -File tools/verify_sdkwork_structure.ps1`
- `powershell -ExecutionPolicy Bypass -File tools/verify_phase1.ps1`
- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --workspace --tests -- -D warnings`

Gateway verification:

- `cargo test -p sdkwork-api-cloud-gateway-config`
- `cargo test -p sdkwork-api-cloud-gateway-runtime`
- `cargo fmt --all -- --check`
- `cargo test --workspace`

## Risks And Controls

- Public prefix drift: route crate, route manifest, OpenAPI, SDK family, and gateway tests all assert `/knowledge/v3/api`.
- Auth mode drift: OpenAPI and route manifest declare `api-key`; tests scan that no app/backend token mode is used for the open surface.
- Backend capability leakage: initial route list excludes backend-only operations.
- Generated-output debt: generated transport is regenerated through `sdkgen` only; no hand edits.
- Gateway-only illusion: the gateway adds only split upstream routing; Knowledgebase remains the business API owner.

## Rollback

Before release, rollback is a normal git revert of the open-api route crate, SDK family, route manifest, standardization script changes, and gateway integration. Because this is a new public surface, no existing public consumer compatibility window is required until the API is published.
