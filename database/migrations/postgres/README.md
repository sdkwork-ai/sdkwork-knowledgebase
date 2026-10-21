# migrations/postgres

Pre-launch the knowledgebase schema is consolidated on the single greenfield
baseline: `database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql`.
All post-baseline migrations (group knowledge spaces, ingestion leases,
provider bindings, live wiki publication, organization isolation, outbox
claim fencing/retry backoff, audit scope indexes, agent-profile ownership) are
folded into the baseline. Shared development schemas converge by resetting the
module state to the baseline.

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
- `0003_agent_profile_owner.up.sql` — add `kb_agent_profile.created_by` so a
  profile with no enabled space binding still has an object-level owner. The
  column is folded into the baseline `CREATE TABLE`, so fresh installs receive
  it during baseline initialization and the idempotent `ALTER` is a no-op there.
  No index is added: authorization loads the profile by its scoped primary key
  and compares the owner in memory.
- `0004_query_indexes.up.sql` — add the space document keyset index
  `(tenant_id, organization_id, space_id, status, id)`, the post-ingest
  embedding backlog index
  `(tenant_id, organization_id, status, index_state, updated_at, id)`, and a
  Wiki source-claim partial index keyed by publication. The same indexes are
  folded into the baseline, so fresh installs receive them during baseline
  initialization and the idempotent `CREATE INDEX` statements are no-ops there.
- `0005_timestamptz_normalization.up.sql` — convert every `kb_*` business
  instant from `TIMESTAMP` to `TIMESTAMPTZ`, interpreting existing UTC values
  with `AT TIME ZONE 'UTC'`. The baseline is normalized in the same change, so
  fresh installs create `TIMESTAMPTZ` columns and the idempotent DO block is a
  no-op there.
- `0006_document_metadata_gin.up.sql` — add the `jsonb_path_ops` GIN index on
  `kb_document.metadata` so retrieval metadata filters use
  `metadata @> jsonb_build_object(key, value)` instead of an unindexable
  `->>` expression. Folded into the baseline, so fresh installs receive it.
- `0007_audit_retention_legal_hold.up.sql` — add subject-scoped legal holds
  (`kb_audit_legal_hold`) and the `(tenant_id, organization_id, created_at, id)`
  purge index. Held actors/resources are skipped by the batched retention delete.
