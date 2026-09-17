# Audit Event Retention and GDPR Operations

Status: prelaunch; retention automation pending
Owner: SDKWork Knowledgebase operators  
Related: [tenant-isolation.md](tenant-isolation.md), [backup-restore.md](../../deployments/runbooks/backup-restore.md)

## Scope

Tables:

- `kb_audit_event` — domain audit trail (visibility, members, admin mutations)
- `framework_audit_event` — framework HTTP audit persistence

## Retention targets

| Environment | Retention | Action |
| --- | --- | --- |
| Production | 365 days | Automated purge required before launch; not implemented |
| Staging | 90 days | Automated purge required before launch; not implemented |
| Development | 30 days | Operator-managed cleanup only |

These values are policy targets, not evidence of enforcement. This repository does not currently
ship a canonical purge operation, scheduler, retention metrics, or purge drill. Do not represent
audit retention as automated until the purge implementation, legal-hold behavior, bounded batch
deletion, PostgreSQL execution evidence, monitoring, and recovery drill are checked in and attached
to the release candidate. This remains a commercial launch blocker.

## GDPR export (tenant data subject request)

Use the backend compliance API (requires `knowledge.platform.manage`):

```http
POST /backend/v3/api/knowledge/compliance/audit_events/export
Content-Type: application/json

{ "actorId": "<iam_subject_id>" }
```

- OpenAPI operation: `compliance.auditEvents.export.create`
- Backend SDK: `client.knowledge.compliance.auditEvents.export.create({ actorId })`
- Response envelope: `SdkWorkApiResponse` with `data.item.items[]` (`KnowledgeAuditEventItem`)
- Tenant scope is derived from the authenticated principal; do not pass `tenant_id` in the body.
- A synchronous export is complete only when the actor has at most 5,000 matching domain audit
  events. The store probes one additional row and returns HTTP `413` with
  `audit_export_limit_exceeded` instead of silently truncating a larger result.
- No paginated or asynchronous audit-export contract exists today. Subjects above the bound require
  a future cursor-based or job-based export API, generated SDK support, bounded archive storage, and
  expiry/audit controls before the DPO workflow can be completed.

Deliver the exported archive through the platform DPO workflow.

## GDPR delete (right to erasure)

Use the backend compliance API to anonymize actor identifiers while retaining event type and timestamps:

```http
POST /backend/v3/api/knowledge/compliance/audit_events/anonymize_actor
Content-Type: application/json

{ "actorId": "<iam_subject_id>" }
```

- OpenAPI operation: `compliance.auditEvents.anonymizeActor.create`
- Backend SDK: `client.knowledge.compliance.auditEvents.anonymizeActor.create({ actorId })`
- Response envelope: `SdkWorkApiResponse` with `data.item.anonymizedCount`
- Rows are updated to `actor_id = 'gdpr-redacted'`, `actor_type = 'system'`
- The current operation targets `kb_audit_event`; `framework_audit_event` lifecycle handling remains part
  of the pending retention/legal-hold implementation.

Before invoking anonymization:

1. Verify legal basis and scope with platform legal.
2. Do not delete aggregate billing counters; redact PII in structured logs per log retention policy.

## Verification

- Security tests assert durable `kb_audit_event` persistence, fail-closed decoding, and explicit
  synchronous export overflow behavior.
- `pnpm test:tenant-quota` asserts OpenAPI, SDK, and runbook alignment for quota and GDPR compliance APIs.
- Production topology uses `SDKWORK_KNOWLEDGEBASE_LOG_FORMAT=json` for log pipeline correlation with `x-request-id`.
- Release evidence must include the PostgreSQL non-owner/RLS export and anonymization paths; SQLite
  tests alone are not production evidence.
- PostgreSQL still lacks the SQLite-equivalent composite index on
  `(tenant_id, organization_id, actor_id, created_at DESC, id DESC)`. A reviewed migration,
  `EXPLAIN (ANALYZE, BUFFERS)` evidence, and concurrent export/anonymization load evidence are
  required before these operations are commercial-production ready.
