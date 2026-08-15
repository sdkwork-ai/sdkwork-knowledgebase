import assert from 'node:assert/strict';
import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';

const ROOT = process.cwd();
const RETIRED_TOPOLOGY_PATTERN = /self-hosted|cloud-hosted|--service-layout|serviceLayout|SERVICE_LAYOUT|unified-process|split-services/u;
const PROFILE_ID_PATTERN = /^(?:standalone|cloud)\.(?:development|test|staging|production)$/u;

async function exists(relativePath) {
  try {
    await stat(path.join(ROOT, relativePath));
    return true;
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return false;
    }
    throw error;
  }
}

async function read(relativePath) {
  return readFile(path.join(ROOT, relativePath), 'utf8');
}

async function readJson(relativePath) {
  return JSON.parse((await read(relativePath)).replace(/^\uFEFF/u, ''));
}

async function listFiles(relativePath) {
  const absolutePath = path.join(ROOT, relativePath);
  const entries = await readdir(absolutePath, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const childPath = path.join(relativePath, entry.name);
    if (entry.isDirectory()) {
      files.push(...await listFiles(childPath));
    } else if (entry.isFile()) {
      files.push(childPath);
    }
  }
  return files;
}

function normalizePathForAssert(relativePath) {
  return relativePath.replaceAll('\\', '/');
}

function ingressRuleForHost(manifest, host) {
  const marker = `    - host: ${host}`;
  const start = manifest.indexOf(marker);
  assert.notEqual(start, -1, `ingress rule for ${host} must exist`);
  const nextRule = manifest.indexOf('\n    - host: ', start + marker.length);
  return manifest.slice(start, nextRule === -1 ? manifest.length : nextRule);
}

function parseEnvValues(text) {
  const values = {};
  for (const line of text.split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) {
      continue;
    }
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) {
      continue;
    }
    values[trimmed.slice(0, separatorIndex)] = trimmed.slice(separatorIndex + 1);
  }
  return values;
}

function schemaNameFromRef(ref) {
  return ref?.split('/').pop();
}

function schemaHasTenantInput(schema, schemas, seen = new Set()) {
  if (!schema || typeof schema !== 'object') {
    return false;
  }
  if (schema.$ref) {
    const schemaName = schemaNameFromRef(schema.$ref);
    if (!schemaName || seen.has(schemaName)) {
      return false;
    }
    seen.add(schemaName);
    return schemaHasTenantInput(schemas[schemaName], schemas, seen);
  }
  if (schema.properties?.tenantId || schema.properties?.tenant_id) {
    return true;
  }
  for (const key of ['allOf', 'anyOf', 'oneOf']) {
    if (Array.isArray(schema[key]) && schema[key].some((item) => schemaHasTenantInput(item, schemas, seen))) {
      return true;
    }
  }
  if (schema.items && schemaHasTenantInput(schema.items, schemas, seen)) {
    return true;
  }
  return false;
}

test('declares SDKWork v5 deployment topology spec and profile env files for sdkwork-knowledgebase', async () => {
  assert.equal(await exists('specs/topology.spec.json'), true);
  assert.equal(await exists('scripts/lib/knowledgebase-topology.mjs'), true);
  assert.equal(await exists('scripts/knowledgebase-dev.mjs'), true);
  assert.equal(await exists('docs/architecture/tech/TECH-topology-standard.md'), true);

  const spec = await readJson('specs/topology.spec.json');
  assert.equal(spec.schemaVersion, 5);
  assert.equal(spec.kind, 'sdkwork.app.topology');
  assert.equal(spec.appId, 'sdkwork-knowledgebase');
  assert.equal(spec.archetype, 'application-http-gateway');
  assert.deepEqual(spec.vocabulary.deploymentProfile.allowed, ['standalone', 'cloud']);
  assert.equal(spec.defaults.developmentProfileId, 'standalone.development');
  assert.equal(spec.defaults.productionProfileId, 'cloud.production');
  assert.ok(spec.surfaces['application.public-ingress']);
  assert.ok(spec.surfaces['application.backend-http']);
  assert.ok(spec.surfaces['application.open-http']);
  assert.ok(spec.surfaces['platform.api-gateway']);
  assert.equal(spec.components.appApiRouter.binary, 'sdkwork-api-knowledgebase-standalone-gateway');
  assert.equal(spec.components.backendApiRouter.binary, 'sdkwork-api-knowledgebase-standalone-gateway');
  assert.equal(spec.components.openApiRouter.binary, 'sdkwork-api-knowledgebase-standalone-gateway');

  for (const profileId of [
    'standalone.development',
    'standalone.test',
    'standalone.staging',
    'standalone.production',
    'cloud.development',
    'cloud.test',
    'cloud.staging',
    'cloud.production',
  ]) {
    assert.equal(profileId.split('.').length, 2, `${profileId} must use deploymentProfile.environment`);
    const profilePath = spec.profileFiles[profileId];
    assert.equal(await exists(profilePath), true, `${profilePath} should exist`);
    const profileEnv = await read(profilePath);
    assert.match(profileEnv, /SDKWORK_KNOWLEDGEBASE_PROFILE_ID=/);
    assert.match(profileEnv, /SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE=/);
    assert.doesNotMatch(profileEnv, /HOSTING|self-hosted|cloud-hosted|SERVICE_LAYOUT|unified-process|split-services/);
    assert.match(profileEnv, /VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL=/);
    assert.match(profileEnv, /VITE_SDKWORK_KNOWLEDGEBASE_PLATFORM_API_GATEWAY_HTTP_URL=/);
    if (profileId.startsWith('standalone.')) {
      const values = parseEnvValues(profileEnv);
      assert.equal(
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND,
        '0.0.0.0:18081',
        `${profilePath} must expose standalone Wiki delivery on the LAN bind`,
      );
      assert.equal(
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_URL,
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
        `${profilePath} backend SDK URL must use the standalone public ingress`,
      );
      assert.equal(
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL,
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
        `${profilePath} open SDK URL must use the standalone public ingress`,
      );
      assert.equal(
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_BIND,
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND,
        `${profilePath} backend bind must use the standalone public ingress`,
      );
      assert.equal(
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_BIND,
        values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND,
        `${profilePath} open bind must use the standalone public ingress`,
      );
      assert.equal(
        values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_URL,
        values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
        `${profilePath} browser backend SDK URL must use the standalone public ingress`,
      );
      assert.equal(
        values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL,
        values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
        `${profilePath} browser open SDK URL must use the standalone public ingress`,
      );
    }
  }
});

test('topology governance files do not retain retired deployment profile segments', async () => {
  const spec = await readJson('specs/topology.spec.json');
  const expectedProfileIds = Object.keys(spec.profileFiles).sort();
  assert.deepEqual(expectedProfileIds, [
    'cloud.development',
    'cloud.production',
    'cloud.staging',
    'cloud.test',
    'standalone.development',
    'standalone.production',
    'standalone.staging',
    'standalone.test',
  ]);

  const topologyEnvFiles = (await listFiles('etc/topology'))
    .map(normalizePathForAssert)
    .filter((relativePath) => relativePath.endsWith('.env'))
    .sort();
  assert.deepEqual(
    topologyEnvFiles,
    expectedProfileIds.map((profileId) => `etc/topology/${profileId}.env`).sort(),
  );

  for (const relativePath of topologyEnvFiles) {
    const profileId = path.basename(relativePath, '.env');
    assert.match(profileId, PROFILE_ID_PATTERN, `${relativePath} must use deploymentProfile.environment`);
    const profileEnv = await read(relativePath);
    assert.doesNotMatch(profileEnv, RETIRED_TOPOLOGY_PATTERN, relativePath);
  }

  const deployment = await read('deployments/deploy.yaml');
  assert.doesNotMatch(deployment, RETIRED_TOPOLOGY_PATTERN);
  const defaultProfile = deployment.match(/^defaultProfile:\s*(\S+)\s*$/mu)?.[1];
  assert.match(defaultProfile ?? '', PROFILE_ID_PATTERN, 'deployments/deploy.yaml defaultProfile must use a canonical profile id');

  const deploymentProfileIds = [...deployment.matchAll(/^  ([A-Za-z0-9.-]+):\s*$/gmu)]
    .map((match) => match[1])
    .sort();
  // Development profiles remain source config under etc/ and are never
  // deploy targets; cloud deploys test/staging/production, standalone
  // deploys production only (customer-managed host installs).
  const expectedDeploymentProfileIds = expectedProfileIds
    .filter((profileId) => {
      const [deploymentProfile, environment] = profileId.split('.');
      if (deploymentProfile === 'standalone') {
        return environment === 'production';
      }
      return environment !== 'development';
    })
    .sort();
  assert.deepEqual(deploymentProfileIds, expectedDeploymentProfileIds);
  assert.ok(
    deploymentProfileIds.includes(defaultProfile),
    'deployments/deploy.yaml defaultProfile must reference a declared production profile',
  );

  const networkPolicy = await read('deployments/kubernetes/networkpolicy.yaml');
  assert.doesNotMatch(networkPolicy, RETIRED_TOPOLOGY_PATTERN);
});

test('root package.json wires @sdkwork/app-topology and standard dev scripts', async () => {
  const packageJson = await readJson('package.json');
  assert.equal(packageJson.dependencies['@sdkwork/app-topology'], 'workspace:*');
  const workspaceYaml = await read('pnpm-workspace.yaml');
  assert.match(workspaceYaml, /["']\.\.\/sdkwork-app-topology["']/);
  assert.match(packageJson.scripts['dev:browser'], /dev:browser:postgres:standalone/);
  assert.match(packageJson.scripts['dev:desktop'], /dev:desktop:postgres:standalone/);
  assert.match(packageJson.scripts['dev:browser:postgres:standalone'], /pnpm exec sdkwork-app dev/);
  assert.match(packageJson.scripts['dev:browser:postgres:standalone'], /--runtime-target browser/);
  assert.match(packageJson.scripts['dev:browser:postgres:standalone'], /--deployment-profile standalone/);
  assert.doesNotMatch(packageJson.scripts['dev:browser:postgres:standalone'], /--service-layout|unified-process|split-services/);
  assert.equal(packageJson.scripts['knowledgebase:dev'], undefined);
  assert.equal(packageJson.scripts['knowledgebase:dev:cloud'], undefined);
  assert.match(packageJson.scripts['topology:validate'], /sdkwork-topology\.mjs validate/);
});

test('declares cloud gateway config bundles referenced by topology spec', async () => {
  const spec = await readJson('specs/topology.spec.json');
  for (const configFile of spec.packaging.cloudConfigFiles) {
    const configPath = path.join('etc', configFile);
    assert.equal(await exists(configPath), true, `${configPath} should exist`);
  }
});

test('knowledgebase dev orchestrator uses orchestration spec and gateway config', async () => {
  const devScript = await read('scripts/knowledgebase-dev.mjs');
  assert.match(devScript, /listOrchestrationProcesses/);
  assert.match(devScript, /buildProcessesFromOrchestration/);
  assert.doesNotMatch(devScript, /createPlatformGatewayProcess/);
  assert.match(devScript, /resolveIamDevEnv/);
  assert.match(devScript, /IAM_APPLICATION_BOOTSTRAP_ENV/);
  const retiredCloudGatewayToken = 'sdkwork-api-' + 'cloud-gateway';
  assert.doesNotMatch(devScript, new RegExp(`--config|${retiredCloudGatewayToken}`));
  assert.match(devScript, /--deployment-profile/);
  assert.match(devScript, /--database/);
  assert.match(devScript, /settings\.database !== 'postgres'/);
  assert.doesNotMatch(devScript, /postgres\|sqlite|resolveDefaultSqliteDatabaseUrl/);
  assert.doesNotMatch(devScript, /SDKWORK_DATABASE_ENGINE:\s*'sqlite'/);
  assert.doesNotMatch(devScript, /--hosting/);
  assert.doesNotMatch(devScript, /self-hosted|cloud-hosted|--service-layout|serviceLayout|unified-process|split-services/);
});

test('knowledgebase topology adapter exports IAM application bootstrap env aliases', async () => {
  const topologyAdapter = await read('scripts/lib/knowledgebase-topology.mjs');
  assert.match(topologyAdapter, /export const IAM_APPLICATION_BOOTSTRAP_ENV/u);
  assert.match(topologyAdapter, /SDKWORK_KNOWLEDGEBASE_APP_ROOT/u);
});

test('default PostgreSQL development path is not blocked by SQLite-only runtime wiring', async () => {
  const runtimeSource = await read('crates/sdkwork-routes-knowledgebase-app-api/src/runtime.rs');
  const gatewayMain = await read('crates/sdkwork-api-knowledgebase-standalone-gateway/src/bin/app_main.rs');
  const gatewayAssembly = await read('crates/sdkwork-api-knowledgebase-assembly/src/bootstrap.rs');

  assert.doesNotMatch(
    runtimeSource,
    /postgresql SDKWORK_DATABASE_URL is not wired to HTTP handlers yet|use sqlite/i,
  );
  assert.doesNotMatch(
    `${gatewayMain}\n${gatewayAssembly}`,
    /KnowledgebaseSqliteRuntime::connect|initialize knowledgebase sqlite runtime/,
  );
  assert.match(gatewayAssembly, /build_backend_business_router\(\)/);
  assert.match(gatewayAssembly, /build_open_business_router\(\)/);
  assert.match(gatewayAssembly, /ApiAssemblyContribution::from_manifest/);
  assert.doesNotMatch(gatewayAssembly, /_with_web_framework/);
  assert.match(gatewayMain, /composed\.into_hosted\(framework\)/);
});

test('pc runtime config does not expose retired application deployment fields', async () => {
  const runtimeFiles = [
    'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-core/src/config/runtimeConfig.ts',
    'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-shell/src/SettingsModal.tsx',
    'apps/sdkwork-knowledgebase-pc/src/bootstrap/knowledgebaseIamRuntime.ts',
    'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-knowledge/src/runtime/locale.ts',
    'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-knowledge/src/i18n/en-US/intelligence/knowledge/shell.json',
    'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-knowledge/src/i18n/zh-CN/intelligence/knowledge/shell.json',
  ];

  for (const relativePath of runtimeFiles) {
    const text = await read(relativePath);
    assert.doesNotMatch(text, /KnowledgebaseRuntimeConfig\['deploymentMode'\]/, relativePath);
    assert.doesNotMatch(text, /\bconfig\.(?:deploymentMode|hosting)\b/, relativePath);
    assert.doesNotMatch(
      text,
      /VITE_SDKWORK_KNOWLEDGEBASE_(?:DEPLOYMENT_MODE|HOSTING)/,
      relativePath,
    );
    assert.doesNotMatch(text, /"hosting"\s*:/, relativePath);
    assert.doesNotMatch(text, /\b(?:self-hosted|cloud-hosted)\b|HOSTING|DEPLOYMENT_MODE/, relativePath);
  }
});

test('OpenAPI and generated SDK inputs do not expose current tenant selectors', async () => {
  const openapiFiles = [
    'apis/open-api/knowledgebase-open-api.openapi.json',
    'apis/app-api/knowledgebase-app-api.openapi.json',
    'apis/backend-api/knowledgebase-backend-api.openapi.json',
    'sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json',
    'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
    'sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json',
  ];

  const openapiViolations = [];
  for (const relativePath of openapiFiles) {
    const spec = await readJson(relativePath);
    const schemas = spec.components?.schemas ?? {};
    for (const [routePath, pathItem] of Object.entries(spec.paths ?? {})) {
      const pathParameters = pathItem.parameters ?? [];
      for (const [method, operation] of Object.entries(pathItem)) {
        if (!['get', 'put', 'post', 'delete', 'patch', 'head', 'options', 'trace'].includes(method)) {
          continue;
        }

        for (const parameter of [...pathParameters, ...(operation.parameters ?? [])]) {
          if (parameter?.name === 'tenantId' || parameter?.name === 'tenant_id') {
            openapiViolations.push(`${relativePath} ${method.toUpperCase()} ${routePath} parameter ${parameter.in}.${parameter.name}`);
          }
        }

        const content = operation.requestBody?.content ?? {};
        for (const [contentType, media] of Object.entries(content)) {
          if (schemaHasTenantInput(media.schema, schemas)) {
            const schemaName = schemaNameFromRef(media.schema?.$ref) ?? '<inline>';
            openapiViolations.push(`${relativePath} ${method.toUpperCase()} ${routePath} ${contentType} requestBody ${schemaName}`);
          }
        }
      }
    }
  }

  assert.deepEqual(openapiViolations, []);

  const generatedRoots = [
    'sdks/sdkwork-knowledgebase-sdk/sdkwork-knowledgebase-sdk-typescript/generated/server-openapi',
    'sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi',
    'sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi',
  ];
  const generatedViolations = [];
  for (const generatedRoot of generatedRoots) {
    if (!await exists(generatedRoot)) {
      continue;
    }
    const generatedFiles = (await listFiles(generatedRoot)).filter((relativePath) => {
      const normalized = relativePath.replaceAll('\\', '/');
      return (
        normalized.includes('/src/api/')
        || /\/src\/types\/.+request\.ts$/u.test(normalized)
        || /\/src\/types\/.+params\.ts$/u.test(normalized)
        || /\/src\/types\/.+options\.ts$/u.test(normalized)
      );
    });
    for (const relativePath of generatedFiles) {
      const text = await read(relativePath);
      if (/\btenantId\b|\btenant_id\b/u.test(text)) {
        generatedViolations.push(relativePath);
      }
    }
  }

  assert.deepEqual(generatedViolations, []);
});

test('production cloud topology orchestrates background worker and health probes', async () => {
  const spec = await readJson('specs/topology.spec.json');
  const productionProfile = spec.orchestration.profiles['cloud.production'];
  assert.ok(productionProfile, 'cloud.production orchestration profile must exist');

  const processIds = productionProfile.processes.map((process) => process.id);
  assert.equal(new Set(processIds).size, processIds.length, 'process ids must be unique');
  assert.ok(!processIds.includes('platform.api-gateway'));
  assert.ok(processIds.includes('application.background-worker'));
  assert.ok(processIds.includes('application.public-ingress'));
  assert.equal(processIds.includes('application.backend-http'), false);
  assert.equal(processIds.includes('application.open-http'), false);
  assert.deepEqual(productionProfile.healthSurfaces, [
    'platform.api-gateway',
    'application.public-ingress',
  ]);

  const bootstrapSource = await read('crates/sdkwork-routes-knowledgebase-app-api/src/bootstrap.rs');
  assert.match(bootstrapSource, /validate_snowflake_node_id_for_production/);
  assert.match(
    bootstrapSource,
    /SDKWORK_KNOWLEDGEBASE_ORGANIZATION_ID must be set when SDKWORK_KNOWLEDGEBASE_ENVIRONMENT is not development/,
  );
  assert.doesNotMatch(bootstrapSource, /DEV_AUTH_BYPASS/);
  assert.doesNotMatch(bootstrapSource, /SDKWORK_KNOWLEDGEBASE_SERVICE_LAYOUT|is_unified_process_layout/);

  const workerSource = await read('crates/sdkwork-knowledgebase-worker/src/lib.rs');
  assert.match(workerSource, /shutdown_signal/);

  const ingressManifest = await read('deployments/kubernetes/app-api-deployment.yaml');
  assert.match(ingressManifest, /path: \/livez/);
  assert.match(ingressManifest, /path: \/readyz/);
  assert.doesNotMatch(ingressManifest, /KB_API_BINARY/);
  assert.match(ingressManifest, /SDKWORK_NODE_INSTANCE_ID/);
  assert.doesNotMatch(ingressManifest, /SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID/);
  assert.equal(await exists('deployments/kubernetes/backend-api-deployment.yaml'), false);
  assert.equal(await exists('deployments/kubernetes/open-api-deployment.yaml'), false);

  const apiDockerfile = await read('Dockerfile');
  // The committed container build uses the pre-assembled staging pattern
  // (scripts/build-knowledgebase-container.mjs builds bin/ outside the image
  // and the Dockerfile COPYs the staged install package), not an in-image
  // multi-stage cargo build.
  assert.match(apiDockerfile, /ARG GATEWAY_BINARY=sdkwork-api-knowledgebase-standalone-gateway/);
  assert.match(apiDockerfile, /COPY \. \$\{INSTALL_ROOT\}/);
  assert.match(
    apiDockerfile,
    /ENTRYPOINT \["\/opt\/sdkwork\/knowledgebase\/bin\/sdkwork-api-knowledgebase-standalone-gateway"\]/,
  );
  assert.doesNotMatch(apiDockerfile, /sdkwork-knowledgebase-api-server|KB_API_BINARY/);

  const ingressRouting = await read('deployments/kubernetes/ingress.yaml');
  assert.equal(
    ingressRouting.match(/name: sdkwork-knowledgebase-app-api/gu)?.length,
    4,
  );
  assert.equal(ingressRouting.match(/path: \/internal\/v3\/api/gu)?.length, 1);

  const publicIngressRule = ingressRuleForHost(ingressRouting, 'knowledgebase.sdkwork.com');
  const adminIngressRule = ingressRuleForHost(ingressRouting, 'knowledgebase-admin.sdkwork.com');
  const openIngressRule = ingressRuleForHost(ingressRouting, 'knowledge.sdkwork.com');
  assert.match(publicIngressRule, /path: \/app\/v3\/api/u);
  assert.match(publicIngressRule, /path: \/internal\/v3\/api/u);
  assert.doesNotMatch(adminIngressRule, /path: \/internal\/v3\/api/u);
  assert.doesNotMatch(openIngressRule, /path: \/internal\/v3\/api/u);
  for (const manifestPath of [
    'deployments/kubernetes/hpa.yaml',
    'deployments/kubernetes/poddisruptionbudget.yaml',
    'deployments/kubernetes/servicemonitor.yaml',
  ]) {
    const manifest = await read(manifestPath);
    assert.doesNotMatch(manifest, /sdkwork-knowledgebase-(?:backend|open)-api/);
  }

  const workerManifest = await read('deployments/kubernetes/worker-deployment.yaml');
  assert.match(workerManifest, /terminationGracePeriodSeconds/);
  assert.match(workerManifest, /SDKWORK_NODE_INSTANCE_ID/);
  assert.match(workerManifest, /SDKWORK_KNOWLEDGEBASE_WORKER_INGESTION_JOB_LEASE_SECONDS/);
  assert.doesNotMatch(workerManifest, /SDKWORK_KNOWLEDGEBASE_SNOWFLAKE_NODE_ID/);

  const runtimeSource = await read('crates/sdkwork-routes-knowledgebase-app-api/src/runtime.rs');
  assert.match(runtimeSource, /SnowflakeNodeAllocator::allocate_process_generator/);
  assert.match(runtimeSource, /snowflake node lease is unhealthy/);
});

test('standalone gateway reads the single application ingress bind env key', async () => {
  const gatewayCargo = await read('crates/sdkwork-api-knowledgebase-standalone-gateway/Cargo.toml');
  const appMain = await read('crates/sdkwork-api-knowledgebase-standalone-gateway/src/bin/app_main.rs');
  const gatewayAssembly = await read('crates/sdkwork-api-knowledgebase-assembly/src/bootstrap.rs');
  const routeRuntime = await read('crates/sdkwork-routes-knowledgebase-app-api/src/runtime.rs');
  const routeBootstrap = await read('crates/sdkwork-routes-knowledgebase-app-api/src/bootstrap.rs');

  assert.match(gatewayCargo, /name = "sdkwork-api-knowledgebase-standalone-gateway"/);
  assert.match(gatewayCargo, /path = "src\/bin\/app_main\.rs"/);
  assert.doesNotMatch(gatewayCargo, /backend_main\.rs|open_main\.rs/);
  assert.match(appMain, /SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND/);
  assert.doesNotMatch(appMain, /SDKWORK_KNOWLEDGEBASE_APP_LISTEN/);
  assert.doesNotMatch(appMain, /SDKWORK_KNOWLEDGEBASE_BACKEND_LISTEN|SDKWORK_KNOWLEDGEBASE_OPEN_LISTEN/);
  assert.match(gatewayAssembly, /pub async fn assemble_api_router/);
  assert.match(gatewayAssembly, /runtime\.build_full_app_router\(\)/);
  assert.match(gatewayAssembly, /runtime\.build_backend_business_router\(\)/);
  assert.match(gatewayAssembly, /runtime\.build_open_business_router\(\)/);
  assert.match(appMain, /ComposedApiAssembly::try_compose/);
  assert.match(appMain, /composed\.into_hosted\(framework\)/);
  assert.doesNotMatch(routeRuntime, /build_[a-z_]+_router_with_web_framework/);
  assert.doesNotMatch(routeBootstrap, /build_served_(?:app|backend|open)_router/);
});
