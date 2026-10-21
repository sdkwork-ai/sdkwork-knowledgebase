# SDKWork Knowledgebase PRD

Status: active
Owner: SDKWork maintainers
Application: sdkwork-knowledgebase
Updated: 2026-07-31
Specs: REQUIREMENTS_SPEC.md, DOCUMENTATION_SPEC.md

## Document Map

- [PRD-mvp-launch.md](PRD-mvp-launch.md) - MVP launch scope and acceptance criteria
- [PRD-phase2-commercial-saas.md](PRD-phase2-commercial-saas.md) - commercial release readiness gates (compatibility path)
- [PRD-live-wiki-publication.md](PRD-live-wiki-publication.md) - active live `sources/raw` Wiki,
  per-file state, reader/author/admin experience, Deploy integration, quotas, and launch gates.

## 1. Background And Problem

Teams need a knowledge platform that combines structured documentation, retrieval-augmented search, and AI-assisted authoring without sacrificing tenant isolation, auditability, or SDKWork platform integration. Existing wikis often lack native RAG, OKF knowledge bundles, and consistent IAM/SDK contracts.

## 2. Target Users

| Persona | Need |
|---------|------|
| Knowledge author | Create/edit documents, import from Drive/Git, publish to WeChat or a governed live Wiki |
| Team member | Search, read, collaborate within granted spaces |
| Tenant admin | Manage spaces, members, ingestion sources, OKF profiles |
| Platform operator | Operate backend-api, worker, observability, and per-tenant deployments |
| Integrator / ISV | Consume Open API and generated SDKs for retrieval and ingest |

## 3. Goals And Non-Goals

**Goals (MVP)**

- Secure multi-surface API (app / backend / open) with SDK-first clients
- Knowledge spaces with Drive-backed storage, ingest, and RAG retrieval
- PC browser and desktop shell for authoring, search, and market/catalog
- Production deployment topology (replicated application public ingress, worker, Postgres, probes, audit)
- Shared cloud object storage through an active Drive provider; cloud replicas never use pod-local
  files as authoritative knowledge content
- Fail-closed auth, tenant/org guards, and backend `knowledge.platform.manage` RBAC
- Managed IM Conversation group knowledge spaces that are created only by the current IM Owner's
  first initialization or failed-provisioning retry, use dedicated group binding and
  current-membership authorization, and open in the full standalone browser or Tauri Knowledgebase
  application only for active joined non-Guest Owners, Admins, and Members. See
  [REQ-2026-0713-group-knowledgebase.md](../requirements/REQ-2026-0713-group-knowledgebase.md).

**Non-Goals (MVP)**

- Shared request-scoped multi-tenant process pooling
- Product billing/subscription ownership, which remains delegated to the SDKWork platform
- Real-time collaborative editing
- Mobile native clients
- Full enterprise compliance program (SOC2) - platform-level concern

## 4. Scope

**In scope**

- App API: spaces, documents, browser, ingest, retrieval, agent chat, bounded WeChat configuration,
  market, and Wiki publication management
- Official Account and Applet replacement accepts at most 100 entries of each kind and at most 50
  values per domain list. Unknown fields, oversized verification text, unsupported enum values, and
  browser/media URL avatar identities fail before persistence; secret inputs are write-only and
  responses are redacted. See
  [REQ-2026-0731](../requirements/REQ-2026-0731-wechat-config-input-boundaries.md).
- Provider-backed media tasks: CloudRouter SDK image generation and speech-to-text; requests fail closed when provider configuration is absent
- Backend API: sources, OKF compile/candidates, indexes, retrieval profiles/traces
- Backend Provider management: explicit tenant/organization/space bindings, capability-aware
  testing and activation, write-only credential references, disable/switch/migrate/rollback, and
  sanitized health. External spaces fail closed without one active tested binding.
- Open API: retrieval, context packs, ingest, document/browser read
- Worker: outbox dispatch, ingestion maintenance
- PC client: editor, search, settings, offline/network awareness
- OKF original file list: knowledgebase file lists call `spaces.browser.list?view=files`; for OKF spaces this displays original source files under `sources/raw` only and must not expose `okf/`, `output/`, `.sdkwork/`, or Drive root system folders.
- OKF browser view separation: OKF concept and bundle tooling uses `view=okf_bundle`; generated output tooling uses `view=outputs`. Root uploads and root folder creation use response `data.parentId` as the Drive parent folder id, never Drive root or a hard-coded `sources/raw` path.
- Knowledgebase Wiki publication: every Knowledgebase is Wiki-capable through exactly one canonical
  WikiPublication provisioned DRAFT/PRIVATE and initialized through fixed-root/checkpoint/event
  delivery verification to READY; no special Knowledgebase type or conversion is required. An
  explicitly active Wiki projects governed multi-format sources
  and static assets from the fixed Drive `sources/raw` root, maintains processing, publication,
  visibility, index, route, and public-version state per file, and resolves eligible representations
  live through the shared Deploy/Web Server site platform. Upload completion never grants anonymous
  access, and ordinary provider lifecycle changes create no SiteRelease, Deploy Release, Deployment,
  or SiteRevision.

**Out of scope (MVP)**

- Payment/subscription product billing
- SCIM/SAML beyond platform IAM
- Page-level real-time co-editing
- WeChat article publish/preview until managed Drive cover identity, optimistic configuration
  concurrency, durable idempotency, recovery, and live-provider evidence pass release review

## 5. User Scenarios

1. **Author creates a space and document** - login -> create space -> create doc -> edit in TipTap -> auto-save via SDK
2. **Member searches knowledge** - search module -> RAG answer with citations -> navigate to source doc
3. **Admin configures ingest** - backend operator with `knowledge.platform.manage` -> create source -> worker completes job -> retrieval returns new chunks
4. **Integrator retrieves via Open API** - API key with org context -> retrieval -> context pack for downstream agent
5. **Operator deploys tenant** - register and health-check the Drive provider -> configure K8s
   tenant, database, provider id, secrets, and outbox delivery -> `/readyz` green -> smoke test
6. **Admin activates an external Provider** - create a write-only credential reference -> create a
   draft binding for one remote resource -> test capabilities -> activate with optimistic version
   fencing -> observe health and quality -> retain the predecessor for rollback

7. **Author manages OKF source files** - open OKF space file list -> view original files from `sources/raw` through `view=files` -> upload or create root folder using `data.parentId` -> inspect generated OKF concepts only through `view=okf_bundle`
8. **Group member opens managed knowledge** - the current IM Owner initializes the group action or
   retries failed provisioning -> exactly one managed space is provisioned -> after activation,
   joined non-Guest Owner, Admin, and Member roles open only that fixed workspace through a one-time
   ticket -> Guest, left, removed, and non-member actors are denied -> removal, role reduction, or
   dissolution updates access and archive state without deleting documents by default
9. **Author publishes a live Wiki** - upload governed pages, documents, media, and assets through
   Drive into `sources/raw` -> inspect processing/publication/visibility/index/route state -> use
   private preview -> explicitly publish, schedule, or use an Owner-approved auto-public policy after
   checks -> open the Deploy-bound Wiki URL -> unpublish or restore a prior Drive/Knowledgebase
   version without a content deployment.

## 6. Success Metrics

| Metric | MVP target |
|--------|------------|
| API availability (per tenant deployment) | >= 99.9% monthly after commercial launch |
| P95 retrieval latency (warm index) | < 2s |
| Authz failures correctly return 403 (no data leak) | 100% in integration tests |
| Critical security alignment tests | Pass in CI |
| Rejected WeChat configuration persisted | 0 |
| PC shell smoke (login + load) | Pass in CI Playwright |
| Document save success when online | > 99% |
| Eligible Wiki public-version commit to Web visibility | p95 <= 5s, p99 <= 30s; >= 99% within objective |
| Draft/private/quarantined/deleted/cross-tenant public disclosure | 0 |
| Canonical WikiPublication cardinality | exactly 1 per Knowledgebase; no duplicate on retry/backfill |
| Cloud object storage readiness | 100% of ready replicas resolve the same active non-local Drive provider and bucket |

## 7. Phases

| Phase | Focus | Exit criteria |
|-------|-------|---------------|
| **1.0 (current, prelaunch)** | Dedicated commercial deployment | PostgreSQL release evidence, dual tenant/organization isolation, shared Drive provider, runbooks, real-API E2E, durable worker recovery, and removal of the temporary `AnyPool` exception |
| **2.0** | Commercial operations and Live Wiki | Capacity/SLO evidence, platform billing integration, bounded privacy workflows, signed release artifacts, and approved live Wiki publication |
| **3.0** | Industry parity | Real-time collab, analytics, mobile |

## 8. Linked Requirements

- `docs/architecture/tech/TECH-2026-06-01-knowledgebase-backend-design.md`
- `docs/architecture/tech/TECH-2026-06-09-knowledgebase-agent-rag-design.md`
- `docs/architecture/decisions/ADR-20260731-dedicated-tenant-organization-runtime.md`
- `docs/product/requirements/REQ-2026-0713-group-knowledgebase.md`
- `docs/architecture/decisions/ADR-20260713-group-knowledgebase-binding-and-launch.md`
- `docs/product/requirements/REQ-2026-0720-knowledge-engine-provider-commercialization.md`
- `docs/architecture/decisions/ADR-20260720-knowledge-engine-provider-binding-spi-v2.md`
- `docs/product/requirements/REQ-2026-0721-live-wiki-cloud-publication.md`
- `docs/architecture/decisions/ADR-20260721-live-mounted-wiki-publication.md`
- `docs/product/requirements/REQ-2026-0731-wechat-config-input-boundaries.md`
- `specs/okf-knowledge-bundle.spec.json` - OKF bundle layers, browser views, and raw source file list contract
- `specs/live-wiki-publication.spec.json` - active Wiki eligibility, state, provider, routing, and freshness contract
- `../sdkwork-specs/SECURITY_SPEC.md`, `IAM_SPEC.md`, `APP_SDK_INTEGRATION_SPEC.md`
- `deployments/README.md` - tenant isolation and observability

## 9. Resolved And Open Questions

- **Deployment isolation:** the only supported production model is one fixed tenant/organization
  pair per API/worker deployment. Shared request-scoped pooling is outside product scope.
- **Billing owner:** SDKWork platform integration remains a commercial release dependency; this
  application does not implement an independent subscription authority.
- **Minimum enterprise audit retention period:** policy is documented in
  [audit-retention.md](../../runbooks/audit-retention.md); bounded export, anonymization, subject
  legal holds, and automated batched purge are implemented, and a production-like purge run with
  its counter/log evidence remains a commercial release gate.
