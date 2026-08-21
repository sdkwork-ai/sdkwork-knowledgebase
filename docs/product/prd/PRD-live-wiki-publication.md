# SDKWork Knowledgebase Live Wiki Publication PRD

Status: implementation active; production evidence blocked
Owner: SDKWork Knowledgebase maintainers
Application: sdkwork-knowledgebase
Updated: 2026-07-23
Requirement: REQ-2026-0721
Parent: [PRD.md](PRD.md)
Machine contract: `specs/live-wiki-publication.spec.json`
Specs: REQUIREMENTS_SPEC.md, DOCUMENTATION_SPEC.md, DRIVE_SPEC.md, DATABASE_SPEC.md,
API_SPEC.md, SDK_SPEC.md, APP_SDK_INTEGRATION_SPEC.md, SECURITY_SPEC.md,
PRIVACY_SPEC.md, PERFORMANCE_SPEC.md, OBSERVABILITY_SPEC.md, TEST_SPEC.md,
MIGRATION_SPEC.md, RELEASE_SPEC.md, MEDIA_RESOURCE_SPEC.md, SUPPLY_CHAIN_SECURITY_SPEC.md

## 1. Purpose

Make every Knowledgebase Wiki-capable through one canonical, Knowledgebase-owned
`WikiPublication` whose fixed source root is the Drive directory `sources/raw`. The publication is
provisioned in `DRAFT`/`PRIVATE`, then initialization binds and verifies the root/checkpoint/event
delivery before converging to `READY`; none of these states makes content public. Authors upload Wiki pages,
documents, presentations, spreadsheets,
source code, media, archives, and website assets through Drive, manage each file's processing,
publication, visibility, index, route, and public-version state in Knowledgebase, and make eligible
changes available without building or deploying a frozen release for every edit.

The Wiki is public only after the Knowledgebase has `publicationType=wiki`, `wikiStatus=ACTIVE`, an
active Deploy `KNOWLEDGEBASE_WIKI` resource, WIKI Mount, verified Binding, and an active
`SiteRevision`. `sources/raw` is the only eligible public source tree; it is not an unconditional
public ACL for every file. Standalone and cloud traffic both enter through Web Server's compiled
descriptor and WIKI handler; Knowledgebase owns no anonymous fixed public route.

### 1.1 Current Delivery Boundary

The canonical initialization, Drive event projection, native source processor, explicit and
automatic publication commands, bounded safe HTML representation, generated Internal SDK provider,
and Web Server WIKI adapter are implemented and covered by focused executable tests. Native means
Markdown/MDX, sanitized HTML, escaped text pages, and passive assets within the current bounded
reader contract. Scheduled publication, isolated multi-format conversion and Drive-backed
renditions, large-object Range/streaming, rendition full-text search, complete user/admin workflows,
managed TLS evidence, and a deployed Deploy-to-Web end-to-end freshness/load certification remain
release blockers. The product must not be described as commercially or production ready while
those gates remain open.

A focused cross-repository contract test now uses real Deploy compiler output, Web activation and
device routing, and the Knowledgebase provider/fake generated-SDK boundary. It proves public Wiki
delivery, private/unpublished failure closure, and live content refresh without a new Deploy
Release, Deployment, SiteRevision, runtime generation, or snapshot. This is implementation evidence,
not deployed SLO or launch evidence.

## 2. Problem

Large knowledgebases may contain thousands of Markdown files, images, attachments, styles, fonts,
and other static assets. Authors expect uploaded eligible content to appear quickly and want to
control draft/review/published/private state per file. Rebuilding an immutable HTML release after
every upload creates unnecessary latency, storage duplication, release state, cleanup, and rollback
complexity. It also makes the source tree and the public Wiki diverge.

Serving `sources/raw` directly as ordinary static files is also insufficient: Markdown needs safe
rendering, canonical routes, navigation, search, redirects, SEO, link/asset resolution, page state,
and privacy enforcement. Knowledgebase must remain the semantic authority while Deploy/Web Server
own host/path/TLS/request delivery.

## 3. Users

| Persona | Outcome |
| --- | --- |
| Knowledge author | Upload/edit files and see publication readiness clearly. |
| Reviewer/publisher | Review, schedule, bulk publish, unpublish, and recover pages. |
| Knowledgebase owner | Enable Wiki, configure structure/theme/SEO, and connect domains. |
| Tenant administrator | Govern permissions, quotas, retention, audit, and public exposure. |
| Public reader | Browse, search, and link to a secure, fast, consistent Wiki. |
| Knowledgebase admin/operator | Operate ingest, projection, index, renderer, events, and incidents. |
| Deploy/Web Server | Validate and resolve a typed live Wiki resource. |

## 4. Goals

- Make `sources/raw` the fixed Drive-backed Wiki source root and preserve its directory hierarchy.
- Support bulk folders and all governed Wiki source formats through the canonical Drive Uploader.
- Support a complete governed Wiki format matrix for Markdown/text, safe HTML, PDF and office
  documents, presentations, spreadsheets, source code, images, audio, video, fonts, archives, and
  downloadable files.
- Maintain processing, publication, visibility, and search-index state per source file.
- Offer review-required and explicitly authorized auto-public-after-checks workflows.
- Publish eligible changes live without Knowledgebase SiteRelease creation.
- Advance public content versions/resource generations for provider lifecycle changes without a
  Deploy Release, Deployment, or SiteRevision.
- Provide professional Wiki routes, rendering, navigation, search, redirects, assets, SEO, locale,
  cache validators, and accessibility.
- Expose one stable `KNOWLEDGEBASE_WIKI` provider resource to Deploy/Web Server.
- Keep domains, client Variants, Mounts, TLS, delivery policy, and public traffic metering in Deploy.
- Provide complete author/owner/tenant-admin/platform-admin views and commercial quotas.

## 5. Non-Goals

- Expose `okf/`, `output/`, `.sdkwork/`, governance, internal index, or provider storage paths.
- Make every uploaded file public regardless of state, visibility, scan, or format.
- Create an immutable SiteRelease or copy rendered HTML/assets to Drive for every content change.
- Store object keys, buckets, presigned URLs, or transient download URLs in Wiki business state.
- Let Knowledgebase own custom domain verification, certificate lifecycle, or Web Node rollout.
- Add a Knowledgebase-owned anonymous `/wiki/...` route or bypass the shared descriptor/Mount model
  in standalone deployments.
- Execute arbitrary server code, build scripts, browser JavaScript, raw HTML, or untrusted theme
  templates by default.
- Treat upload support as permission to execute JavaScript, macros, formulas, active HTML, active
  SVG, service workers, WebAssembly, or arbitrary CSS on the standard Wiki origin.
- Use User-Agent/device selection as authorization.

## 6. Eligibility And State Model

### 6.1 Knowledgebase Publication

Publication states are `DRAFT`, `VALIDATING`, `READY`, `ACTIVE`, `DEGRADED`, `PAUSED`, `ARCHIVED`,
and `FAILED`. `publicationType` is `wiki` for this product. Only `ACTIVE` is provider-eligible.

Every Knowledgebase has exactly one canonical WikiPublication, created idempotently during
Knowledgebase provisioning or backfilled before the feature is enabled. No special Knowledgebase
type or conversion is required. The publication references the Knowledgebase Space, its Drive
`knowledge_base` Space, and the fixed `sources/raw` folder UUID. It does not own a domain or
certificate.

The same WikiPublication may be referenced by multiple authorized Deploy Site Resources, Sites,
Variants, Mounts, and domains. Those connections never create another WikiPublication or copy
content. A draft/paused publication may be selected for authenticated setup/preview, but public Site
activation fails closed until provider validation returns `ACTIVE`.

### 6.2 Source File Dimensions

Each file has independent dimensions:

| Dimension | States |
| --- | --- |
| `sourceState` | `DISCOVERED`, `QUEUED`, `PROCESSING`, `READY`, `ERROR`, `QUARANTINED`, `DELETED` |
| `publicationState` | `DRAFT`, `IN_REVIEW`, `SCHEDULED`, `PUBLISHED`, `UNPUBLISHED`, `ARCHIVED` |
| `visibility` | `PRIVATE`, `UNLISTED`, `PUBLIC` |
| `indexState` | `NOT_REQUIRED`, `PENDING`, `INDEXING`, `READY`, `ERROR` |
| `fileKind` | `PAGE`, `DOCUMENT`, `PRESENTATION`, `SPREADSHEET`, `CODE`, `MEDIA`, `ASSET`, `ARCHIVE` |

A page can switch to a new anonymous representation only after that exact Drive version reaches
READY and passes every publication gate. Runtime resolution uses the pinned
`publicDriveVersionUuid`: publication must be PUBLISHED, visibility PUBLIC or UNLISTED, the
WikiPublication ACTIVE, and the pinned snapshot still security/policy eligible. Under
`KEEP_LAST_PUBLIC_UNTIL_READY`, a newer current source may be DISCOVERED, PROCESSING, or ERROR
without removing the prior pinned public snapshot. UNLISTED is not shown in
navigation/sitemap/search but remains accessible by URL; it is not an authorization boundary.
PRIVATE is never anonymously resolvable, and quarantine/delete/private/unpublish revokes the
pinned snapshot atomically.

Assets use the same source/publication/visibility checks. A public page cannot make a private asset
public by reference.

## 7. Source And Route Semantics

- `sources/raw/index.md` maps to the Wiki root unless an explicit eligible homepage is configured.
- `sources/raw/guides/install.md` maps to `/guides/install/` under the WIKI Mount.
- `sources/raw/manual.pdf` maps to a Wiki document page such as `/manual/`; the original PDF is a
  separate policy-checked pinned-version download representation, not a public Drive path.
- Page-capable processor profiles declare which terminal extension is removed. Two source files
  such as `manual.md` and `manual.pdf` cannot silently claim the same canonical route; the conflict
  blocks publication until an explicit reviewed route override resolves it.
- Canonical routes are normalized, extensionless, slash-consistent, locale-aware, and conflict-free.
- Direct `.md` requests redirect to the canonical route or return not found according to policy.
- Directory index uses `index.md`; sibling route collisions and case/canonical collisions block
  publication readiness.
- Renames/moves create an optional reviewed redirect from the previous canonical route; redirects
  are bounded, non-cyclic, and expire or persist by policy.
- Relative Markdown links/assets resolve against the source page directory and cannot leave
  `sources/raw`.
- Dotfiles, backup files, `okf`, `output`, `.sdkwork`, and reserved internal names are denied even if
  guessed through encoding/case tricks.

### 7.1 Shared Site Platform Contract

Deploy is the only Site control plane. Its `sdkwork.website-runtime.v1` descriptor owns Site,
Binding, Variant, WIKI Mount, Site Resource, delivery/security policy, and rollout. The descriptor
uses a Deploy-owned `resourceUuid` for routing and observations, plus a distinct
`providerResourceUuid` containing the Knowledgebase `WikiPublication` UUID. Provider Space/root
identities and a required contract version are stable references; endpoints, tokens, storage keys,
presigned URLs, and credentials never enter the descriptor.

Web Server resolves exact/wildcard Binding, longest Binding path, Variant, and longest WIKI Mount
before calling Knowledgebase through its unified provider adapter. Knowledgebase owns public Wiki
eligibility and maps to the shared validate, route resolution, content open, search, and event
subscription semantics. Public headers never supply tenant/resource scope.

Changing a domain, Binding, Variant, Mount, Site Resource, or delivery/security policy creates a
Deploy configuration revision. Uploading, processing, publishing, changing visibility, updating
theme/navigation/search, or revoking Wiki content changes provider versions/generations and cache
events only. Those ordinary Wiki changes never create a Deploy Release, Deployment, or
`SiteRevision`; TLS snapshots are independent as well.

### 7.2 File Format Capability Matrix

The raw source layer accepts more formats than the OKF bundle itself. OKF v0.1 concepts remain
Markdown under `okf/`; the following is the SDKWork Knowledgebase `sources/raw` publication profile.

| Profile | Typical files | Public Wiki behavior | Search behavior | Execution policy |
| --- | --- | --- | --- | --- |
| Native page | `.md`, `.markdown`, `.txt`, `.rst`, `.adoc`, `.mdx` | Versioned text renderer; MDX JSX/components are disabled | Body, headings, front matter, links | No code execution |
| Safe HTML page | `.html`, `.htm` | Parse DOM, remove active content, rewrite eligible local links/assets, sanitize to approved HTML | Sanitized visible text and headings | Scripts, handlers, embeds and active URLs removed |
| Document | `.pdf`, `.doc`, `.docx`, `.odt`, `.rtf`, `.epub` | PDF/document viewer, thumbnails and original download; office files get an isolated PDF/page rendition | Page/document text with page references | Macros and embedded active content never execute |
| Presentation | `.ppt`, `.pptx`, `.odp`, `.key` | Slide viewer from verified slide-image/PDF rendition, notes panel when policy allows, original download | Slide text, titles and approved notes | Transitions, macros and embedded scripts never execute |
| Spreadsheet | `.xls`, `.xlsx`, `.ods`, `.csv`, `.tsv` | Bounded read-only sheet/table preview and original download | Sheet names and bounded cell text | Formulas are displayed as values/text and never evaluated by Wiki runtime |
| Source code | `.js`, `.ts`, `.tsx`, `.jsx`, `.css`, `.json`, `.yaml`, `.xml`, `.toml`, `.sql`, `.py`, `.rs`, `.java`, and other approved text code | Escaped syntax-highlighted view with line anchors, copy and download | Bounded source text | Never executed on the standard Wiki origin |
| Media | images, audio and video | Approved responsive viewer, metadata, poster/thumbnail and range delivery | Metadata, alt text and approved transcript/OCR | No embedded active document execution |
| Archive | `.zip`, `.tar`, `.gz`, `.7z`, `.rar` | Attachment download and optional bounded safe member manifest | File name and approved manifest only | Members are never implicitly rendered or executed |
| Unsupported binary | executables, libraries, installers and unknown signatures | Quarantine or explicitly approved attachment-only workflow | Metadata only | Never executed or inline rendered |

Extension, declared MIME, detected signature, checksum, scan state, and selected processor must
agree. A mismatch blocks public readiness and creates an author-visible problem. The standard does
not use a filename extension alone as a security decision.

Metadata precedence is explicit Knowledgebase projection override, validated native front matter,
sanitized embedded PDF/Office/media metadata, then normalized filename fallback. Authors can edit
title, description, canonical route, locale, navigation placement, alt text, and download label in
the Source Explorer without rewriting binary files. Publication state, visibility, permissions,
scan state, and execution policy are never elevated by embedded document metadata.

## 8. Upload And Publication Workflows

The standard separates author visibility from anonymous public visibility:

| Moment | Author workspace | Private preview | Public Wiki |
| --- | --- | --- | --- |
| Upload starts | immediate byte progress | unavailable | unchanged |
| Drive upload completes | source row appears immediately | processing | unchanged |
| Scan/parse/render/index gates complete | READY with problems or success | available | unchanged in review-required mode |
| Explicit publish succeeds | PUBLISHED | pinned published preview | new public version within freshness SLO |
| Owner-approved auto-public gates complete | PUBLISHED by attributed system actor | available | new public version within freshness SLO |

This author-immediate/public-gated model is the default. It gives authors instant feedback without
making an accidental upload, malware, private document, broken route, failed conversion, or
unfinished index publicly visible.

### 8.1 Bulk Upload

The browser obtains `sources/raw` as the authorized upload parent and uses the generated Drive app
SDK/composed uploader for files/folders. Folder-relative paths are preserved. Knowledgebase receives
stable Drive Space/node/version references through an ingest/projection command or event; it never
receives object-storage identity.

Large uploads are asynchronous and resumable. The file list immediately shows DISCOVERED/QUEUED and
progresses through processing, scan, parse, link analysis, render validation, and index state.
The source row is author-visible as soon as Drive commits stable identity; this is not a public
publication event.

### 8.2 Review Required

This is the default. New source files enter DRAFT and PRIVATE unless an authorized import rule
specifies otherwise. Authors edit metadata; reviewers inspect render, links, assets, sanitization,
route conflicts, and policy. Once ready, the file shows private preview plus `AWAITING_PUBLISH`.
Publishers perform a version-fenced single, bounded bulk, or scheduled publication. A bulk command
publishes only the ready selected subset and returns a per-item outcome; it never silently skips or
publishes failed/quarantined items.

### 8.3 Auto Public After Checks

An owner may enable `AUTO_PUBLIC_AFTER_CHECKS` only with explicit permission and an acknowledged
default visibility. Eligible files become PUBLISHED/PUBLIC after upload commit, required scanning,
projection, parse/render validation, and index readiness. Errors/quarantine remain non-public and
are visible in the Problems view. A generic Drive upload never silently activates this mode.
The automatic workflow invokes the same publish command and optimistic version checks using an
attributed system actor. Policy can be scoped only through an explicit site/import-rule contract;
filename, folder placement, embedded metadata, or uploader identity cannot enable it implicitly.

### 8.4 Updates And Unpublish

A logical source file is mutable through immutable Drive versions. `driveNodeUuid` remains the
stable file identity; every successful content edit or replacement creates a new immutable
`driveVersionUuid` and atomically advances the node current version. Rename or same-root move keeps
the node identity and reconciles both old and new routes. Delete/restore is version-fenced. A
storage adapter must not reject an edit merely because the `sources/raw` logical path already
exists; only a committed physical version/blob is immutable.

A new Drive version re-enters processing while the current public-version policy is explicit:

- `KEEP_LAST_PUBLIC_UNTIL_READY` keeps the prior verified public representation until the new
  version is ready. In `REVIEW_REQUIRED`, readiness produces `UPDATE_AWAITING_PUBLISH` and the old
  public version stays active until explicit republish. In `AUTO_PUBLIC_AFTER_CHECKS`, the same
  readiness gates invoke the attributed system publish command and switch atomically;
- `UNPUBLISH_DURING_PROCESSING` removes the page until the new version is ready.

Security/quarantine/private/deleted transitions override stale policy and invalidate public cache
immediately. Unpublish does not delete source content. Upload, processing, publish, republish,
unpublish, visibility, route-content mapping, theme, renderer, navigation, search, quarantine,
delete, and restore remain provider lifecycle operations: they advance the narrowest per-page,
navigation, search, or provider-wide generation and never create a Deploy Release, Deployment, or
SiteRevision. A SiteRevision
is compiled only for Deploy-owned Site/resource composition, Variant, Mount, Binding, delivery,
security, limit, or observability configuration changes.

### 8.5 Multi-Format Processing And Renditions

Native text formats render directly through a bounded parser. PDF, office documents,
presentations, spreadsheets, thumbnails, posters, OCR, transcripts, and archive manifests may need
derived renditions. Conversion runs asynchronously in an isolated worker or approved converter
service with no ambient credentials, no private-network access, read-only input, bounded temporary
storage, explicit timeout, and output validation.

Every rendition is keyed by source Drive version, source checksum, processor id/version, policy
version, rendition kind, and output checksum. It is written through the Drive server-side uploader
and represented by stable Drive/`MediaResource` identity. Renditions are rebuildable cache and
presentation state; they are not source authority, SiteRelease artifacts, or process-local durable
files.

Processor profiles declare supported MIME/extensions, input/output limits, sandbox, network policy,
timeout, emitted rendition types, and text-extraction behavior. Processor upgrades use bounded
reprocessing, compatibility tests, canary, rollback, and cleanup of expired derived artifacts.

## 9. Wiki Reader Experience

- responsive article layout with accessible heading hierarchy, table of contents, breadcrumbs, and
  previous/next navigation;
- configurable navigation tree derived from folders/front matter or curated order;
- full-text search over published public pages with highlights and bounded pagination;
- title, description, author policy, updated time, locale, canonical URL, Open Graph, sitemap, and
  robots controls;
- sanitized Markdown features: headings, code, tables, task lists, footnotes, diagrams only through
  approved renderers, and safe links/images;
- PDF/document page viewer, presentation slide viewer, bounded spreadsheet preview, escaped
  source-code view, image gallery, audio/video player, and attachment download states;
- asset delivery for images, documents, CSS, fonts, and other allowed static types;
- theme tokens and approved template packages with versioned renderer identity;
- print view, copy-link, anchor links, error/not-found, empty-search, and offline/degraded messaging;
- conditional requests, cache validators, and canonical redirects.

Knowledgebase may store all website static resources in `sources/raw`, but public execution is
policy-controlled. Scripts, raw HTML, active SVG, remote embeds, and arbitrary CSS are blocked by
default. A future trusted-active-asset profile requires isolated origin/CSP and separate security
approval; upload support alone does not imply execution permission.

The trusted-active-site profile is not an option on the standard Wiki origin. It requires an
isolated origin, signed versioned packages, dependency inventory, SBOM/provenance, vulnerability
review, strict CSP, no shared SDKWork credentials, rollout/rollback evidence, and explicit security
approval before any user-supplied JavaScript or active HTML can execute.

## 10. Author And Owner Interface

### 10.1 Wiki Overview

Every Knowledgebase shows a Wiki capability entry. Before activation it presents setup readiness and
an `Enable Wiki` workflow; after activation it shows status, all connected Sites/Mounts/public URLs
from Deploy, source/root health, page/asset counts by state, last content change/freshness,
search/index health, certificate/domain summary, usage/quota, and problems.

### 10.2 Source Explorer

| Column/control | Behavior |
| --- | --- |
| Path/type | folder hierarchy, format/profile icon, canonical route |
| Source | Drive version, checksum, scan, source state |
| Publication | draft/review/scheduled/published/unpublished/archived |
| Readiness | uploading/uploaded/processing/preview-ready/awaiting-publish/update-awaiting-publish/error/quarantined |
| Visibility | private/unlisted/public swatch/control when permitted |
| Index | pending/indexing/ready/error/not-required |
| Quality | broken links/assets, route conflict, sanitizer warnings |
| Activity | updated by/time, last public time, public version |
| Bulk actions | review, publish, schedule, unpublish, visibility, reprocess, reindex |

Filters cover state, visibility, kind, locale, folder, problems, changed since, and assignee. Lists
are server-paginated and support selection across bounded result sets through an asynchronous bulk
command, not a full browser collection.

`Preview` is available only to authorized users and never changes public state. `Publish` and
`Republish` show the exact pinned source version, target visibility, canonical route, blocking
problems, and whether an older public version remains active. Bulk publish requires confirmation,
reports ready/blocked/conflict outcomes per item, and cannot use a stale selection token after a
source or policy version changes.

### 10.3 Page Workspace

Split editor/preview, front matter/metadata, route, visibility, publication schedule, navigation,
links/backlinks, assets, versions/diff, comments/review, search preview, SEO, and audit. Preview uses
the same versioned renderer/sanitizer as public resolution.

### 10.4 Wiki Settings

General identity, homepage, source policy, review/automation, navigation, routes/redirects, theme,
renderer features, locale, search, SEO/robots/sitemap, retention, members/permissions, connected
Deploy Site/Site Resource status, analytics deep link, audit, pause/archive/delete safeguards.

Enabling `AUTO_PUBLIC_AFTER_CHECKS` requires Owner permission, an explicit default visibility,
policy summary, affected-scope preview, confirmation, and audit. Disabling automation never
unpublishes existing content; it sends future ready versions to `AWAITING_PUBLISH`.

Domains, TLS, client Variants, Mount prefixes, delivery headers, and public traffic analytics open
the Deploy console rather than duplicating configuration. The Knowledgebase view shows the connected
Deploy Site Resource identity, active descriptor observation, provider contract compatibility, and
last validation result, but Deploy remains the mutation authority.

## 11. Knowledgebase Admin Views

| View | Purpose |
| --- | --- |
| Wiki publications | tenant/Space/status/root, connected Site, pages/assets, freshness, quota |
| Source projection | queue depth, processing latency, error/quarantine, Drive event checkpoint/gap |
| Renderer/sanitizer | active versions, failure rates, blocked features, compatibility |
| Search/index | backlog, lag, size, errors, rebuild operations |
| Route/link quality | route collisions, redirect cycles, broken links/assets, orphan assets |
| Provider service | validation/resolve/render/asset/search latency, errors, circuit and consumer |
| Quotas/usage | page count, index bytes, render work, retained versions, automation frequency |
| Audit/investigation | publication/visibility/reprocess/reindex/settings/support actions |

Admin operations are reason-coded, permissioned, bounded, and audited. They do not expose Drive
storage credentials or change Deploy domain/certificate state.

## 12. Permissions

Conceptual roles: owner, maintainer, author, reviewer, publisher, analyst, and Knowledgebase platform
administrator. Authors edit/upload but do not publish by default. Reviewers can approve/reject.
Publishers can publish/unpublish/schedule and set public/unlisted within policy. Owners manage Wiki
settings and Deploy connection. Exact permissions come from the application permission manifest and
are enforced at command and store boundaries.

## 13. Commercial Model

Knowledgebase owns entitlement/meter facts for active Wiki publications, projected source files,
published pages, asset bytes attributed to Wiki, index bytes, ingest/render/index operations,
automation frequency, locales, link checks, and Knowledgebase analytics retention. Drive owns stored
and version bytes/uploads. Deploy owns requests/egress/domains/certificates. Commerce owns pricing,
invoices, and payment.

Plan changes do not delete or silently publish content. Over-limit behavior blocks new capacity or
automation according to explicit policy, preserves current public pages when allowed, and shows an
actionable quota state.

## 14. Non-Functional Requirements

| Area | Target |
| --- | --- |
| Content freshness | p95 <= 5 seconds, p99 <= 30 seconds after eligible state commit |
| Author projection | Drive commit to authenticated source state p95 <= 5 seconds |
| Native auto-public | supported small native page/asset Drive commit to public p95 <= 15 seconds, p99 <= 60 seconds when required dependencies are healthy |
| Priority revocation | committed private/quarantine/delete/unpublish to not-public p95 <= 5 seconds; stale public delivery forbidden |
| Route resolution | indexed and bounded; no full file/page collection |
| Render | bounded input/output/time/extensions with cache by source+renderer version |
| Conversion | per-processor size class, timeout, concurrency and queue SLO; no synchronous claim for large/complex files |
| Search | provider-side index and cursor/page bounded results |
| Availability | provider participates in standard publishing data-plane SLO after certification |
| Isolation | zero cross-tenant/Space/page/cache/index disclosure |
| Recovery | projection/index rebuildable from Drive + Knowledgebase authority |

Drive/Knowledgebase event delivery is at-least-once and idempotent. Read-through validation covers
event delay/gap. Public-to-private/deleted/quarantine is prioritized and cannot remain stale.
The content-freshness clock starts when an explicit, automatic, or scheduled publish transition
commits the new eligible public version. Upload completion, conversion completion, and private
preview readiness do not start the public SLO.

Realtime means event-driven bounded eventual visibility without polling-only publication, Deploy
Release, Deployment, or SiteRevision. It is policy-dependent: `REVIEW_REQUIRED` updates author
state and private preview in realtime but waits for publish/republish; `AUTO_PUBLIC_AFTER_CHECKS`
may meet the Drive-commit-to-public target after all gates pass. Large or conversion-required files
use a processor-class asynchronous SLO and never inherit the native realtime claim.

## 15. Security And Privacy

- Service-provider resolution requires authenticated tenant/publication-scoped runtime context.
- Sanitize Markdown/HTML/URLs/assets/front matter and bound nesting, code blocks, diagrams, tables,
  output, CPU, memory, and external fetch behavior.
- Sandbox document/presentation/spreadsheet/archive processors; block macros, formulas, embedded
  executables, decompression bombs, malformed output, private-network access, ambient credentials,
  and unbounded temporary files.
- External URL preview/fetch is absent or SSRF-safe through an approved media/provider capability;
  rendering never fetches arbitrary private network URLs.
- Enforce source-root confinement and route canonicalization at API, service, store, renderer, asset,
  cache, and event boundaries.
- Private/unpublished content, review comments, author identity policy, source paths, and search index
  are classified and excluded from public/telemetry output.
- Public errors are non-disclosing. Logs/audit contain stable IDs/reason codes, not file bodies,
  secrets, tokens, presigned URLs, or unbounded user text.
- Support malware/phishing/abuse report, takedown, appeal, legal hold, and restoration workflows with
  Deploy/Drive coordination before commercial GA.

## 16. Success Metrics

- A large multi-format source directory can be uploaded once and becomes a navigable Wiki without a
  content deployment.
- At least 99% of eligible state commits meet the freshness objective.
- Zero draft/private/quarantined/deleted/cross-tenant page or asset disclosure.
- Search/navigation/route projection can be rebuilt from source authority and versioned settings.
- Authors can identify why any file is not public from one Source Explorer row/detail.
- Public request traces identify publication/page/source/render version without exposing content.

## 17. Acceptance Criteria

- Only active Wiki publications rooted at `sources/raw` validate as `KNOWLEDGEBASE_WIKI` resources.
- Every Knowledgebase has exactly one canonical WikiPublication provisioned DRAFT/PRIVATE,
  initialized through bound `sources/raw` checkpoint/event-delivery verification to READY, and
  transitioned with the Knowledgebase lifecycle;
  idempotent provisioning/backfill, archive/delete, retry, and concurrency tests pass.
- One canonical WikiPublication can be reused by multiple authorized Deploy Sites/Variants/Mounts;
  draft/paused publications remain non-public and no connection creates a duplicate publication.
- Bulk multi-format source upload uses Drive SDK/uploader and persists only stable Drive identities.
- Native page, safe HTML, PDF/document, presentation, spreadsheet, source-code, media, archive, and
  unsupported-binary profiles have upload, projection, preview, search, download, security, and
  failure-state E2E coverage.
- Renditions are Drive-backed, content-addressed, versioned by processor, rebuildable, cleaned up by
  policy, and never become a parallel content authority or SiteRelease.
- Per-file source/publication/visibility/index state and state-transition permissions are complete.
- Review-required, auto-public, scheduled, update policy, unpublish, delete, quarantine, rename/
  redirect, theme/renderer/navigation/search generation, and version restore tests pass without
  creating `kb_site_release`, Deploy Release, Deployment, or SiteRevision.
- Wiki render/navigation/search/SEO/assets/locale/redirect/conditional-cache E2E works under system and
  custom Deploy domains.
- `okf/`, `output/`, `.sdkwork/`, private/draft/unready files and cross-root assets fail closed.
- Provider SDK/service, event/reconciliation, pagination, tenant/cache/index isolation, load/soak,
  backup/rebuild, admin UI, quotas, and audit gates pass.
- Drive input and Knowledgebase output events have accepted AsyncAPI authorities and executable
  producer/consumer compatibility tests; Knowledgebase exposes a generated internal SDK and
  standalone typed-port parity without raw HTTP.
- Provider generation, per-route page public version, navigation/search generation, and Drive
  checkpoint are independently fenced; private work does not cause global public cache churn.
- The linked integration-readiness review has no open P0 finding, Web Server writable control-plane
  routes are retired, and one Deploy control-plane writer is proven before public activation.
- Deploy descriptor tests distinguish `resourceUuid` from `providerResourceUuid`, reject secret
  topology, and prove that only Site configuration changes create `SiteRevision`.
- Web Server contract tests cover the unified provider port, non-disclosing error mapping, complete
  tenant/Site/Binding/Variant/Mount/resource/version cache key, event checkpoint/gap recovery, and
  last-known-good isolation.
- Existing release-oriented site contracts/docs are superseded or migrated; two authoritative Wiki
  publication models cannot remain active.

## 18. Dependencies

- Deploy PRD: `../sdkwork-deployments/docs/product/prd/PRD-cloud-site-publishing-platform.md`
- Web Server architecture: `../sdkwork-webserver/docs/architecture/tech/TECH-cloud-site-delivery-data-plane.md`
- Deploy architecture: `sdkwork-deployments/docs/architecture/tech/TECH-cloud-site-publishing-control-plane.md`
- Web Server architecture: `sdkwork-webserver/docs/architecture/tech/TECH-cloud-site-delivery-data-plane.md`
- Local architecture: [TECH-live-wiki-resource-provider.md](../../architecture/tech/TECH-live-wiki-resource-provider.md)
- Local migration: [MIG-2026-0721-release-to-live-wiki-publication.md](../../migrations/MIG-2026-0721-release-to-live-wiki-publication.md)
- Machine contract: `specs/live-wiki-publication.spec.json`
- Readiness review: [REVIEW-20260721-live-wiki-deployment-integration-readiness.md](../../engineering/reviews/REVIEW-20260721-live-wiki-deployment-integration-readiness.md)
