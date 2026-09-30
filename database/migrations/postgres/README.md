# migrations/postgres

Pre-launch the knowledgebase schema is consolidated on the single greenfield
baseline: `database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql`.
All post-baseline migrations (group knowledge spaces, ingestion leases,
provider bindings, live wiki publication, organization isolation, outbox
claim fencing/retry backoff, audit scope indexes) are folded into the
baseline. Shared development schemas converge by resetting the module state
to the baseline.

Ordered corrections for already-initialized environments:

- `0001_organization_id_not_null.up.sql` — backfill organization_id and set
  NOT NULL/DEFAULT on every scoped table.
- `0002_tenant_quota_usage_counters.up.sql` — exact tenant quota usage
  counters (`kb_tenant_quota_usage`) maintained by `kb_document` /
  `kb_drive_object_ref` row triggers, with a one-time idempotent backfill.
  Quota enforcement reads the counter row (O(1)) instead of running full
  COUNT/SUM aggregate scans inside the advisory-locked write transaction.
  The same contract is folded into the baseline, so fresh installs receive
  it during baseline initialization and this migration is a no-op there.
