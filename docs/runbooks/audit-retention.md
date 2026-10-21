# Audit Event Retention, Legal Hold, and GDPR Operations

Status: active — retention automation, legal hold enforcement, and purge drill are implemented
Owner: SDKWork Knowledgebase operators
Related: [tenant-isolation.md](tenant-isolation.md), [audit-investigation.md](audit-investigation.md),
[backup-restore.md](../../deployments/runbooks/backup-restore.md)

## Scope

| Table | Owner | Retention |
| --- | --- | --- |
| `kb_audit_event` | this module | automated purge by the worker `audit_retention` phase |
| `framework_audit_event` | `sdkwork-web-framework` | framework-owned; this module does not purge it |

`framework_audit_event` is created by the framework lifecycle and is not part of the Knowledgebase
baseline. Its retention must be configured and evidenced in the framework deployment; this runbook
covers only `kb_audit_event`.

## Retention policy

| Environment | Default retention | Configuration |
| --- | --- | --- |
| Production | 365 days | `SDKWORK_KNOWLEDGEBASE_AUDIT_RETENTION_DAYS=365` |
| Staging | 90 days | `SDKWORK_KNOWLEDGEBASE_AUDIT_RETENTION_DAYS=90` |
| Development | 30 days | `SDKWORK_KNOWLEDGEBASE_AUDIT_RETENTION_DAYS=30` |

Retention days are bounded to `30..=3650`; the batch size is bounded to `1..=1000` and defaults to
500. The worker binary always configures the phase; an invalid value fails worker startup instead of
silently disabling retention.

## Enforcement

- Phase: `audit_retention` in `sdkwork-knowledgebase-worker`, run on every maintenance tick under the
  per-phase time budget.
- Query: one `DELETE` over `(tenant_id, organization_id, created_at, id)` ordered by the oldest
  `created_at, id`, limited to the batch size. A crash cannot leave a half-selected batch; a rerun
  continues with the next oldest window.
- Legal hold: events whose `actor_id` matches an active `actor` hold, or whose `resource_id` matches
  an active `resource` hold, are skipped by the same statement.
- Observability: `knowledgebase_audit_events_purged_total` counter and a structured
  `audit retention purge deleted expired events` log line with `purged`, `cutoff`, and
  `retention_days`.
- Recovery: the purge is forward-only. If an operator needs evidence after deletion, restore the
  database snapshot (backup-restore runbook) and place a legal hold before the next worker tick.

## Legal hold operations

Backend admin API (requires `knowledge.platform.manage`, dual-token, mutation-audited):

```http
POST /backend/v3/api/knowledge/compliance/audit_legal_holds
Content-Type: application/json

{ "subjectType": "actor", "subjectId": "<iam_subject_id>", "reason": "<case reason>", "caseReference": "CASE-123" }
```

- `subjectType` is `actor` or `resource`; `subjectId` is the actor id or numeric resource id.
- Response: HTTP `201`, `data.item` is the created hold (`status = "active"`).

```http
GET /backend/v3/api/knowledge/compliance/audit_legal_holds?page_size=50&cursor=<opaque>
```

- Opaque cursor pagination, `page_size` 1..=200; `data.items` + `data.pageInfo.nextCursor`.

```http
POST /backend/v3/api/knowledge/compliance/audit_legal_holds/{holdId}/release
```

- Response: HTTP `201` with the released hold (`status = "released"`, `releasedBy`, `releasedAt`).
- Releasing an unknown or already released hold returns HTTP `409 audit_legal_hold_conflict`; no
  silent success.

OpenAPI operations: `compliance.auditLegalHolds.create`, `.list`, `.release.create`. Generated
backend SDKs expose the same operations. Holds are scoped to the deployment tenant and organization;
they never cross the RLS boundary.

## GDPR export (tenant data subject request)

```http
POST /backend/v3/api/knowledge/compliance/audit_events/export
Content-Type: application/json

{ "actorId": "<iam_subject_id>" }
```

- OpenAPI operation: `compliance.auditEvents.export.create`
- Response envelope: `SdkWorkApiResponse` with `data.item.items[]` (`KnowledgeAuditEventItem`)
- Tenant scope is derived from the authenticated principal; do not pass `tenant_id` in the body.
- A synchronous export is complete only when the actor has at most 5,000 matching domain audit
  events. The store probes one additional row and returns HTTP `413` with
  `audit_export_limit_exceeded` instead of silently truncating a larger result.
- No paginated or asynchronous audit-export contract exists today. Subjects above the bound require
  a cursor-based or job-based export API, generated SDK support, bounded archive storage, and
  expiry/audit controls.

## GDPR delete (right to erasure)

```http
POST /backend/v3/api/knowledge/compliance/audit_events/anonymize_actor
Content-Type: application/json

{ "actorId": "<iam_subject_id>" }
```

- OpenAPI operation: `compliance.auditEvents.anonymizeActor.create`
- `data.item.anonymizedCount` reports the number of redacted rows.
- Rows become `actor_id = 'gdpr-redacted'`, `actor_type = 'system'`; event type, resource, and
  timestamps are retained.
- Place a legal hold **before** erasure when retention obligations conflict; otherwise anonymization
  removes the actor linkage the hold would match.
- `framework_audit_event` erasure remains a framework-owned operation.

## Verification

- `cargo test -p sdkwork-intelligence-knowledgebase-repository-sqlx --test postgres_optional_integration`
  covers the retention/legal-hold drill: an expired unheld event is deleted, an equally old held
  event survives, and release restores normal retention.
- `hosted_backend_audit_legal_hold_lifecycle_is_operator_scoped` covers the HTTP lifecycle
  (create/list/release/duplicate-release conflict) against PostgreSQL.
- The audit subject composite index `idx_kb_audit_event_scope_actor_created`
  `(tenant_id, organization_id, actor_id, created_at DESC, id DESC)` and the purge index
  `idx_kb_audit_event_scope_created_id` are part of the baseline and migrations.
- Release evidence must include a production-like retention run and the counter/log line; SQLite
  fixture tests are schema-parity evidence, not production evidence.
