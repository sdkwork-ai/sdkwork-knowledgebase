# REQ-2026-0721 Live Wiki Cloud Publication

```yaml
id: REQ-2026-0721
title: Publish a live sources/raw Wiki with per-file state through the shared cloud site platform
owner: SDKWork Knowledgebase maintainers
status: in-progress
source: product
problem: The release-builder site model conflicts with WYSIWYG Markdown publication, duplicates artifacts, and does not make the source tree the live public authority.
goals:
  - make every Knowledgebase Wiki-capable through one canonical publication aggregate
  - make active wiki publication and sources/raw the only eligible Knowledgebase public resource
  - maintain source, publication, visibility, and index state per file
  - provide professional live render, navigation, search, assets, routes, and SEO
  - integrate domains, Variants, TLS, and delivery through Deploy/Web Server
non_goals:
  - create an immutable site release for each content change
  - own domains or certificates
  - expose internal knowledgebase directories or storage topology
affected_surfaces:
  - database
  - api
  - sdk
  - backend
  - worker
  - pc
  - drive
  - deployment
  - webserver
```

Specs: REQUIREMENTS_SPEC.md, ARCHITECTURE_DECISION_SPEC.md, DATABASE_SPEC.md, DRIVE_SPEC.md,
API_SPEC.md, SDK_SPEC.md, APP_SDK_INTEGRATION_SPEC.md, SECURITY_SPEC.md, PRIVACY_SPEC.md,
PERFORMANCE_SPEC.md, OBSERVABILITY_SPEC.md, TEST_SPEC.md, MIGRATION_SPEC.md, RELEASE_SPEC.md,
MEDIA_RESOURCE_SPEC.md, SUPPLY_CHAIN_SECURITY_SPEC.md

## Requirements

1. Every Knowledgebase shall own exactly one canonical `WikiPublication`, provisioned with
   `publicationType=wiki`, `wikiStatus=DRAFT`, `publicationMode=REVIEW_REQUIRED`, and
   `defaultVisibility=PRIVATE`. Initialization shall bind the fixed Drive root `sources/raw`,
   provision its checkpoint, verify event delivery, and converge through `VALIDATING` to `READY`.
   Only an explicit version-fenced transition to `ACTIVE` can validate as publicly eligible
   `KNOWLEDGEBASE_WIKI`.
2. Uploads use Drive app SDK/uploader and Knowledgebase stores stable Space/node/version references,
   never object keys or presigned URLs.
3. Maintain per-file source, publication, visibility, and index state with permissioned optimistic
   transitions and non-disclosing public behavior.
4. Default new content to review-required/private. Permit auto-public-after-checks only through an
   explicit owner policy and required scan/projection/render/index gates.
5. Resolve every supported page/document/media route, sanitized representation, navigation, search,
   redirects, metadata, locale, and public asset through a typed provider SDK/service port.
6. Preserve the last verified public version during reprocessing only when configured; private,
   quarantine, delete, revoke, and security transitions override stale behavior.
7. Emit idempotent page/asset/publication/root/index events and support checkpoint reconciliation.
8. Do not create `kb_site_release` or copy rendered artifacts for ordinary publication changes.
9. Delegate Site/Binding/domain/Variant/TLS/runtime revision to Deploy and all standalone/cloud
   public HTTP/TLS/cache execution to Web Server through generated SDKs/service ports. Knowledgebase
   exposes no anonymous fixed public route that bypasses a WebsiteRuntimeDescriptor WIKI Mount.
10. Provide author/owner and backend-admin views, commercial entitlements/meters, audit, and
    operational evidence described by the PRD.
11. Support a governed raw-source format matrix: native Markdown/text pages; sanitized HTML;
    PDF/document/presentation/spreadsheet viewers through isolated processors; escaped source-code
    pages for JavaScript, TypeScript, CSS, JSON, YAML, XML, SQL, and common programming languages;
    approved image/audio/video viewers; and attachment-only or quarantined handling for archives and
    unsupported binaries.
12. Store generated previews, extracted text, thumbnails, posters, slide images, and other
    renditions as content-addressed, Drive-backed, rebuildable derived state keyed by source version
    and processor version. Renditions never become source authority or SiteRelease artifacts.
13. Never execute uploaded JavaScript, active HTML, service workers, WebAssembly, macros, formulas,
    or arbitrary CSS on the standard Wiki origin. Any future trusted active-site capability requires
    an isolated origin, signed packages, supply-chain evidence, and separate security approval.
14. Use author-immediate/public-gated visibility by default: uploaded files and processing progress
    appear immediately in the authenticated Source Explorer, private preview becomes available when
    ready, and the public Wiki changes only after an explicit version-fenced publish command.
    `AUTO_PUBLIC_AFTER_CHECKS` is an Owner-approved opt-in and must execute the same gates and
    attributable publish command. The public freshness SLO starts at the successful publish-state
    commit, not at upload completion.
15. Align cloud delivery with Deploy `sdkwork.website-runtime.v1`: `resourceUuid` identifies the
    Deploy Site Resource, `providerResourceUuid` identifies the Knowledgebase `WikiPublication`, and
    stable provider Space/root/contract-version fields contain no endpoint, token, storage key,
    presigned URL, database connection, or credential.
16. Let Deploy exclusively own Site, Site Resource, Variant, Mount, Binding, domain, TLS,
    descriptor revision, rollout, and delivery policy. Knowledgebase may retain stable connection
    references and status observations but must not duplicate those authorities.
17. Implement the Knowledgebase adapter compatibility contract for Web Server's unified
    `validateResource`, `resolveWikiRoute`, `openContent`, `searchWiki`, and
    `subscribeResourceEvents` semantics, with authenticated tenant/resource-scoped runtime context
    and non-disclosing public errors.
18. Key Web Server Wiki caches by the full Site revision policy generation, tenant scope, Binding,
    Variant, Mount, Deploy resource, provider resource generation, normalized route, public content
    version, renderer/theme/locale, and encoding tuple. Event uncertainty must trigger provider
    revalidation and cannot expand stale eligibility.
19. Create Deploy `SiteRevision` only for Deploy-owned Site composition or delivery/security/
    observability policy changes. Ordinary Wiki upload, processing, publication, visibility,
    rendering, navigation, search, quarantine, and delete changes emit provider lifecycle events and
    create no Deploy Release, Deployment, or `SiteRevision`.
20. Provision the canonical WikiPublication idempotently with Knowledgebase creation and backfill all
    eligible prelaunch Knowledgebases before feature activation. Provisioning failure is retryable
    and must not leave a second aggregate or make content public.
21. Permit multiple authorized Deploy Site Resources/Sites/Variants/Mounts to reference the same
    canonical WikiPublication. Knowledgebase must not persist one singular Site/domain authority or
    clone publication/content state per connection.
22. Treat a logical `sources/raw` file as a stable Drive node whose edits create new immutable Drive
    versions. A committed version/blob is immutable, but an existing logical path must not be
    rejected merely because it already has a current version.
23. Consume versioned Drive events `drive.node.version.committed.v1`,
    `drive.node.path.changed.v1`, `drive.node.eligibility.changed.v1`, and
    `drive.node.deleted.v1` through a Drive-owned AsyncAPI contract, durable root-scoped checkpoint,
    replay, dead letter and bounded reconciliation.
24. Own a generated `sdkwork-knowledgebase-internal-sdk` from
    `sdkwork-knowledgebase-internal-api` for split-topology provider validation, route resolution,
    pinned stream open, navigation/search and generation reconciliation. Standalone topology uses
    the equivalent typed Rust port. Raw HTTP/manual auth and direct cross-service database reads are
    forbidden.
25. Separate Drive source checkpoint, provider-wide generation, per-route page public version,
    navigation generation, search generation and Deploy SiteRevision policy generation. Private
    processing must not invalidate the public Wiki; an ordinary page edit invalidates only affected
    routes and required navigation/search snapshots.
26. Emit versioned Knowledgebase provider events atomically with public state:
    `knowledgebase.wiki.provider.changed.v1`, `knowledgebase.wiki.route.changed.v1`,
    `knowledgebase.wiki.route.revoked.v1`, `knowledgebase.wiki.navigation.changed.v1`, and
    `knowledgebase.wiki.search.changed.v1`. Web Server consumes these events directly; Deploy is not
    the ordinary content-update hot path.
27. Keep implementation status blocked until the linked integration-readiness review closes every
    P0 finding. Ingestion success, private preview, a proposed contract, or a planned certificate
    renewal must not be represented as public/realtime publication success.

## Implemented Evidence (2026-07-23)

- Knowledgebase creation now performs the complete bounded initialization chain: canonical
  publication, `sources/raw` scope, immutable binding, Drive checkpoint, event-delivery
  registration/verification, then `READY`. Activation no longer depends on test or operator SQL.
- The worker applies Drive events and then claims the same bounded checkpoint page's source
  projections with lease/fence protection. Native Markdown/MDX, sanitized HTML, and escaped text
  pages derive canonical routes and become `READY`; JavaScript, SVG, WebAssembly, pin mismatches,
  and exhausted failures remain non-public and become retryable errors or `QUARANTINED`.
- `AUTO_PUBLIC_AFTER_CHECKS` now invokes the same optimistic page publish command with a configured
  tenant-local service actor. Review-required and private-default publications remain `READY` but
  unpublished, and an optimistic auto-publish conflict is reclaimable without another Drive event.
- Public native pages are materialized as sanitized `text/html; charset=utf-8`; opaque handles bind
  the exact rendered SHA-256, byte length, media type, public page version, and Drive version, all
  of which are revalidated on content open.
- A focused cross-repository contract compiles a real Deploy Site/runtime set, activates it in Web,
  executes desktop/mobile Wiki routes through the Knowledgebase adapter/fake generated-SDK
  boundary, fails private/unpublished routes closed, and observes updated content with the same
  SiteRevision, runtime generation, and snapshot hash.
- These closures do not complete isolated PDF/Office/presentation/spreadsheet/media conversion,
  Drive-backed rendition upload, large-object streaming/Range, rendition full-text search,
  Deploy-to-Web deployed-service E2E/SLO evidence, managed TLS/ACME evidence, or user/admin UI
  workflows.

## Acceptance Criteria

- State-machine, permission, optimistic concurrency, route uniqueness, and dual-engine store tests
  pass.
- One-to-one provisioning/backfill and multi-Site resource reuse tests pass without duplicate
  publications or accidental public activation.
- Bulk upload/projection/render/search/assets and review/auto/schedule/unpublish/restore E2E pass.
- Private/unlisted semantics, reserved-root, traversal, raw HTML/script/active asset, SSRF, cache,
  search-index, and cross-tenant tests pass.
- Ordinary provider lifecycle changes generate no Knowledgebase SiteRelease, Deploy Release,
  Deployment, or SiteRevision.
- Provider event duplicate/order/gap/replay and read-through freshness tests pass.
- Existing-path edit creates a new immutable Drive version, preserves stable node identity, and
  passes concurrent update, rename/move, delete/restore, rollback and old/new route invalidation.
- Drive input and Knowledgebase output AsyncAPI schemas, generated internal SDK ownership,
  standalone/cloud parity, and component event inventories pass contract checks.
- Native auto-public Drive-commit-to-public, explicit-publish-to-public and priority-revocation
  p95/p99 targets pass under production-like event/cache load; conversion-required formats report
  their processor-class SLO instead of claiming synchronous realtime publication.
- Deploy descriptor tests prove resource/provider identity separation, WIKI Mount compatibility,
  revision-trigger ownership, contract-version fencing, and absence of secret topology.
- Web Server tests prove unified provider-port and error compatibility, full cache-key isolation,
  move invalidation, durable checkpoint/gap reconciliation, and last-known-good behavior.
- Format detection rejects extension/MIME/signature mismatch; native, HTML, PDF, Office,
  presentation, spreadsheet, source-code, media, archive, and unsupported-binary policies have
  contract and E2E evidence.
- Rendition workers prove sandboxing, bounded conversion, timeout, malformed-output, macro,
  decompression-bomb, active-content, cleanup, rebuild, and processor-version rollback behavior.
- PDF page text, presentation slides/notes, document text, bounded spreadsheet cells, source code,
  media metadata, and approved transcripts participate in visibility-filtered search without
  leaking private source bytes.
- First upload, private preview, explicit single/bulk publish, scheduled publish, auto-public,
  published-file update awaiting republish, old-public-version retention, atomic version switch,
  and priority revocation have state-machine and UI E2E coverage.
- User/admin UI covers all states, problems, bulk commands, quotas, integrations, and audit using
  generated SDKs.
- Release-oriented REQ/ADR/PLAN/MIG artifacts are explicitly superseded and prelaunch schema/API/SDK
  implementation follows the approved migration before merge.

## Trace

- PRD: `docs/product/prd/PRD-live-wiki-publication.md`
- Decision: `docs/architecture/decisions/ADR-20260721-live-mounted-wiki-publication.md`
- Architecture: `docs/architecture/tech/TECH-live-wiki-resource-provider.md`
- Migration: `docs/migrations/MIG-2026-0721-release-to-live-wiki-publication.md`
- Machine contract: `specs/live-wiki-publication.spec.json`
- Readiness review: `docs/engineering/reviews/REVIEW-20260721-live-wiki-deployment-integration-readiness.md`
- Deploy authority: `sdkwork-deployments/docs/architecture/tech/TECH-cloud-site-publishing-control-plane.md`
- Web Server authority: `sdkwork-webserver/docs/architecture/tech/TECH-cloud-site-delivery-data-plane.md`
