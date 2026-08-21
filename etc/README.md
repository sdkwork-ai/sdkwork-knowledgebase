# SDKWork Knowledgebase Source Configuration

`sdkwork.deployment.config.json` is the single source-controlled profile index for SDKWork
Knowledgebase. It selects typed profile values from `topology/`; the topology contract is
`../specs/topology.spec.json` and the global authority is
`../sdkwork-specs/SOURCE_CONFIG_SPEC.md`.

The canonical matrix contains `standalone|cloud` crossed with
`development|test|staging|production`. Standalone development owns the local application
gateway and worker. Cloud development starts clients only and consumes explicit deployed
application and platform surfaces.

Additional safe templates:

- `examples/knowledgebase-redis.env.example`: non-secret Redis configuration example.
- `examples/browser-security-headers.nginx.conf`: browser static-host security headers.

Host-local overrides such as `.env.postgres`, `.env.local`, and `etc/**/*.local.*` stay out of
source control. Passwords, ingress tokens, signing masters, certificates, and provider credentials
come from the deployment secret manager or mounted ignored secret files. Installed runtime config
is materialized to the locations governed by `../sdkwork-specs/RUNTIME_DIRECTORY_SPEC.md`; source
`etc/` is never used as mutable runtime state.

Every profile declares the bounded Wiki event and source-processing controls consumed by
`sdkwork-knowledgebase-worker`. `SDKWORK_KNOWLEDGEBASE_WORKER_WIKI_ACTOR_ID` identifies a
tenant-local, non-human service actor that must be provisioned and retained for audit attribution;
deployments must replace the profile example when their IAM allocation differs. Source processing
runs only after the corresponding Drive event page, uses independent leases/retry limits, and never
creates a Deploy Release, Deployment, or SiteRevision.

Validate this authority with:

```powershell
node ../sdkwork-specs/tools/check-source-config-standard.mjs --root .
pnpm check:client-env
pnpm topology:validate
pnpm deploy:validate
```

<!-- SDKWORK-DEPLOY-LAYOUT: v1 -->
## Installed Runtime Paths

Authority: `APPLICATION_DEPLOY_LAYOUT_SPEC.md` (`../sdkwork-specs/`).

| Item | Value |
| --- | --- |
| `appId` | `sdkwork-knowledgebase` |
| `runtimeCode` | `knowledgebase` |
| Config root | `/etc/sdkwork/knowledgebase/` |
| Runtime TOML | `/etc/sdkwork/knowledgebase/config.toml` |
| Secrets | `/etc/sdkwork/knowledgebase/secrets/` |
| Override | `SDKWORK_KNOWLEDGEBASE_CONFIG_FILE` |

Source profiles live under `etc/` (`sdkwork.deployment.config.json` index). Deploy manifest: `deployments/deploy.yaml`. Web data-plane source: `deployments/webserver/` (`SDKWORK_WEBSERVER_SPEC.md` layout v2).

```bash
node ../sdkwork-specs/tools/check-source-config-standard.mjs --root .
node ../sdkwork-specs/tools/check-application-deploy-layout.mjs --root .
node ../sdkwork-specs/tools/check-webserver-toml-standard.mjs --root deployments/webserver
```
<!-- /SDKWORK-DEPLOY-LAYOUT -->


