> Owner: SDKWork maintainers

# SDKWork Knowledgebase Backend Design

Date: 2026-06-01 (revised 2026-06-29)  
Status: **active reference**  
Owner: sdkwork-knowledgebase

## Document authority

| Topic | Canonical document |
| --- | --- |
| OKF knowledge bundle (Drive layout, concepts, export, agent provider) | [TECH-2026-06-19-okf-knowledge-bundle-design.md](TECH-2026-06-19-okf-knowledge-bundle-design.md) |
| OKF operator summary | [TECH-okf-knowledge-bundle.md](TECH-okf-knowledge-bundle.md) |
| Local OKF contract | `specs/okf-knowledge-bundle.spec.json` |
| System topology and surfaces | [TECH_ARCHITECTURE.md](TECH_ARCHITECTURE.md) |
| Open API | [TECH-2026-06-12-knowledgebase-open-api-design.md](TECH-2026-06-12-knowledgebase-open-api-design.md) |
| Agent RAG | [TECH-2026-06-09-knowledgebase-agent-rag-design.md](TECH-2026-06-09-knowledgebase-agent-rag-design.md) |

This document defines backend platform decisions: storage boundaries, crate layout, persistence rules, HTTP/SDK contracts, security, and verification. **OKF bundle behavior is not duplicated here** — implement against the OKF design and local spec.

---

## 1. Purpose

Build a Rust backend knowledgebase application under the `sdkwork-knowledgebase` repository root. The application provides reusable knowledge services as standalone app/backend/open APIs and as dependency components for other SDKWork applications.

---

## 2. Hard decisions

### 2.1 Storage is drive-first

`sdkwork-drive` is the only approved lower-level file storage capability.

The knowledgebase service must not implement its own filesystem, S3, OSS, MinIO, Azure Blob, GCS, or local filesystem business storage paths. Original documents, parsed artifacts, OCR outputs, page images, thumbnails, export archives, parse manifests, OKF bundle Markdown, schema files, `index.md`, `log.md`, local mirror packages, delta packages, and runtime bundles must be written through `sdkwork-drive`.

The knowledgebase service owns semantic metadata and indexes only:

- knowledge spaces
- collections and document metadata
- document versions
- stable drive object references
- parse and indexing state
- chunks, embeddings, and vector index metadata
- retrieval profiles, traces, and citations
- OKF concept metadata (`kb_okf_*`)
- ACL, audit, retention, and outbox records

The knowledgebase service stores stable drive object references, not file bytes and not presigned URLs.

### 2.2 Adapter-first architecture

- SQL is the system of record for metadata, permissions, tasks, audit, and retrieval traces.
- `sdkwork-drive` is the system of record for file/object bytes.
- Vector, full-text, parser, OCR, embedding, rerank, and external knowledge-engine capabilities are ports with replaceable adapters.
- PostgreSQL plus pgvector is the default production SQL/vector adapter.
- SQLite is the default local/test metadata adapter.

---

## 3. Standards alignment

This design follows SDKWork standards under `../sdkwork-specs/`:

- API contracts and `SdkWorkApiResponse`: `API_SPEC.md` §15–§16
- SDK generation: `SDK_SPEC.md`
- Database contracts: `DATABASE_SPEC.md`
- Security: `SECURITY_SPEC.md`
- Performance: `PERFORMANCE_SPEC.md`
- Observability: `OBSERVABILITY_SPEC.md`
- Media/object references: `MEDIA_RESOURCE_SPEC.md`

Shared utilities: prefer `sdkwork-utils-rust` and `sdkwork-id-core` over local duplicates.

---

## 4. Non-goals

- No handwritten app/backend SDK clients — use generated SDK families.
- No direct object storage in the knowledgebase service.
- No file bytes or presigned URLs persisted as business state in SQL.
- No fake parsing, indexing, retrieval, or successful job completion.
- No unpublished OKF concept is authoritative without source lineage, revision history, and citation/provenance state.

---

## 5. Domain model

Primary app-local domain: `knowledge`.

```yaml
domain: knowledge
status: app-local-extension
owner: sdkwork-knowledgebase
database_prefix: kb
api_tags:
  - knowledge
sdk_namespaces:
  - knowledge
capabilities:
  - spaces
  - collections
  - sources
  - documents
  - document_versions
  - drive_object_refs
  - ingestion_jobs
  - parse_artifacts
  - chunks
  - embeddings
  - retrieval_profiles
  - retrievals
  - citations
  - okf_concepts
  - okf_bundle_files
  - okf_log_entries
  - okf_candidates
  - local_mirror_packages
  - local_mirror_deltas
  - local_runtime_bundles
  - acl
  - audit_events
depends_on:
  - iam
  - content
  - intelligence
  - sdkwork-drive
rust_parity: required
sdk_generation: required
```

Knowledge representation uses **OKF v0.1** bundles per space. Agent provider id: `provider.knowledge.okf`. HTTP operations use the `okf.*` operationId family.

---

## 6. Rust workspace structure

```text
sdkwork-knowledgebase/
  Cargo.toml
  crates/
    sdkwork-knowledgebase-contract/
    sdkwork-knowledgebase-config/
    sdkwork-knowledgebase-drive/
    sdkwork-knowledgebase-memory/
    sdkwork-knowledgebase-agent-provider/
    sdkwork-knowledgebase-worker/
    sdkwork-intelligence-knowledgebase-service/
    sdkwork-intelligence-knowledgebase-repository-sqlx/
    sdkwork-routes-knowledgebase-{app,backend,open}-api/
    sdkwork-knowledgebase-engine-*/
    sdkwork-api-knowledgebase-standalone-gateway/
  apps/sdkwork-knowledgebase-pc/
  apis/
  sdks/
  database/
  specs/
```

### 6.1 Crate responsibilities

| Crate | Responsibility |
| --- | --- |
| `sdkwork-knowledgebase-contract` | Shared DTOs, enums, OKF types, validation |
| `sdkwork-intelligence-knowledgebase-service` | Business logic, OKF bundle services, retrieval, ingestion |
| `sdkwork-intelligence-knowledgebase-repository-sqlx` | SQLx persistence for `kb_*` tables |
| `sdkwork-knowledgebase-drive` | `KnowledgeDriveStorage` adapter over `sdkwork-drive` |
| `sdkwork-routes-knowledgebase-*-api` | Axum HTTP boundaries, response envelope mapping |
| `sdkwork-knowledgebase-agent-provider` | Agent kernel OKF knowledge provider |
| `sdkwork-knowledgebase-worker` | Outbox, ingest maintenance, async jobs |

---

## 7. Drive integration

### 7.1 Port boundary

Only `sdkwork-knowledgebase-drive` implements `KnowledgeDriveStorage` by calling `DriveObjectStore`. Knowledgebase production code must not read or join drive physical tables (`dr_*`).

Drive workspace, space, node, and object-version metadata are owned by `sdkwork-drive`. When knowledgebase needs a browser-visible folder tree or knowledge drive space, `sdkwork-drive` exposes that through product services or generated SDK APIs.

### 7.2 Space initialization (compensating workflow)

1. Create `kb_space` in an uninitialized active state.
2. Provision or find the dedicated Drive `knowledge_base` space through `sdkwork-drive`.
3. Bind `kb_space.drive_space_id`.
4. Bootstrap OKF bundle standard files under `okf/` through `OkfBundleInitializer`.
5. Mark `kb_space.okf_bundle_initialized = true`.

If any step after local space creation fails, soft-delete the local `kb_space` row and release the Drive space when safe. Cleanup failures must surface explicitly.

### 7.3 Object roles

Every `kb_drive_object_ref` has an explicit `object_role`. Document ingestion roles include `original_document`, `normalized_text`, `ocr_result`, `page_image`, `thumbnail`, `parse_manifest`, and `chunk_manifest`. OKF bundle roles include `bundle_profile`, `bundle_agents`, `bundle_index`, `bundle_log`, `concept_revision`, `graph_export`, `context_pack`, and `output_export`. Export and mirror roles include `export_archive`, `local_mirror_snapshot`, `local_mirror_delta`, and `local_runtime_bundle`.

Rules:

- `original_document` is immutable after document version creation.
- Parser outputs are new drive objects.
- Download URLs are short-lived grants, never stored as system-of-record values.
- When the Drive provider does not return a native object version, synthesize `sha256:<content_digest>` before persisting a locator.

### 7.4 Forbidden storage bypasses

Knowledgebase production code must not call `std::fs` for business storage, use cloud SDKs directly, construct presigned URLs outside drive ports, store object bytes in SQL, or issue SQL against `sdkwork-drive` physical tables.

---

## 8. OKF knowledge bundle

Each knowledge space stores curated knowledge as an OKF v0.1 bundle. Drive layout:

```text
{space_root}/
|-- sources/raw/                 # immutable original source files
|-- okf/                         # generated OKF bundle root
|   |-- index.md
|   |-- log.md
|   |-- schema/
|   |   |-- AGENTS.md
|   |   `-- okf_profile.yaml
|   `-- <domain>/<concept>.md
|-- output/                      # generated answers, reports, and exports
`-- .sdkwork/governance/         # drafts and revisions (never exported)
```

SQL stores `kb_okf_concept`, `kb_okf_concept_revision`, `kb_okf_concept_link`, `kb_okf_bundle_file`, `kb_okf_log_entry`, and `kb_okf_candidate` metadata. File bytes flow only through `sdkwork-drive`.

The knowledge browser separates user-facing file navigation from generated OKF bundle navigation. For OKF spaces, `spaces.browser.list?view=files` resolves to `sources/raw` and returns original source files only. Generated concepts and bundle files are available through `view=okf_bundle`, and generated outputs through `view=outputs`. The response uses `KnowledgeBrowserListData` so clients receive the resolved Drive `parentId` for root uploads and root folder creation.

See [TECH-2026-06-19-okf-knowledge-bundle-design.md](TECH-2026-06-19-okf-knowledge-bundle-design.md) for workflows (ingest, compile, eval, query, lint), export modes, and purge rules.

---

## 9. Database design

All database objects created by SDKWork Knowledgebase use the `kb_` prefix for tables, `idx_kb_` for non-unique indexes, and `uk_kb_` for unique indexes.

### 9.1 Runtime ID strategy

All persistent `kb_*` tables use `id` as an `int64` internal primary key. Runtime insert paths generate and bind `id` explicitly through the SDKWork database-backed Snowflake allocator (leased node range 0–1023 with heartbeat and fencing). Public API identifiers use UUID strings. A numeric `SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID` is permitted only as an explicitly approved development or emergency override.

Common columns for core L2 tables: `id`, `uuid`, `tenant_id`, `organization_id`, `status`, `created_at`, `updated_at`, `version`, optional soft-delete fields.

API serialization: numeric IDs and counters are strings in JSON; timestamps are ISO 8601 UTC.

### 9.2 Core tables

| Table | Purpose |
| --- | --- |
| `kb_space` | Knowledge space; includes `okf_bundle_initialized`, `okf_log_sequence_counter`, `drive_space_id` |
| `kb_source` | Import source record |
| `kb_drive_object_ref` | Stable reference to a drive object |
| `kb_document` / `kb_document_version` | Document master data and immutable versions |
| `kb_ingestion_job` / `kb_ingestion_job_item` | Async ingest pipeline |
| `kb_parse_artifact` / `kb_chunk` / `kb_embedding` | Parse and index read models |
| `kb_retrieval_profile` / `kb_retrieval_trace` | Retrieval configuration and audit |
| `kb_okf_*` | OKF concept, revision, link, bundle file, log, candidate metadata |
| `kb_outbox_event` | Reliable async dispatch |

Authoritative DDL lives under `database/` and `crates/sdkwork-intelligence-knowledgebase-repository-sqlx/migrations/`.

---

## 10. Application services

| Service | Responsibility |
| --- | --- |
| `KnowledgeSpaceService` | Create, update, list, archive spaces; enforce tenant ownership |
| `KnowledgeDocumentService` | Document master data and versions |
| `KnowledgeDriveStorageService` | Wrap drive port; validate locators and checksums |
| `KnowledgeIngestionService` | Jobs, idempotency, retry semantics |
| `KnowledgeParseService` / `KnowledgeChunkingService` / `KnowledgeEmbeddingService` | Parse pipeline and index materialization |
| `KnowledgeRetrievalService` | Keyword/vector/hybrid retrieval with security trimming |
| `OkfConceptService` | Publish, upsert, read OKF concepts |
| `OkfBundleLinter` / `OkfConformanceValidator` | Bundle quality and OKF v0.1 compliance |
| `OkfIndexSynthesizer` / `OkfLogSynthesizer` | Materialize `index.md` and `log.md` |
| `OkfBundleImporter` / `OkfBundleExporter` | External bundle import and mirror/tarball export |
| `OkfBundleInitializer` | Space bootstrap |
| `KnowledgeLocalMirrorPackageService` | Snapshot and delta packages via drive |
| `KnowledgePermissionService` | ACL/RBAC/ABAC evaluation |
| `KnowledgeAuditService` | Audit events and outbox |

OKF service modules live under `crates/sdkwork-intelligence-knowledgebase-service/src/okf/`.

---

## 11. API design

| Surface | Prefix | SDK family | Auth |
| --- | --- | --- | --- |
| App API | `/app/v3/api` | `sdkwork-knowledgebase-app-sdk` | dual-token |
| Backend API | `/backend/v3/api` | `sdkwork-knowledgebase-backend-sdk` | dual-token + `knowledge.platform.manage` |
| Open API | `/knowledge/v3/api` | `sdkwork-knowledgebase-sdk` | API key |

OpenAPI authorities are authored in `sdks/*/openapi/` and synchronized to `apis/` via `pnpm api:materialize`. Generated SDKs use dotted resource operation IDs.

### 11.1 Response envelope

All L2+ success JSON bodies use `SdkWorkApiResponse`:

```json
{ "code": 0, "data": { ... }, "traceId": "<server-uuid>" }
```

- Single resource: `data.item`
- Lists: `data.items` + `data.pageInfo`
- Commands: `data.accepted` plus optional `resourceId` / `status`
- Errors: HTTP 4xx/5xx with `application/problem+json` (`ProblemDetail`) including numeric `code` and `traceId`

Forbidden legacy envelopes: `PlusApiResult`, `*ApiResult`, wire field `requestId`, bare domain DTOs at the HTTP root.

### 11.2 OKF operation family

OKF HTTP operations use the `okf.*` prefix. Examples:

```text
okf.concepts.list
okf.concepts.retrieve
okf.concepts.revisions.list
okf.bundle.index.list
okf.bundle.export.create
okf.lintRuns.create
okf.evalRuns.create
okf.bundle.import.create
```

Document, space, retrieval, and ingest operations use the `knowledge` tag with `spaces.*`, `documents.*`, `ingests.*`, `retrievals.*`, etc. Authoritative operation lists are in the OpenAPI files.

---

## 12. SDK generation

SDK families:

- `sdks/sdkwork-knowledgebase-app-sdk/`
- `sdks/sdkwork-knowledgebase-backend-sdk/`
- `sdks/sdkwork-knowledgebase-sdk/`

All SDKWork Knowledgebase HTTP SDKs MUST be generated by the canonical SDKWork generator at `..\sdkwork-sdk-generator`, invoked through `..\sdkwork-sdk-generator\bin\sdkgen.js`.

Generation flags:

```text
--standard-profile sdkwork-v3
--fixed-sdk-version 0.1.0
```

Each generated TypeScript SDK root contains:

```text
sdkwork-sdk.json
.sdkwork/sdkwork-generator-manifest.json
.sdkwork/sdkwork-generator-changes.json
.sdkwork/sdkwork-generator-report.json
custom/
src/
```

Local build artifacts (`node_modules/`, `dist/`, `package-lock.json`) must not be committed under generated SDK roots.

Run `node sdks/standardize-knowledgebase-sdk-family.mjs` after OpenAPI changes. Generated TypeScript SDKs unwrap `data` by default.

---

## 13. Security design

Protected app/backend APIs require dual-token security per SDKWork IAM standards.

Controls:

- Tenant and organization context from validated token context (fail-closed).
- Object-level authorization via `KnowledgePermissionService`.
- Retrieval security trimming before results are returned.
- Admin/backend operations require `knowledge.platform.manage` or finer-grained permission codes.
- Audit events for document import, delete, permission changes, retrieval profile changes, reindex, and object reconciliation.
- Redact tokens, drive credentials, presign material, provider secrets, and private object keys from logs and errors.

---

## 14. Performance design

| Class | Examples |
| --- | --- |
| P1 interactive | Space/document list, retrieval |
| P2 async | Upload, import, parse, embed, reindex |
| P3 background | Audit reconciliation |

Default limits: page size 20 (max 100 user / 200 admin), retrieval topK bounded, batch `IN (...)` ≤ 200, interactive timeout 30s.

---

## 15. Observability

Every API request produces `traceId`, route template, `operationId`, latency, and status. Worker jobs expose job/item state, attempt count, and stage latency.

OKF Prometheus counters include `kb_okf_concept_publish_total`, `kb_okf_bundle_lint_issues_total`, `kb_okf_conformance_failures_total`, and related bundle workflow counters. `/metrics` is in-cluster only.

---

## 16. Error model

HTTP errors use RFC 9457 `ProblemDetail` with numeric platform codes per `API_SPEC.md` §15.3. Domain-specific problem types map through `sdkwork-web-framework` response mapping.

---

## 17. Testing and verification

```bash
pnpm verify
pnpm check
pnpm test
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node tools/check_okf_knowledge_bundle_standard.mjs
cargo test --workspace
```

Launch acceptance: [PRD-mvp-launch.md](../../product/prd/PRD-mvp-launch.md).

---

## 18. Related documents

- Phase 1 implementation record: [TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md](TECH-2026-06-01-knowledgebase-backend-phase1-implementation.md)
- Structure standardization: [TECH-2026-06-11-sdkwork-structure-standardization-design.md](TECH-2026-06-11-sdkwork-structure-standardization-design.md)
