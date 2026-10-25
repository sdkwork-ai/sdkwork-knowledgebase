# SDKWork Knowledgebase Commercial Release Readiness

Status: prelaunch-gated
Owner: SDKWork maintainers
Application: `sdkwork-knowledgebase`
Updated: 2026-07-31
Parent: [PRD.md](PRD.md)
Prerequisite: [PRD-mvp-launch.md](PRD-mvp-launch.md)

The filename is retained as a compatibility path. This document defines the active commercial
release gates for the dedicated tenant/organization deployment model; it does not authorize or
promise shared request-scoped multi-tenancy.

## Supported Product Boundary

- One dedicated API/worker deployment serves one tenant and one non-zero organization.
- PostgreSQL is authoritative server storage. Server bootstrap rejects file-backed SQLite, and
  in-memory SQLite exists only as a deterministic repository test fixture; client-local storage is
  owned outside the application server.
- API, backend, and worker replicas use fixed tenant/organization guards and dual-column RLS.
- Drive-backed object storage is shared infrastructure but every object authority is scoped.
- Subscription, entitlement, suspension, and payment authority belong to the SDKWork platform.

## Implemented Foundations

- App/backend authorization derives tenant and organization from authenticated context and rejects
  deployment mismatches.
- PostgreSQL connection options bind `app.current_tenant_id` and
  `app.current_organization_id` before the process-shared pool is created.
- The API and internal RPC runtimes reuse one scoped typed PostgreSQL handle for Drive, pgvector, and
  cloud provider resolution, while the remaining compatibility pool shares the same bounded
  per-process connection budget.
- Organization isolation migration adds non-null organization ownership, scoped indexes, composite
  foreign keys, and dual-column forced RLS to legacy business tables.
- Backend list endpoints and SDK consumers use bounded cursor pagination.
- Desktop resource reads, exports, secure values, and provider secret resolution have explicit size
  and concurrency bounds.

These foundations are implementation evidence, not commercial release approval.

## Exit Criteria

### Data Isolation And Correctness

- [ ] Human review approves the organization migration and dedicated-runtime ADR.
- [ ] A machine-readable repository inventory proves every organization-owned read/write binds both
  tenant and organization, including outbox, audit, worker, and administrative paths.
- [ ] Real release PostgreSQL tests prove cross-organization denial for representative repositories,
  RLS enforcement with a non-owner role, migration upgrade, and rollback/recovery procedures.
- [ ] The temporary production `AnyPool` compatibility path is removed in favor of typed `PgPool`
  authoritative repositories.
- [ ] SQLite client/test migrations and PostgreSQL server migrations have explicit parity assertions
  only for the contracts they are both required to implement.
- [ ] Knowledge market publication has an explicit authorized create/review/unpublish workflow;
  catalog reads never auto-publish ordinary spaces or synthesize provider, model, author, or media
  metadata.

### Reliability, Performance, And Memory

- [ ] Outbox claims use owner/token fencing, lease renewal, stale-worker rejection, retry policy,
  idempotency, and a bounded dead-letter path.
- [ ] Every background job has durable state, bounded batches, retry/backoff, lease recovery, and
  crash/cancellation tests.
- [ ] Cluster connection budgets account for all pools per process, replicas, migrations, probes,
  and operational reserve without exceeding PostgreSQL capacity.
- [ ] Load, soak, failover, fault-injection, cancellation, and OOM tests establish supported request,
  queue, document, export, and import limits.
- [ ] Availability, latency, saturation, queue age, retry, dead-letter, and error-budget alerts are
  installed and exercised.

### Security And Privacy

- [ ] Provider egress is constrained by reviewed network policy and DNS-rebinding/redirect tests.
- [ ] Audit export is cursor-streamed and bounded; actor anonymization and retention purge are
  durable, authorized, idempotent, and tested on release PostgreSQL.
- [ ] Production secret manager/KMS integration, credential rotation, revocation, and incident drills
  have evidence.
- [ ] Threat model, dependency audit, penetration test, and privacy review have no unresolved
  release-blocking findings.

### API, SDK, Operations, And Supply Chain

- [ ] Authored OpenAPI, materialized contracts, generated SDKs, implementation routes, pagination,
  ProblemDetail errors, and examples are byte-for-contract aligned.
- [ ] Backup/restore, migration rollback, tenant cutover, provider outage, audit investigation, and
  disaster-recovery runbooks are exercised against the release candidate.
- [ ] Images and desktop/web artifacts are immutable and include checksum, signature, SBOM,
  provenance, attestation, license approval, and reproducible workflow evidence.
- [ ] Real catalog media, rollout/rollback evidence, live smoke results, and production SLO ownership
  are attached before `agent.app.config.json` publication gates are enabled.

## Verification

```bash
pnpm check
pnpm test:security
pnpm test:multi-tenant-isolation
pnpm api:materialize:check
pnpm sdk:generate:check
pnpm verify
```

Repository checks are necessary but do not replace human migration/security review, release
PostgreSQL evidence, capacity testing, operational drills, or supply-chain approval.

## References

- [dedicated tenant/organization ADR](../../architecture/decisions/ADR-20260731-dedicated-tenant-organization-runtime.md)
- [tenant isolation specification](../../../specs/tenant-isolation.md)
- [tenant isolation runbook](../../runbooks/tenant-isolation.md)
- [PRD-mvp-launch.md](PRD-mvp-launch.md)
