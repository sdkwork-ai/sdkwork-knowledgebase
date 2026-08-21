# REVIEW-20260721 Live Wiki Deployment Integration Readiness

Status: core-delivery-implemented-production-evidence-blocked
Owner: SDKWork Knowledgebase maintainers
Date: 2026-07-23
Requirement: REQ-2026-0721
Decision: ADR-20260721-live-mounted-wiki-publication (accepted)
Machine contract: `specs/live-wiki-publication.spec.json`
Specs: REQUIREMENTS_SPEC.md, ARCHITECTURE_DECISION_SPEC.md, API_SPEC.md, INTERNAL_API_SPEC.md,
SDK_SPEC.md, EVENT_SPEC.md, DATABASE_SPEC.md, DEPLOYMENT_SPEC.md, RELEASE_SPEC.md,
SECURITY_SPEC.md, PERFORMANCE_SPEC.md, PAGINATION_SPEC.md, TEST_SPEC.md

## 1. Scope And Review States

This review records executable implementation evidence across Knowledgebase, Drive, Deployments,
and Web Server. It supersedes earlier absence findings that were closed by the canonical Wiki
schema, Drive event projection, Knowledgebase public provider, generated Internal SDK, and Web
Server immutable runtime-set work.

The states in this review mean:

- `closed`: the owned contract and focused executable evidence exist;
- `partially closed`: a bounded production-shaped implementation exists but a named capability or
  its required production evidence is still missing;
- `blocking`: the required business or runtime path is absent, so public/commercial claims remain
  prohibited.

The reviewed request path is:

```text
Drive sources/raw commit
  -> Knowledgebase durable event projection
  -> WikiPublication and page public-version state
  -> Knowledgebase typed Internal API / generated SDK
  -> Web Server KNOWLEDGEBASE_WIKI adapter
  -> Deploy-owned Site/Binding/Variant/Mount routing
  -> public HTTP response
```

Ordinary source updates, publish/unpublish actions, navigation changes, and search changes are
provider lifecycle operations. They must not create a Deploy Release, Deployment, or SiteRevision.

## 2. Verdict

The storage, source projection, initialization-to-READY chain, native source processing,
explicit/automatic publication lifecycle, sanitized native public reads, contract generation,
immutable Web runtime, generated-SDK Web provider adapter, durable provider-event processing,
data-plane bootstrap, and public HTTP mapping are implemented. The repositories still do not
provide the complete commercial Wiki product because isolated multi-format renditions, Range and
rendition full-text search, managed cloud TLS closure, UI workflows, and deployed end-to-end
freshness, security, and scale evidence remain incomplete.

The system must therefore be described as `core-delivery-implemented-production-evidence-blocked`.
It is incorrect to describe the Wiki schema, Drive consumer, Knowledgebase provider API/SDK, Web
runtime-set, or focused cross-repository delivery path as absent. It is also incorrect to describe
the overall capability as production-ready, commercially ready, or backed by a certified realtime
SLO.

## 3. Current Evidence Matrix

| Surface | Executable evidence | State |
| --- | --- | --- |
| Canonical contract | Accepted requirement, ADR, and `specs/live-wiki-publication.spec.json` | closed |
| Wiki persistence | PostgreSQL and SQLite baselines/migrations contain `kb_site_publication`, `kb_source_file_projection`, rendition, redirect, checkpoint, inbox, and outbox structures | closed |
| Wiki initialization | One canonical DRAFT/private publication is provisioned, `sources/raw` is bound, a Drive checkpoint and event delivery are verified, and the publication converges to READY; existing spaces are backfilled idempotently | closed |
| Drive source sync | Root-scoped events, inbox/checkpoint fencing, projection application, reconciliation, and standalone/cloud typed Drive adapters exist | closed |
| Native source processor | Worker-bounded checkpoint/source claims validate exact Drive identity/bytes, derive canonical routes, render-check native pages, retry with lease/fence protection, quarantine exhausted/active content, and expose maintenance counters | closed |
| Knowledgebase public provider | Active-publication lookup, normalized route/redirect resolution, opaque content handles, exact public-version validation, navigation, and metadata search are implemented | closed |
| Internal API and SDK | Six ingress-token owner operations exist in OpenAPI, route manifest, Rust routes, and generated TypeScript/Rust transports | closed |
| Public isolation | Provider reads derive tenant/organization from the authenticated principal and use non-disclosing not-found behavior | closed |
| Web runtime descriptor/set | Strict descriptor plus node-scoped `sdkwork.website-runtime-set.v1`, bounded compilation, collision rejection, atomic activation, replay fencing, and rollback exist | closed |
| Web delivery executor | Immutable provider registry and runtime-set-backed STATIC/explicit SPA fallback/WIKI execution preserve compiled tenant/Site/Binding/Variant/Mount scope, typed outcomes, bounded streams, and browser HTTP semantics | closed |
| Content open | Exact pinned Drive version, source and rendered length/SHA-256/media type, and current page public-version are revalidated; native pages return sanitized HTML, while the reader buffers at most 16 MiB and has no Range contract | partially closed |
| Search | Store-paginated metadata search covers title, canonical route, and source path; rendition-backed full-text search is absent | partially closed |
| Publication lifecycle | Owner-only activate/pause plus Writer publish/republish/unpublish/visibility commands use optimistic publication/page fences, exact Drive-version pinning, transactional lifecycle events, and transactionally coupled audit records | closed |
| App API and SDK | Six owner operations exist in App OpenAPI, Rust routes/manifest, and the generated TypeScript App SDK; Reader/Writer/Owner and organization-isolation tests pass | closed |
| Provider event production | The owner AsyncAPI authority defines all five event types; provider, route change/revocation, navigation, and search events are transactionally produced, and source-driven public revocation advances navigation/search generations and emits all three invalidation facts atomically | closed |
| Provider event consumption | Durable Web Server checkpoints, duplicate/order/gap fencing, initial/gap reconciliation, concurrent stream isolation, and route-scoped invalidation are implemented and tested | closed |
| Web Server Wiki adapter | Generated Knowledgebase Rust Internal SDK adapter implements resource/Wiki ports with tenant-bound resolution, conditional metadata, bounded content, navigation/search, registry/bootstrap wiring, initial/hot-update validation, and browser-facing tests | closed |
| Render/rendition safety | Markdown/MDX and safe HTML/text rendering use an HTML5 sanitizer, bounded output, and active-content tests; the isolated multi-format/Drive-backed rendition chain is not executable | partially closed |
| Deploy-to-Web delivery | A focused cross-repository test compiles a real Deploy Site/runtime set, activates the exact output in Web, routes desktop/mobile requests through the Knowledgebase provider and fake generated-SDK boundary, fails private/unpublished routes closed, and observes live content without a new revision/generation/snapshot | closed |
| Managed TLS | Domain/certificate policy foundations exist; automated ACME renewal, rotation, fleet convergence, and expiry-drill evidence remain incomplete | blocking |
| User/admin workflows | Generated-SDK-backed publication, source-state, domain/TLS, provider-health, reconciliation, and failure-management views are incomplete | blocking |
| Commercial launch | Release, security, performance, soak, backup/restore, billing reconciliation, rollout, rollback, and live-smoke evidence are incomplete | blocking |

## 4. Implemented Public Provider Contract

The Knowledgebase Internal API authority owns exactly these operations:

| Operation id | Method and path | Implemented behavior |
| --- | --- | --- |
| `driveEvents.receive` | `POST /internal/v3/api/knowledgebase/drive_events` | authenticated Drive event ingestion |
| `wikiPublications.retrieve` | `GET /internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}` | active publication metadata and provider generations |
| `wikiPublications.routes.resolve` | `POST /internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/routes/resolve` | normalized route or reviewed redirect resolution |
| `wikiPublications.contents.retrieve` | `GET /internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/contents/{contentHandle}` | bounded exact pinned-version representation retrieval; native pages return sanitized HTML |
| `wikiPublications.navigation.list` | `GET /internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/navigation` | public-only keyset navigation window |
| `wikiPublications.pages.search` | `GET /internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/pages/search` | public-only keyset metadata search |

Direct route resolution permits `PUBLIC` and `UNLISTED`. Navigation permits only `PUBLIC` with
`nav_hidden=false`. Search permits only `PUBLIC` with `index_state=READY`. Every public read
revalidates tenant, organization, publication status, page eligibility, and current page public
version.

The current representation operation is deliberately not described as streaming: it rejects a
source larger than 16 MiB and returns a bounded buffered body. Native rendered output is separately
bounded to 32 MiB and its media type, byte length, and SHA-256 are bound into the opaque handle. The
current search operation is deliberately not described as full-text: it searches normalized
metadata only.

## 5. Remaining P0 Closure Work

### P0-1 Deployed Freshness And Cache Evidence

Web Server now consumes `knowledgebase.wiki.provider.changed.v1`,
`knowledgebase.wiki.route.changed.v1`, `knowledgebase.wiki.route.revoked.v1`,
`knowledgebase.wiki.navigation.changed.v1`, and `knowledgebase.wiki.search.changed.v1` through a
durable checkpoint/reconciliation processor. The current delivery path has no content cache, so the
delivery executor instead maintains a bounded node-local resolution metadata cache and never caches
response bodies or credentials. It has Provider-qualified positive and non-disclosing negative
entries, exact route/Provider/type invalidation, revocation eviction, bounded single-flight,
positive-only stale revalidation, uncertainty purge, epoch fencing, O(1) LRU eviction, and
fixed-cardinality Prometheus metrics. Authenticated provider read-through remains the correctness
path. Remaining closure is measured deployed freshness, outage, event-storm, eviction, and capacity
evidence; shared/edge body caching is separate future work. Deployments remains outside this content
hot path.

### P0-2 Rendition, Range, And Search Completion

Add a streaming/Range contract before enabling large PDF/media/download workloads. Retain the
implemented versioned native Markdown/HTML/text sanitizer and complete the isolated multi-format,
Drive-backed rendition chain. Replace metadata-only search with a tenant/publication/public-version-
filtered rendition index before claiming full-text Wiki search.

### P0-3 Deployed Delivery, TLS, And Product Operations

The focused Site/Binding/Variant/Mount-to-provider execution contract is closed with real Deploy
compiler output and Web/Knowledgebase adapter execution. Still required: prove that path in
standalone and cloud deployed topologies, complete automatic ACME renewal/rotation and served-SNI
convergence, and deliver generated-SDK-backed user/admin workflows, provider health, lag/gap,
reconcile, cache purge, quota, audit, and commercial usage operations.

## 6. Realtime Claim Boundary

Drive-to-Knowledgebase projection, native source readiness, eligible automatic/explicit public-state
transitions, and Web provider-event consumption are event-driven and durable. Web fences duplicates
and ordering, reconciles gaps, and scopes invalidation to the affected publication/route. Public
Wiki freshness is not yet a certified commercial realtime SLO because the cross-repository path has
not been measured in a deployed production-like environment and a future content cache still needs
its concrete eviction evidence.

For this system, realtime means bounded eventual visibility from the committed public-state
transition, not from upload completion. Events improve freshness; authenticated provider
read-through validation remains the correctness authority. Private, quarantine, delete, pause, and
unpublish transitions must deny public reads immediately even during event or cache lag.

## 7. Verification Evidence

The implemented provider boundary has passed:

```text
cargo test -p sdkwork-intelligence-knowledgebase-service --test wiki_public_provider
cargo test -p sdkwork-intelligence-knowledgebase-service --test wiki_source_processor
cargo test -p sdkwork-intelligence-knowledgebase-service wiki_representation
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test wiki_persistence_store
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test wiki_public_provider_store
cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test wiki_publication_lifecycle_store
cargo test -p sdkwork-routes-knowledgebase-app-api --test wiki_publication_routes
cargo test -p sdkwork-routes-knowledgebase-app-api --test wiki_publication_hosted_access
cargo test -p sdkwork-knowledgebase-worker
cargo test -p sdkwork-routes-knowledgebase-app-api --test app_openapi_routes
cargo test -p sdkwork-routes-knowledgebase-internal-api --test internal_routes
pnpm api:materialize:check
node tools/knowledgebase_sdk_generate.mjs --check --family sdkwork-knowledgebase-app-sdk
pnpm --dir sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript typecheck
node sdks/sdkwork-knowledgebase-internal-sdk/bin/generate-sdk.mjs --check
node --test sdks/sdkwork-knowledgebase-internal-sdk/tests/sdk-family-smoke.test.mjs
pnpm --dir sdks/sdkwork-knowledgebase-internal-sdk/sdkwork-knowledgebase-internal-sdk-typescript typecheck
node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --root .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --root .
node ../sdkwork-specs/tools/check-route-path-collisions.mjs --root .
node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
```

The Web Server repository adapter boundary has additionally passed:

```text
cargo test -p sdkwork-webserver-contract
cargo test -p sdkwork-webserver-delivery-runtime
cargo test -p sdkwork-webserver-knowledgebase-provider
cargo test -p sdkwork-api-webserver-standalone-gateway
cargo check --workspace
cargo clippy -p sdkwork-webserver-core -p sdkwork-webserver-contract -p sdkwork-webserver-drive-provider -p sdkwork-webserver-knowledgebase-provider -p sdkwork-webserver-delivery-runtime -p sdkwork-api-webserver-standalone-gateway --all-targets -- -D warnings
cargo test -p sdkwork-deploy-runtime-compiler --test knowledgebase_wiki_delivery_contract
```

The generated TypeScript and Rust package check/build workflows also pass. Web Server provider-event
tests additionally prove durable checkpoints, duplicate/order/gap handling, reconciliation, and
route-scoped invalidation. These checks prove the bounded provider, explicit publication command,
generated-SDK Web adapter, event/source processors, native safe renderer, runtime-set activation,
bootstrap, browser HTTP mapping, and real Deploy-compiler-to-Web/Knowledgebase execution boundaries.
They do not prove production cache capacity and invalidation SLOs, a multi-format rendition chain,
TLS, UI, or real deployed cross-repository delivery SLOs.

## 8. Claim Policy

Until every remaining P0 item is closed with executable evidence:

- Wiki public deployment remains production-evidence-gated;
- upload or processing success must not be presented as public publication success;
- the current bounded representation reader must not be presented as large-object streaming or
  Range delivery;
- the current metadata query must not be presented as full-text search;
- no commercial or production launch may rely on the path until its deployed production evidence
  is complete.
