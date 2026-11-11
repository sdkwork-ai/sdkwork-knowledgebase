# Production launch runbook

Status: active  
Application: sdkwork-knowledgebase  
Topology: `cloud.production`<br>
Updated: 2026-07-23

## Purpose

Operational checklist for launching SDKWork Knowledgebase in a tenant-scoped production deployment. Run this only after the current repository verification and release-evidence gates pass locally and in CI.

## Pre-flight

1. Confirm secrets are provisioned outside topology files:
   - Database password (`SDKWORK_DATABASE_PASSWORD_FILE`)
   - Secrets encryption key (`SDKWORK_KNOWLEDGEBASE_SECRETS_ENCRYPTION_KEY_FILE`)
   - Outbox webhook URL and signing secret
   - Drive event signing master secret (`sdkwork-knowledgebase-drive-events/current`) only when a split deployment pushes signed webhook events; the default deployment delivers Drive events through the embedded trusted relay and needs no Drive Internal API ingress token
   - Database-backed Snowflake node lease identity for every ingress/worker replica
   - No static `SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID` unless an emergency change record explicitly enables the override
   - A dedicated tenant-local service actor matching `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID`; it must not be a human administrator account
2. Apply topology from `etc/topology/cloud.production.env`.
3. Ensure public ingress exposes the app/backend/open paths and the single protected Drive callback at `/internal/v3/api/knowledgebase/drive_events`. The callback requires both application ingress authentication and the Drive signature contract and must not be re-exported by `platform.api-gateway`; it only receives events from a split deployment that is configured to push them, because the default deployment consumes Drive's outbox in-process. **Do not expose `/metrics` on ingress**; Prometheus scrapes in-cluster via ServiceMonitor.
4. Run repository gates:
   ```bash
   pnpm verify
   pnpm test:launch-readiness
   pnpm lint
   ```

## Database bootstrap

1. Point `SDKWORK_DATABASE_URL` at the production PostgreSQL instance.
2. Bootstrap and validate:
   ```bash
   pnpm db:bootstrap
   pnpm db:validate
   pnpm db:drift:check
   ```
3. Record migration status output in the change ticket.

## Deployment smoke

1. Deploy Kubernetes manifests from `deployments/kubernetes/`.
2. Verify health on each API surface:
   ```bash
   SDKWORK_KNOWLEDGEBASE_SMOKE_BASE_URL=https://knowledgebase.example.com pnpm test:smoke
   ```
   Public smoke checks only probe `/livez` and `/readyz`; `/metrics` remains in-cluster only.
   Probe the application public ingress and worker independently; app/backend/open URLs are logical route authorities on the same ingress:
   ```bash
   SDKWORK_KNOWLEDGEBASE_SMOKE_APP_URL=https://knowledgebase.example.com \
   SDKWORK_KNOWLEDGEBASE_SMOKE_BACKEND_URL=https://knowledgebase-admin.example.com \
   SDKWORK_KNOWLEDGEBASE_SMOKE_OPEN_URL=https://knowledge.example.com \
   SDKWORK_KNOWLEDGEBASE_SMOKE_WORKER_URL=https://knowledgebase-worker.example.com \
   pnpm test:smoke
   ```
3. Optional internal metrics smoke, run only from an in-cluster network path:
   ```bash
   SDKWORK_KNOWLEDGEBASE_SMOKE_METRICS_URLS=http://sdkwork-knowledgebase-app-api,http://sdkwork-knowledgebase-worker:18085 \
   pnpm test:smoke
   ```
4. Optional authenticated admin probe:
   ```bash
   SDKWORK_KNOWLEDGEBASE_SMOKE_BACKEND_URL=https://knowledgebase-admin.example.com \
   SDKWORK_KNOWLEDGEBASE_SMOKE_ACCESS_TOKEN=... \
   pnpm test:launch-readiness
   ```
5. Optional integrator open API probe:
   ```bash
   SDKWORK_KNOWLEDGEBASE_SMOKE_OPEN_URL=https://knowledge.example.com \
   SDKWORK_KNOWLEDGEBASE_SMOKE_API_KEY=... \
   pnpm test:launch-readiness
   ```
6. Confirm worker `/readyz` returns 200, queued ingestion jobs drain, and Wiki source counters do
   not show a growing retry/quarantine backlog after a test ingest.

## Drive event delivery

Normal operation delivers Drive events through the embedded trusted relay: the worker dispatches
Drive's transactional outbox into the Wiki event consumer in-process, over the shared PostgreSQL.
There is no Drive Internal API call, no webhook channel to register, and no channel renewal to
operate.

1. Confirm the worker's bounded renewal scan reports `embedded_relays_verified` for the tenant's
   Wiki checkpoints. Treat every structured
   `knowledgebase.wiki.drive_event_delivery_renewal_failed` event as a failed gate; its
   `checkpoint_id`, `source_scope_uuid`, and stable `error_code` identify the retry target.
2. Upload a Markdown file under a Wiki `sources/raw` root and verify: Drive outbox status is
   delivered, the Knowledgebase inbox event is applied, its checkpoint advances, the source moves
   `DISCOVERED -> PROCESSING -> READY`, and an auto-public Wiki moves the page to `PUBLISHED`.
   Resolve the public route and verify the response is sanitized `text/html; charset=utf-8` pinned
   to the uploaded Drive version. Delete the same file and verify the public route is revoked
   immediately. Neither operation may create a Deploy Release, Deployment, or SiteRevision.

A deliberately separated deployment that pushes signed Drive events to the inbound
`/internal/v3/api/knowledgebase/drive_events` receiver keeps the two-secret overlap procedure:
mount only `/run/secrets/sdkwork/drive-event-signing/current` normally; during a controlled
rotation publish `current = new` and `previous = old`, set
`SDKWORK_KNOWLEDGEBASE_DRIVE_EVENT_PREVIOUS_SIGNING_SECRET_FILE` on both the app API and worker
Deployments, roll both, wait for the Drive webhook retry window to drain, then remove the
`previous` Secret key and environment entry. Never perform an uncoordinated single-secret
rollback.

## Observability

1. Set `SDKWORK_KNOWLEDGEBASE_LOG_FORMAT=json` in production topology for structured log aggregation.
2. When an OTLP collector is available, set:
   - `OTEL_EXPORTER_OTLP_ENDPOINT`
   - `OTEL_SERVICE_NAME` per process (`sdkwork-knowledgebase-public-ingress`, `sdkwork-knowledgebase-worker`)
3. Confirm ServiceMonitor targets scrape `/metrics` in-cluster and dashboards include:
   - `knowledgebase_health_status`
   - `knowledge_api_requests_total`
   - `knowledge_api_auth_failures_total`

## Release artifacts

1. Keep the browser bundle package (`web-universal-cloud-browser-zip`) disabled while its manifest
   package metadata remains `releaseStatus: prelaunch-artifact-pending`.
2. Do not publish or enable `web-universal-cloud-browser-zip` until release evidence has been
   recorded for checksum, signature, SBOM, provenance, attestation, workflow run, rollout,
   rollback, and live smoke results per `agent.app.config.json`.
3. Publish or approve private-registry consumption for all three SDK families:
   - `sdkwork-knowledgebase-app-sdk`
   - `sdkwork-knowledgebase-backend-sdk`
   - `sdkwork-knowledgebase-sdk`
4. Keep desktop bundles disabled until desktop CI packaging targets ship (`releaseStatus: prelaunch-disabled`).
5. Run Playwright launch flows in CI:
   ```bash
   pnpm test:e2e:playwright
   ```

## Backup drill

Before traffic cutover, exercise `deployments/runbooks/backup-restore.md` in staging:

1. Take a logical PostgreSQL dump.
2. Restore to an isolated database.
3. Re-run `pnpm db:migrate`, `pnpm db:status`, and `/readyz` checks.

## Rollback

1. Roll back Deployments to the previous image tag.
2. Restore database only when schema drift requires it; otherwise prefer forward-fix migrations.
3. Disable ingress traffic and verify `/readyz` on the rolled-back revision before re-enabling traffic.

## Sign-off

| Gate | Owner | Evidence |
|------|-------|----------|
| Security (`pnpm test:security`) | Platform | CI run URL |
| Architecture (`pnpm check`) | App team | CI run URL |
| DB bootstrap/drift | DBA | `pnpm db:status` output |
| API smoke | SRE | `pnpm test:smoke` output |
| Drive event delivery (embedded relay) | SRE | renewal page evidence + outbox/inbox/checkpoint smoke |
| PC author/search E2E | Frontend | Playwright CI job |
| Release artifacts | Release | Workflow run + checksum + signature + SBOM + provenance + attestation + rollout/rollback + live smoke record |
