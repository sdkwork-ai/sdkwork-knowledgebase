# SDKWork Knowledgebase
repository-kind: application

Rust backend and PC web/desktop client for SDKWork Knowledgebase.

This repository follows SDKWork App Standard v3 and the canonical specs in `../sdkwork-specs/`.

## Workspace paths

Repository root declares one monorepo manifest per `APP_COMPOSITION_SPEC.md` (single root graph; no nested app-level workspace files).

```text
apis/                                      Synchronized OpenAPI review copies (materialized from sdks/).
apps/sdkwork-knowledgebase-pc/             PC React + optional Tauri desktop app surface.
crates/
  sdkwork-knowledgebase-contract           Public DTOs, enums, IDs, operation IDs, and OKF bundle contracts.
  sdkwork-knowledgebase-agent-provider     Thin adapter from Knowledgebase retrieval contracts to sdkwork-agent-kernel KnowledgeProvider.
  sdkwork-intelligence-knowledgebase-object-key-service
                                             Object key planning helpers.
  sdkwork-intelligence-knowledgebase-service
                                             Business services, ports, and pure use cases.
  sdkwork-intelligence-knowledgebase-repository-sqlx
                                             SQL migration registry and SQLite/Postgres SQLx repositories via sdkwork-database.
  sdkwork-routes-knowledgebase-app-api     App HTTP route boundary for generated App SDK operations.
  sdkwork-routes-knowledgebase-backend-api Backend HTTP route boundary for generated Backend SDK operations.
  sdkwork-routes-knowledgebase-open-api    Open HTTP route boundary for generated Open SDK operations.
  sdkwork-knowledgebase-drive              Adapter to sdkwork-drive storage contracts.
  sdkwork-knowledgebase-memory             Adapter from Knowledgebase context packs to sdkwork-memory SPI.
  sdkwork-knowledgebase-test-support       Test fakes and fixtures.
  sdkwork-api-knowledgebase-standalone-gateway         Runnable app/backend/open HTTP API binaries.
sdks/
  sdkwork-knowledgebase-app-sdk            App SDK family, app-api OpenAPI authority, generated transport, and composed TypeScript facade (`createKnowledgebaseAppClient`).
  sdkwork-knowledgebase-backend-sdk        Backend SDK family, backend-api OpenAPI authority, and generated TypeScript SDK.
  sdkwork-knowledgebase-sdk                Open SDK family and generated TypeScript SDK.
jobs/ plugins/ examples/ etc/ deployments/ scripts/ tests/
                                             Standard root capability directories for jobs, plugins, deployment, and verification.
```

This repository root is the primary SDKWork Knowledgebase application root and owns `sdkwork.app.config.json`. The PC app surface lives under `apps/sdkwork-knowledgebase-pc/`.

This workspace follows the standard project root directory dictionary from `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`.

### apis/ vs sdks/

`sdks/*/openapi/` is the **generation source of truth** for OpenAPI contracts and SDK families. `apis/` holds synchronized review copies materialized by `tools/materialize-apis-authority.mjs` (`sdks/` → `apis/`). Edit contracts in `sdks/`, then run `pnpm api:materialize` before commit.

Governance metadata is applied by:

- `tools/apply-knowledgebase-openapi-auth-mode.mjs`
- `tools/apply-knowledgebase-openapi-permissions.mjs`
- `sdks/standardize-knowledgebase-sdk-family.mjs`

### plugins/ vs .sdkwork/plugins/

`plugins/` stores application/runtime plugin source packages. `.sdkwork/plugins/` stores repository/application agent plugin workspaces. They are distinct directories with different purposes.

### etc/ vs runtime config

`etc/` is the only source-controlled authority for safe deployment profiles, gateway templates, examples, and non-secret defaults. Runtime user/private config is governed by `RUNTIME_DIRECTORY_SPEC.md` and must not be committed. No alternate source-configuration entrypoint is supported.
## Storage Rule

`sdkwork-drive` is the only lower-level file/object storage boundary.

Business logic must not write source files, parsed artifacts, OKF bundle Markdown, schema files, `okf/index.md`, `okf/log.md`, local mirror packages, or delta packages through direct filesystem, S3, OSS, MinIO, Azure Blob, or GCS SDKs.

Business logic depends on `KnowledgeDriveStorage`. Only `crates/sdkwork-knowledgebase-drive` depends on `sdkwork-drive-storage-contract`.

## Memory Rule

`sdkwork-memory` is the external memory context boundary.

Context pack assembly depends on the `KnowledgeMemoryContextProvider` port. Only `crates/sdkwork-knowledgebase-memory` adapts that port to `sdkwork-memory-spi`; retrieval and API crates must not call Memory HTTP APIs or copy Memory SDK DTOs. Memory context is returned as `memoryFragments` and remains separate from knowledge `fragments`, so Memory entries are not treated as knowledge chunks or citations.

## OKF Bundle Standard Files

Each knowledge space initializes:

```text
okf/schema/AGENTS.md
okf/schema/okf_profile.yaml
okf/index.md
okf/log.md
```

Local mirror contracts expose:

```text
schema/AGENTS.md
schema/okf_profile.yaml
raw/**
**/*.md
```

## Verification

Run:

```powershell
pnpm check
pnpm check:app-composition
pnpm verify
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
pnpm api:materialize:check
pnpm sdk:generate:check
pnpm test
pnpm test:launch-readiness
pnpm lint
```

`pnpm check` includes architecture alignment, native app composition (`verify-repo.mjs` + `component.spec.json#contracts.sdkDependencies`), PC app hygiene (including capability-package SDK import boundaries), utils integration, and SDKWork script/workflow standards.

`pnpm verify` includes Phase 1 contract checks, security/observability alignment, launch-readiness gates, and cloud gateway validation.

Current commercialization status: prelaunch-gated. The app manifest remains a `DRAFT` with `publish.preLaunch=true`, `release.defaultChannel=DEV`, disabled prelaunch packages, and disabled placeholder media projection. Production/commercial publication still requires the unresolved release evidence listed in [docs/product/prd/PRD-mvp-launch.md](docs/product/prd/PRD-mvp-launch.md), including real product media assets, artifact checksum/signature/SBOM/provenance evidence, release-environment PostgreSQL validation, rollout/rollback evidence, and live smoke records.

Launch runbook: [deployments/runbooks/production-launch.md](deployments/runbooks/production-launch.md)

Acceptance criteria: [docs/product/prd/PRD-mvp-launch.md](docs/product/prd/PRD-mvp-launch.md)

## Runtime

Default `pnpm dev` uses the topology profile `standalone.development`:

- one application ingress at `http://127.0.0.1:18081`
- browser dev server at `http://127.0.0.1:5184` with same-origin API proxying
- IAM app-api auth/oauth routes embedded through `sdkwork-iam-gateway-assembly`
- platform gateway `http://127.0.0.1:3900` is **not** required for standalone dev

| Process | Default listen | Surface |
|--------|----------------|---------|
| `sdkwork-api-knowledgebase-standalone-gateway` | `127.0.0.1:18081` | unified app/backend/open + embedded IAM app-api |
| `sdkwork-knowledgebase-worker` | health `127.0.0.1:18085` | background worker |

Cloud profiles use `etc/topology/cloud.development.env`; cloud development consumes deployed API surfaces and does not start a local cloud gateway.

Run from the repository root:

```powershell
pnpm dev
# cloud development example:
pnpm dev:browser:cloud
```

Common environment variables:

- `SDKWORK_DATABASE_URL` (PostgreSQL in the public development and deployment profiles)
- `SDKWORK_DATABASE_MAX_CONNECTIONS` (combined process budget shared by all embedded database consumers)
- `SDKWORK_DATABASE_TEMPORARY_ANY_POOL_EXCEPTION=true` and
  `SDKWORK_DATABASE_TEMPORARY_DRIVER_POOL_COUNT=1` (tracked pre-release compatibility contract)
- `SDKWORK_KNOWLEDGEBASE_TENANT_ID` (must match IAM login tenant; development profiles use `100001` from `sdkwork.app.config.json`)
- `SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND` / `SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL`
- `SDKWORK_KNOWLEDGEBASE_DEV_ALLOWED_ORIGINS` (dev CORS allowlist for browser origins such as `http://127.0.0.1:5184`)
- `SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT` (standalone local Drive provider only; default `data/drive-objects`)
- `SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_PROVIDER_ID` (required in cloud; identifies an active,
  non-local Drive provider whose credential remains a Drive-owned reference)

Runtime authentication is owned by `sdkwork-iam` through the shared web framework,
`sdkwork-iam-web-adapter`, and the embedded IAM app API assembly in standalone mode.
Local and production deployments must provide standard IAM-backed request context instead of product-local auth bypass middleware.

## Runtime ID Configuration

All runtime inserts into `kb_*` tables bind explicit service-generated Snowflake IDs. The database must not generate knowledgebase primary keys through SQLite rowid autogeneration, `AUTOINCREMENT`, PostgreSQL `SERIAL`/`BIGSERIAL`, identity columns, or ad hoc repository sequence calls.

`SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID` configures the 10-bit Snowflake node id:

```powershell
$env:SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID = "42"
```

Valid values are `0` through `1023`. Local and test runs may omit the variable and use node id `0`. Production multi-instance deployments must assign a unique node id per process or pod; invalid configured values fail closed during generator initialization.

## Implemented Service Slices

- Knowledge space creation initializes OKF bundle standard files through the drive storage port.
- OKF bundle rendering covers `AGENTS.md`, `okf_profile.yaml`, `okf/index.md`, and `okf/log.md`.
- Ingestion jobs support idempotent creation and basic state transitions.
- API Markdown payload ingest writes `inbox/api/{ingest_id}/payload.md` through `KnowledgeDriveStorage` and rejects empty payloads or unsafe idempotency keys.
- Drive object import verifies the existing sdkwork-drive object with `head_object`, persists a stable `KnowledgeDriveObjectRef`, creates source/document/version metadata through service ports, and creates an idempotent ingest job.
- Source, document, document version, ingest request, and drive import DTOs are available in the contract crate.
- RAG retrieval, context pack, citation, retrieval trace, knowledge-agent profile, and knowledge-agent binding DTOs are available in the contract crate.
- Context packs can include bounded `sdkwork-memory` context through an injected Memory provider and keep Memory fragments separate from retrieved knowledge chunks.
- SQL migration skeletons include source, document, document version, ingestion job, and ingestion job item tables (PostgreSQL baseline and SQLite fixture).
- SQL migrations include chunk, index, embedding, retrieval profile, retrieval trace, retrieval hit, knowledge-agent profile, and knowledge-agent knowledge binding tables (PostgreSQL baseline and SQLite fixture).
- PostgreSQL SQLx repositories persist the drive-import metadata chain: source, document, document version, stable drive object ref, and ingestion job rows. `create_or_get` paths use `kb_*` unique indexes plus insert-first conflict handling so concurrent identical imports reuse existing metadata instead of creating duplicates.
- PostgreSQL SQLx repositories persist knowledge spaces and OKF bundle file entries, so space creation can initialize `okf/schema/AGENTS.md`, `okf/schema/okf_profile.yaml`, `okf/index.md`, and `okf/log.md` through the drive port and then mark the space as `okf_bundle_initialized`.
- The SQLite schema fixture (`tests/fixtures/database/sqlite/`) is a deterministic in-memory test schema kept in column-level parity with the PostgreSQL folded baseline; server runtimes are PostgreSQL-only (`database.manifest.json` engines: postgres).
- OKF concept revision numbers and `okf/log.md` sequence numbers are reserved with database-backed counters to avoid `MAX + 1` races under concurrent writes.
- Local mirror snapshot and delta manifest services create OKF-compatible export artifacts, compute SHA-256 checksums, reject unsafe path segments, and persist those manifests only through `KnowledgeDriveStorage`.
- App and Backend OpenAPI authority files use SDKWork dotted operation IDs, including `okf.bundle.index.list`, `okf.bundle.log.list`, `driveImports.create`, `documents.versions.versions`, and `sources.create`.
- Generated App and Backend TypeScript SDKs are produced with the canonical `sdkwork-sdk-generator` using the SDKWork v3 standard profile.
- App and Backend SDK families declare Appbase, Drive, and Memory dependency SDKs; dependency-owned Appbase, Drive, and Memory APIs are not generated into knowledgebase transports.
- App and Backend Rust API crates mount every generated OpenAPI operation path. The hosted SQLx runtime (`KnowledgebaseRuntime`) wires all **109** HTTP operations (69 app + 32 backend + 8 open) to concrete service implementations; test doubles are confined to test modules and are never mounted by a deployable runtime.
- The agent provider crate exposes `provider.knowledge.sdkwork-knowledgebase` as a typed `sdkwork-agent-kernel::KnowledgeProvider` adapter backed by an injected retrieval client.

## SDKWork Documentation Contract

Domain: intelligence
Capability: knowledgebase
Package type: rust-crate
Status: standard

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts.publicExports`.

### Required SDK Surface

- None declared in `specs/component.spec.json`.

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### SaaS/Private/Local Behavior

This module follows the canonical standards linked from `specs/component.spec.json`, including deployment and runtime configuration rules where applicable.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Extension Points

Extension points are limited to declared public exports, runtime entrypoints, SDK clients, events, and config keys.

### Verification

- `cargo test`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.

## Documentation Canon

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Application Roots

- [apps directory index](apps/README.md)
