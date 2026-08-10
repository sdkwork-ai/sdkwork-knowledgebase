# SDKWork Knowledgebase Deployment Artifacts

Deployment descriptors for the `cloud.test`, `cloud.staging`,
`cloud.production`, and `standalone.production` deploy.yaml profiles. The
cloud profiles expose the registered hosts
(`knowledgebase[-test|-staging].sdkwork.com` plus the
`knowledgebase-admin` / `knowledge` auxiliary surfaces) per
`../sdkwork-specs/APP_RUNTIME_TOPOLOGY_NAMING.md` section 9. Development
profiles are source config under `etc/topology/` and are never deploy
targets; `standalone.test`/`standalone.staging` fold to loopback URLs.

## Contents

| Path | Purpose |
|------|---------|
| `../Dockerfile` | Standalone container image (staged context from `pnpm build:container`; gateway + worker binaries + portal dist + database modules) |
| `../docker-compose.yml` | Standalone container composition (api + worker + postgres + redis) |
| `../docker/` | Compose env template, postgres init schema, nginx test-domain reverse proxy |
| `kubernetes/app-api-deployment.yaml` | App API Deployment + Service |
| `kubernetes/worker-deployment.yaml` | Background worker Deployment |
| `kubernetes/ingress.yaml` | NGINX Ingress for app/backend/open/internal API paths |
| `kubernetes/hpa.yaml` | Resource-based HorizontalPodAutoscaler for API and worker Deployments; custom RPS/backlog metrics require deployed Prometheus Adapter rules |
| `kubernetes/poddisruptionbudget.yaml` | PodDisruptionBudget for rolling update safety |
| `kubernetes/networkpolicy.yaml` | Restrict ingress to NGINX and monitoring namespaces |
| `kubernetes/servicemonitor.yaml` | Prometheus Operator scrape targets for `/metrics` |
| `runbooks/backup-restore.md` | PostgreSQL and Drive object backup/restore |
| `runbooks/production-launch.md` | Production cutover sequencing, smoke gates, and rollback |

## Quick start (standalone container, docker compose)

Build the image and start the stack (PostgreSQL + Redis are bundled as compose
services; see [docker-deployment.md](../docs/installation/docker-deployment.md)):

```bash
pnpm build:container            # docker build -f Dockerfile -t sdkwork-knowledgebase:local
docker compose up -d            # api (3904:18081) + worker + postgres + redis
curl -fsS http://127.0.0.1:3904/readyz
```

## Quick start (Kubernetes)

1. Build and push images (replace registry) for the `cloud.production`
   topology; the standalone container image above is the single-deployment-unit
   build, and cloud deploys the same binaries as k8s Deployments:
2. Apply secrets and config from `etc/topology/cloud.production.env`.
   - `sdkwork-knowledgebase-drive-internal-api` must contain key `ingress-token`.
   - `sdkwork-knowledgebase-drive-events` must contain key `current`; add key `previous`
     only during a controlled signing-secret rotation overlap and set the previous-secret env key.
   - Follow `runbooks/production-launch.md` for the required paged channel renewal, overlap window,
     event smoke, rollback, and previous-key removal sequence.
3. Apply manifests:
   ```bash
   kubectl apply -f deployments/kubernetes/
   ```
4. Verify probes:
   - Liveness: `GET /livez`
   - Readiness: `GET /readyz`

## Observability

| Variable | Purpose |
|----------|---------|
| `RUST_LOG` | Tracing filter (e.g. `info,sdkwork_api_knowledgebase_standalone_gateway=debug`) |
| `SDKWORK_KNOWLEDGEBASE_LOG_FORMAT` | Set to `json` for structured JSON logs in production aggregators |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | When set, API/worker processes export traces over OTLP/HTTP (requires `otel` feature build) |
| `SDKWORK_NODE_INSTANCE_ID` | Stable per-process allocator identity; Kubernetes injects the pod UID |
| `SDKWORK_DATABASE_MAX_CONNECTIONS` | Combined per-process PostgreSQL connection budget; must be at least `2` and is split between the typed pool and temporary `AnyPool` compatibility pool |
| `SDKWORK_KNOWLEDGEBASE_TENANT_ID` | Required canonical positive signed `BIGINT`; fixes the deployment tenant |
| `SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID` | Required canonical positive signed `BIGINT`; fixes the deployment organization |
| `SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_PROVIDER_ID` | Required cloud Drive provider id; provider must be active, non-local, and bucket-ready |
| `SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_JOB_LEASE_SECONDS` | Worker job lease TTL, 30-3600 seconds; default `300` |
| `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID` | Required tenant-local service actor for auditable Wiki maintenance commands |
| `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_BATCH_SIZE` | Maximum source projections claimed per checkpoint and tick, 1-100; default `10` |
| `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_LEASE_SECONDS` | Source-processing lease TTL, 1-3600 seconds; default `120` |
| `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_RETRY_DELAY_SECONDS` | Delay before retrying a failed source, 1-86400 seconds; default `30` |
| `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_SOURCE_MAX_ATTEMPTS` | Maximum processing attempts before quarantine, 1-100; default `10` |
| `OTEL_SERVICE_NAME` | Overrides the default OpenTelemetry service name per process |

Production ID generation uses the shared `sdkwork_node_registry` database table. The allocator heartbeats a fenced node lease and `/readyz` fails if the lease becomes unhealthy. Do not set `SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID` in normal deployments; a static numeric override additionally requires `SDKWORK_KNOWLEDGEBASE_ALLOW_STATIC_SNOWFLAKE_NODE_ID=true` in production-like environments.

Cloud API and worker replicas resolve object storage through the Drive provider registry. The
provider credential is referenced by Drive configuration and resolved only inside the trusted Drive
runtime. `local_filesystem` and `SDKWORK_KNOWLEDGEBASE_DRIVE_STORAGE_ROOT` are rejected as cloud
storage authority. Provider lookup, version, active state, adapter creation, and bucket health are
part of startup readiness.

HTTP APIs emit an `x-request-id` response header (or echo inbound `x-request-id`) for request correlation. Prometheus metrics are exposed at `GET /metrics` on API and worker health processes, including `knowledgebase_health_status` (updated by `/readyz`). **Do not expose `/metrics` on public ingress**; use in-cluster ServiceMonitor scraping only.

The Drive event receiver is mounted only at
`/internal/v3/api/knowledgebase/drive_events` on `application.public-ingress`. It requires both
the ingress token and the signed Drive event headers. The callback is not routed through
`platform.api-gateway`; no legacy or alternate callback path is supported.

Each maintenance tick first applies the bounded Drive event page and then processes the same
checkpoint page's source projections. Markdown/MDX and approved passive page formats are rendered
to sanitized HTML; JavaScript, SVG, WebAssembly, invalid pins, and exhausted failures are
quarantined. `AUTO_PUBLIC_AFTER_CHECKS` uses the same version-fenced page publication command as
manual publication, while review-required and private content remains unpublished.

Structured audit events (for example `knowledge.document.visibility_changed`, `knowledge.space.member_granted`, `knowledge.space.member_revoked`, `okf.concept.published`) are written to structured logs with an `audit_event` field. Related Prometheus counters are exported at `GET /metrics` (`knowledge_audit_*`).

Billable usage counters (`knowledge_retrievals_total`, `knowledge_context_packs_total`, `knowledge_ingest_jobs_succeeded_total`, `knowledge_ingest_jobs_failed_total`) and structured `billing_event` JSON log lines support commercial metering pipelines.

Post-deploy public health smoke check (optional). Public smoke checks only probe `/livez` and `/readyz`; `/metrics` must stay off public ingress:

```bash
SDKWORK_KNOWLEDGEBASE_SMOKE_BASE_URL=https://knowledgebase.sdkwork.com pnpm test:smoke
```

Internal metrics smoke (optional, run from an in-cluster network path only):

```bash
SDKWORK_KNOWLEDGEBASE_SMOKE_METRICS_URLS=http://sdkwork-knowledgebase-app-api,http://sdkwork-knowledgebase-worker:18085 pnpm test:smoke
```

Optional PC renderer shell probe (requires a running Vite preview or dev server):

```bash
SDKWORK_KNOWLEDGEBASE_E2E_BASE_URL=http://127.0.0.1:5173 pnpm test:e2e
```

Playwright shell smoke (build + preview + Chromium):

```bash
pnpm --dir apps/sdkwork-knowledgebase-pc run test:e2e:install
pnpm test:e2e:playwright
```

## Tenant isolation

Each API/worker deployment is bound to exactly one runtime tenant and one non-zero organization
through `SDKWORK_KNOWLEDGEBASE_TENANT_ID` and
`SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID`. Authenticated request context must match both values;
mismatches return `403` with `tenant_id_mismatch` or `organization_id_mismatch`.

The supported production profile is **one dedicated deployment per tenant/organization pair**.
Replicas in that deployment share its fixed identity. Multiple dedicated deployments may use the
same PostgreSQL cluster and tables, but each pool sets both `app.current_tenant_id` and
`app.current_organization_id`; ordinary repositories must also bind both columns. A process that
switches scope per request is unsupported and must not receive production traffic.

Each process owns exactly one scoped typed PostgreSQL pool and, until the prelaunch migration is
complete, one scoped `AnyPool` compatibility pool. `SDKWORK_DATABASE_MAX_CONNECTIONS` is divided
between them; an odd connection is assigned to the typed pool. Drive, pgvector, and cloud provider
resolution clone the same typed pool handle and must not create another pool. Capacity planning must
multiply this per-process budget by the maximum simultaneous replica count and retain PostgreSQL
headroom for migrations, probes, administration, and failover.

Integration coverage: `crates/sdkwork-routes-knowledgebase-app-api/tests/integration_tenant_isolation.rs`.

## Backend authorization

Backend API operations require the `knowledge.platform.manage` permission (or `knowledge.*`) on the authenticated operator's access token. Mutations are audited as `knowledge.backend.admin_operation` structured log events and exported via `knowledge_audit_backend_admin_operation_total` at `GET /metrics`.

## Related specs

- `../sdkwork-specs/DEPLOYMENT_SPEC.md`
- `../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md`
- `../specs/topology.spec.json`

Status: active.
