# KNOWLEDGEBASE Database Module

Canonical lifecycle assets for `sdkwork-knowledgebase` per `DATABASE_FRAMEWORK_SPEC.md`.

- moduleId: `knowledgebase`
- serviceCode: `KNOWLEDGEBASE`
- tablePrefix: `kb_`; SDKWork Web Framework owns and migrates `web_` tables independently in the shared schema.
- baselineAnchorTable: `kb_space`

## Initialization State

The module uses `baselineStrategy: baseline-plus-migrations`:

1. **Baseline** — `database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql` is the
   authoritative greenfield DDL snapshot. Post-baseline changes are folded into it for fresh
   installs; the baseline is not a complete migration inventory.
2. **Migrations** — `database/migrations/postgres/` carries the ordered, idempotent corrections that
   converge already-initialized environments (`0001` organization NOT NULL, `0002` quota counters,
   `0003` agent-profile owner, `0004` query indexes). Each migration has an `sdkwork:migration`
   header and a forward-fix policy; none is deleted.
3. **Drift** — run `pnpm db:drift:check` before release. Business tables are not ignored by drift
   policy.

The authoritative server lifecycle is PostgreSQL-only. The SQLite schema under
`tests/fixtures/database/sqlite/` is a reference artifact used by the repository schema-parity test
(test-only `sqlite` driver); it is not a server profile and does not define server authority.
`database/database.manifest.json` declares `engines: ["postgres"]`.

Web Framework audit tables are initialized by `sdkwork-web-framework`, not copied into this module's
baseline.

## Commands

```bash
pnpm run db:validate
pnpm run db:materialize:contract
pnpm run db:plan
pnpm run db:init
pnpm run db:migrate
pnpm run db:seed
pnpm run db:status
pnpm run db:drift:check
```
