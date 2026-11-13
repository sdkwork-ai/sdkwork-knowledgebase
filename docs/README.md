# Docs

## Purpose

`docs/` contains repository documentation, architecture records, product requirements, and operator runbooks for SDKWork Knowledgebase.

## Owner

SDKWork Knowledgebase maintainers.

## Canon documents

| Document | Path |
| --- | --- |
| Product PRD | [product/prd/PRD.md](product/prd/PRD.md) |
| MVP launch acceptance | [product/prd/PRD-mvp-launch.md](product/prd/PRD-mvp-launch.md) |
| Commercial release readiness | [product/prd/PRD-phase2-commercial-saas.md](product/prd/PRD-phase2-commercial-saas.md) |
| Technical architecture | [architecture/tech/TECH_ARCHITECTURE.md](architecture/tech/TECH_ARCHITECTURE.md) |
| Backend design (storage, crates, persistence, API envelope) | [architecture/tech/TECH-2026-06-01-knowledgebase-backend-design.md](architecture/tech/TECH-2026-06-01-knowledgebase-backend-design.md) |
| OKF bundle operator summary | [architecture/tech/TECH-okf-knowledge-bundle.md](architecture/tech/TECH-okf-knowledge-bundle.md) |
| Open API design | [architecture/tech/TECH-2026-06-12-knowledgebase-open-api-design.md](architecture/tech/TECH-2026-06-12-knowledgebase-open-api-design.md) |
| OKF knowledge bundle | [architecture/tech/TECH-2026-06-19-okf-knowledge-bundle-design.md](architecture/tech/TECH-2026-06-19-okf-knowledge-bundle-design.md) |
| Live Wiki publication PRD | [product/prd/PRD-live-wiki-publication.md](product/prd/PRD-live-wiki-publication.md) |
| Live Wiki provider architecture | [architecture/tech/TECH-live-wiki-resource-provider.md](architecture/tech/TECH-live-wiki-resource-provider.md) |
| Writing agent integration | [architecture/tech/TECH-writing-agent-integration.md](architecture/tech/TECH-writing-agent-integration.md) |
| Agent RAG design | [architecture/tech/TECH-2026-06-09-knowledgebase-agent-rag-design.md](architecture/tech/TECH-2026-06-09-knowledgebase-agent-rag-design.md) |
| Managed group knowledgebase requirement | [product/requirements/REQ-2026-0713-group-knowledgebase.md](product/requirements/REQ-2026-0713-group-knowledgebase.md) |
| Managed group knowledgebase decision | [architecture/decisions/ADR-20260713-group-knowledgebase-binding-and-launch.md](architecture/decisions/ADR-20260713-group-knowledgebase-binding-and-launch.md) |
| Provider commercialization requirement | [product/requirements/REQ-2026-0720-knowledge-engine-provider-commercialization.md](product/requirements/REQ-2026-0720-knowledge-engine-provider-commercialization.md) |
| Provider SPI v2 decision | [architecture/decisions/ADR-20260720-knowledge-engine-provider-binding-spi-v2.md](architecture/decisions/ADR-20260720-knowledge-engine-provider-binding-spi-v2.md) |
| Live Wiki publication requirement | [product/requirements/REQ-2026-0721-live-wiki-cloud-publication.md](product/requirements/REQ-2026-0721-live-wiki-cloud-publication.md) |
| Live mounted Wiki decision | [architecture/decisions/ADR-20260721-live-mounted-wiki-publication.md](architecture/decisions/ADR-20260721-live-mounted-wiki-publication.md) |
| Live Wiki implementation plan | [engineering/plans/PLAN-2026-0721-live-wiki-cloud-publication.md](engineering/plans/PLAN-2026-0721-live-wiki-cloud-publication.md) |
| Release-to-live migration | [migrations/MIG-2026-0721-release-to-live-wiki-publication.md](migrations/MIG-2026-0721-release-to-live-wiki-publication.md) |
| Live Wiki deployment integration readiness review | [engineering/reviews/REVIEW-20260721-live-wiki-deployment-integration-readiness.md](engineering/reviews/REVIEW-20260721-live-wiki-deployment-integration-readiness.md) |
| Provider commercial readiness review | [engineering/reviews/REVIEW-20260720-knowledge-engine-provider-commercial-readiness.md](engineering/reviews/REVIEW-20260720-knowledge-engine-provider-commercial-readiness.md) |
| Provider Binding readiness runbook | [runbooks/RUNBOOK-provider-binding-readiness.md](runbooks/RUNBOOK-provider-binding-readiness.md) |
| Provider credential resolution runbook | [runbooks/RUNBOOK-provider-credential-resolution.md](runbooks/RUNBOOK-provider-credential-resolution.md) |
| Tenant isolation | [runbooks/tenant-isolation.md](runbooks/tenant-isolation.md) |
| Audit retention | [runbooks/audit-retention.md](runbooks/audit-retention.md) |
| Dedicated tenant/organization runtime ADR | [architecture/decisions/ADR-20260731-dedicated-tenant-organization-runtime.md](architecture/decisions/ADR-20260731-dedicated-tenant-organization-runtime.md) |
| Docker deployment (standalone container) | [installation/docker-deployment.md](installation/docker-deployment.md) |

## Operator runbooks (repository root)

Production launch and backup procedures live under `deployments/runbooks/`:

- [deployments/runbooks/production-launch.md](../deployments/runbooks/production-launch.md)
- [deployments/runbooks/backup-restore.md](../deployments/runbooks/backup-restore.md)

## Allowed content

- Architecture decision records under `docs/architecture/decisions/`
- Active product and architecture docs under `docs/product/` and `docs/architecture/`
- Operator runbooks under `docs/runbooks/`

## Forbidden content

- Root SDKWork standard copies (use `../sdkwork-specs/`)
- Generated SDK transport output
- Secrets, tokens, or customer data
- Active API contracts (use `apis/` and `sdks/`)
- Duplicate historical design copies (canonical architecture lives under `docs/architecture/tech/`; `docs/superpowers/` is redirect-only)
- Active ADR content under retired `docs/adr/`; that directory may contain compatibility redirects only

## Related specs

- `../sdkwork-specs/DOCUMENTATION_SPEC.md`
- `../sdkwork-specs/DEPLOYMENT_SPEC.md`
- `../sdkwork-specs/RELEASE_SPEC.md`

## Verification

```bash
pnpm verify
pnpm test:launch-readiness
pnpm lint
```

Status: active.
