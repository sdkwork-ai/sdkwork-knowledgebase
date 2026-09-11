# Runbooks

Purpose: operational runbooks for SDKWork Knowledgebase services.

Owner: SDKWork Knowledgebase maintainers.

## Active runbooks

- [token-rotation.md](token-rotation.md)
- [tenant-isolation.md](tenant-isolation.md)
- [migration-rollback.md](migration-rollback.md)
- [RUNBOOK-provider-binding-readiness.md](RUNBOOK-provider-binding-readiness.md)
- [RUNBOOK-provider-credential-resolution.md](RUNBOOK-provider-credential-resolution.md)
- [provider-outage.md](provider-outage.md)
- [rate-limit-incident.md](rate-limit-incident.md)
- [audit-investigation.md](audit-investigation.md)
- [RUNBOOK-group-knowledgebase-lifecycle.md](RUNBOOK-group-knowledgebase-lifecycle.md)
- [../deployments/runbooks/backup-restore.md](../deployments/runbooks/backup-restore.md)

Allowed content: operational runbooks for token rotation, tenant isolation, migration rollback,
Provider Binding readiness, Provider credential resolution, provider outage, rate-limit incidents,
and audit investigation.

Forbidden content: implementation code, generated SDK output, secrets, and runtime state.

Related specs: `../sdkwork-specs/DOCUMENTATION_SPEC.md`, `../sdkwork-specs/OBSERVABILITY_SPEC.md`.


<!-- scaffold-module-runbooks:index -->
## Docker 运维四件套（bin/ 标准，OPERATIONS_SPEC.md §7）

| Runbook | 内容 |
| --- | --- |
| [deploy.md](deploy.md) / [deploy.en.md](deploy.en.md) | 安装 / 升级 / 回滚 / 下线（bin/docker-deploy.sh + bin/docker-image.sh） |
| [troubleshooting.md](troubleshooting.md) / [troubleshooting.en.md](troubleshooting.en.md) | 症状 → doctor 检查 → 处置 |
| [backup-restore.md](backup-restore.md) / [backup-restore.en.md](backup-restore.en.md) | 备份 / 校验 / 恢复 / 演练（bin/backup.sh） |
| [log-reference.md](log-reference.md) / [log-reference.en.md](log-reference.en.md) | 健康日志特征与失败签名（bin/docker-deploy.sh logs） |
