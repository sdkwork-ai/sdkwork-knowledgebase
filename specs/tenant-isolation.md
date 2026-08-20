# Tenant Isolation Architecture Specification

Status: active, prelaunch-gated
Owner: SDKWork Knowledgebase architecture team

## 1. Design Principle

**Tenant identity is derived from the authenticated principal, never from HTTP request
parameters.** The IAM-token-derived tenant is the security boundary. Application-layer
`tenant_id` and `organization_id` filters are the application guard; PostgreSQL RLS
enforces the same deployment-bound pair as defense in depth. Shared request pooling is
not a supported runtime profile.

## 2. Identity Flow

```text
HTTP Request
  │
  ├─ Authorization: Bearer <IAM_jwt>
  ├─ Access-Token: <access_jwt>
  │
  ▼
sdkwork-iam-web-adapter
  │
  ├─ Validates JWT claims
  ├─ Extracts: tenant_id, user_id, organization_id, session_id, permission_scope
  │
  ▼
WebRequestContext { principal: WebRequestPrincipal }
  │
  ├─ tenant_id()     -> Snowflake u64
  ├─ user_id()       -> Snowflake u64 (actor/operator)
  ├─ organization_id -> Snowflake u64 claim (required for production organization scope)
  ├─ session_id      -> Option<String>
  └─ scopes          -> Vec<String>
  │
  ▼
KnowledgeAppRequestContext  /  KnowledgeBackendRequestContext
  │
  ├─ tenant_id: u64           (from principal - never from client body/headers)
  ├─ actor_id / operator_id: Option<u64>
  ├─ organization_id: Option<u64> (framework shape; production guard requires a match)
  └─ session_id / permission_scope
```

**No `x-tenant-id` header, no `tenant_id` request body parameter for protected routes.**
Every handler must derive tenant identity exclusively from the request context injected
by the framework middleware.

## 3. Isolation Layers (Defense in Depth)

### 3.1 IAM/Dual-Token Auth (Layer 1)

- **App API** and **Backend API**: require two JWT headers
  - `Authorization: Bearer <auth_token>` — proves identity
  - `Access-Token: <access_token>` — proves scope/permission
- **Open API**: API key auth (`x-sdkwork-auth-mode: api-key`)
- `sdkwork-iam-web-adapter` validates both tokens before any handler runs

### 3.2 Runtime Tenant Binding (Layer 2)

Each API/worker deployment binds to ONE tenant via environment variable:

```bash
SDKWORK_KNOWLEDGEBASE_TENANT_ID=<tenant_id>               # required in production
SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID=<org_id>            # required and non-zero in production
```

- `ensure_runtime_tenant(context)` compares `context.tenant_id` against the
  configured deployment tenant. Mismatch → `403 tenant_id_mismatch`.
- `ensure_runtime_organization(context)` checks organization scoping.
- Production-like environments **fail closed** when either deployment scope is absent,
  zero, non-canonical, or outside signed PostgreSQL `BIGINT` range: no server pool is created.
- Domain `organization_id=0` means personal scope and is never a wildcard. The current
  production profile requires an explicit non-zero organization deployment.

### 3.3 Application-Layer Query Filters (Layer 3)

Every organization-owned SQLx repository operation must bind the deployment `tenant_id`
and `organization_id`. Queries and mutations must include both predicates; RLS is not a
substitute for repository-local scope. Cross-organization identifiers must be rejected
even when they belong to the same tenant.

Additional guard functions enforce scope at the handler level:
- `ensure_tenant_scope(request_tenant_id, store_tenant_id)` — rejects mismatches
- `require_space_access(space_id, tenant_id, actor_id)` — validates membership

### 3.4 PostgreSQL RLS (Layer 4)

Each API/worker process binds exactly one tenant and one organization. PostgreSQL
connection options set both variables before the process-shared pool is created:

```text
app.current_tenant_id
app.current_organization_id
```

RLS policies on organization-owned tables use both values:
  ```sql
  USING (
    tenant_id = current_setting('app.current_tenant_id', true)::bigint
    AND organization_id = current_setting('app.current_organization_id', true)::bigint
  )
  WITH CHECK (
    tenant_id = current_setting('app.current_tenant_id', true)::bigint
    AND organization_id = current_setting('app.current_organization_id', true)::bigint
  )
  ```
- `FORCE ROW LEVEL SECURITY` applies to business tables.
- Release verification uses a non-owner, non-superuser application role; owner/superuser
  connections are not isolation evidence.
- Request-scoped `SET LOCAL` and cross-tenant connection checkout are deliberately absent.
- The process-wide connection budget is split between one scoped typed PostgreSQL pool and the
  temporary scoped `AnyPool`; Drive, pgvector, and cloud provider resolution reuse the typed handle.
  The compatibility pool remains a prelaunch removal gate.

### 3.5 Drive Object Store (Layer 5)

Storage paths include tenant IDs:
```
knowledge/{tenant_id}/{space_uuid}/...
```
This provides object-level tenant isolation alongside SQL.

## 4. Scope Model

### 4.1 Permission Scopes

Authority-tier scopes (Stage 1 — lateral containment):

| Scope Pattern | Permission |
|---|---|
| `knowledge.*` | All knowledge operations |
| `knowledge.platform.manage` | Admin operations (backend API) |
| `knowledge.spaces.*` | All space operations |
| `knowledge.spaces.read` | Read spaces |
| `knowledge.spaces.write` | Create/update spaces |
| `knowledge.spaces.delete` | Delete spaces |
| `knowledge.documents.*` | All document operations |
| `knowledge.index_jobs.*` | All index job operations |
| `knowledge.drive_object_refs.*` | All drive object ref operations |
| `knowledge.audit_events.*` | Audit log access |
| `knowledge.retrievals.*` | Retrieval access |
| `knowledge.browser.*` | Browser tree visibility |

Deny-over-allow: ACL deny entries take precedence over allow entries.

### 4.2 Scope Evaluator (Domain Scope)

The `can_access()` function evaluates domain scope by testing whether the
authenticated context's permission_scope matches the target domain pattern.

## 5. Tenant Lifecycle

### 5.1 Tenant Creation (IAM Layer)

**Tenant creation is handled by the IAM layer**, not by knowledgebase.
Knowledgebase does not have a `setup_tenant` endpoint — tenant provisioning
is a platform-level operation managed by SDKWork IAM.

### 5.2 Tenant Status (Knowledgebase Layer)

Knowledgebase provides a single endpoint to retrieve the caller's own tenant
knowledgebase statistics:

- `GET /backend/v3/api/knowledge/tenants/current` — retrieve tenant status
  - Returns: `{ tenant_name?, status, space_count, document_count, created_at?, quota? }`
  - `quota` includes document, ingest concurrency, and retrieval-per-minute limits with current usage
  - **Security**: Tenant identity is derived from `WebRequestPrincipal.tenant_id()`
    — no `tenant_id` parameter is accepted.

### 5.3 Cross-Deployment Isolation

- Each tenant/organization pair runs in its own API/worker deployment.
- Deployments may share a PostgreSQL cluster and tables because RLS and repository predicates
  isolate the pair; they do not share an authenticated request-processing pool.
- No cross-tenant data access is possible even if token validation is bypassed

### 5.4 Tenant Business Quotas

Business quotas are enforced in app-api handlers and surfaced to operators through
`KnowledgeTenantStatus.quota` on the backend admin API and `/admin` console.

| Environment variable | Default (dev) | Enforced on |
| --- | --- | --- |
| `SDKWORK_KNOWLEDGEBASE_TENANT_MAX_DOCUMENTS` | platform default | Document create |
| `SDKWORK_KNOWLEDGEBASE_TENANT_MAX_CONCURRENT_INGEST_JOBS` | platform default | Ingest start / post-enqueue verify |
| `SDKWORK_KNOWLEDGEBASE_TENANT_MAX_RETRIEVALS_PER_MINUTE` | platform default | Retrieval rate limit store |
| `SDKWORK_KNOWLEDGEBASE_TENANT_MAX_STORAGE_BYTES` | 100 GiB | Measured upload/object-ref writes; Drive import projected-size reservation is a commercial release gate |

Quota violations return `ProblemDetail` with code `60002` (`knowledge_tenant_quota_exceeded`).

GDPR subject export and anonymization for `kb_audit_event` are available through:

- `POST /backend/v3/api/knowledge/compliance/audit_events/export` (`compliance.auditEvents.export.create`)
- `POST /backend/v3/api/knowledge/compliance/audit_events/anonymize_actor` (`compliance.auditEvents.anonymizeActor.create`)

The synchronous export returns a complete result only up to 5,000 rows and returns
`413 audit_export_limit_exceeded` above that bound. It never returns a partial result. A larger
export requires a future SDK-governed cursor or asynchronous job contract.

See [audit-retention.md](../docs/runbooks/audit-retention.md) for operator procedures.

## 6. Fail-Closed Verification

| Scenario | Expected Behavior |
|---|---|
| tenant or organization deployment scope absent/zero in production-like env | App boot fails; no DB pool |
| `tenant_id` in token != `SDKWORK_KNOWLEDGEBASE_TENANT_ID` | `403 tenant_id_mismatch` |
| `organization_id` mismatch | `403 organization_id_mismatch` |
| Unauthenticated request | `401` |
| Either RLS session variable missing (PostgreSQL) | No business rows are visible or writable |
| Client sends `tenantId` in request body | Silently overridden with token-derived value |
| Tenant status = `Suspended` | All API calls return `403` |

## 7. Context Binding Isolation

Context bindings (`kb_space_context_binding`) associate agent profiles with spaces. Isolation must enforce:
1. Space access check before binding creation
2. No blind retrieval of bindings — caller must have space access
3. tenant_id and organization_id are bounded by the space membership check

## 8. Observability

- Metric: `knowledge_api_auth_failures_total` (401/403 counter)
- Structured audit events: `knowledge.backend.admin_operation` with tenant context
- Log field: `deployment_tenant_id` for platform-level events
- Mismatch counters use bounded reason labels; raw tenant/organization ids remain log fields,
  not metric labels.

## 9. References

- ADR: `docs/architecture/decisions/ADR-20260731-dedicated-tenant-organization-runtime.md`
- Runbook: `docs/runbooks/tenant-isolation.md`
- RLS Migration: `database/migrations/postgres/202607310001_core_organization_isolation.up.sql`
- Contract: `crates/sdkwork-knowledgebase-contract/src/tenant.rs`
- Backend Auth Guards: `crates/sdkwork-routes-knowledgebase-backend-api/src/auth.rs`
- App API Context: `crates/sdkwork-routes-knowledgebase-app-api/src/web_bootstrap.rs`
- RLS Session: `crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/db/postgres_tenant_session.rs`
- Tenant Isolation Spec: `specs/tenant-isolation.md` (this document)
