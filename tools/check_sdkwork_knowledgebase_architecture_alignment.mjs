#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const failures = [];
const warnings = [];
const retiredTopologyPattern = /self-hosted|cloud-hosted|--service-layout|serviceLayout|SERVICE_LAYOUT|unified-process|split-services/u;
const v5TopologyProfileIdPattern = /^(?:standalone|cloud)\.(?:development|test|staging|production)$/u;

function readText(relativePath) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    failures.push(`${relativePath} must exist`);
    return '';
  }
  return fs.readFileSync(absolutePath, 'utf8');
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
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

function assert(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}

function listFilesRecursive(relativePath) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    failures.push(`${relativePath}/ must exist`);
    return [];
  }

  const files = [];
  for (const entry of fs.readdirSync(absolutePath, { withFileTypes: true })) {
    const childPath = path.join(relativePath, entry.name);
    if (entry.isDirectory()) {
      files.push(...listFilesRecursive(childPath));
    } else if (entry.isFile()) {
      files.push(childPath.replaceAll('\\', '/'));
    }
  }
  return files;
}

function assertDirectory(relativePath) {
  assert(fs.existsSync(path.join(repoRoot, relativePath)), `${relativePath}/ must exist`);
}

function assertCargoDependsOnWebFramework(relativeCrateToml) {
  const text = readText(relativeCrateToml);
  assert(
    text.includes('sdkwork-web-axum.workspace = true')
      || text.includes('sdkwork-web-axum = {'),
    `${relativeCrateToml} must depend on sdkwork-web-axum per WEB_FRAMEWORK_SPEC.md`,
  );
}

const requiredDirectories = [
  'apis',
  'apps',
  'crates',
  'sdks',
  'deployments',
  'etc',
  'scripts',
  'docs',
  'tests',
  '.sdkwork',
  'specs',
];

for (const directory of requiredDirectories) {
  assertDirectory(directory);
}

assert(
  !fs.existsSync(path.join(repoRoot, 'configs')),
  'configs/ is retired; source-controlled application configuration must exist only under etc/',
);

assert(fs.existsSync(path.join(repoRoot, 'agent.app.config.json')), 'agent.app.config.json must exist');
assert(fs.existsSync(path.join(repoRoot, 'sdkwork.workflow.json')), 'sdkwork.workflow.json must exist');
assert(fs.existsSync(path.join(repoRoot, 'package.json')), 'package.json must exist per PNPM_SCRIPT_SPEC.md');
assert(
  fs.existsSync(path.join(repoRoot, '.github/workflows/package.yml')),
  '.github/workflows/package.yml must exist per GITHUB_WORKFLOW_SPEC.md',
);
assert(fs.existsSync(path.join(repoRoot, 'pnpm-workspace.yaml')), 'repository root pnpm-workspace.yaml must exist per APP_COMPOSITION_SPEC.md');
assert(
  !fs.existsSync(path.join(repoRoot, 'apps/sdkwork-knowledgebase-pc/pnpm-workspace.yaml')),
  'nested app-level pnpm-workspace.yaml is forbidden per APP_COMPOSITION_SPEC.md',
);
assert(
  !fs.existsSync(path.join(repoRoot, 'apps/_pc26-merge')),
  'legacy apps/_pc26-merge extraction directory must not remain in workspace',
);
assert(
  !fs.existsSync(path.join(repoRoot, 'apps/_pc26-extract')),
  'legacy apps/_pc26-extract extraction directory must not remain in workspace',
);

const packageJson = readJson('package.json');
assert(packageJson.scripts?.['check:app-composition'], 'package.json must expose pnpm check:app-composition');
for (const script of ['dev', 'build', 'test', 'check', 'verify', 'clean']) {
  assert(packageJson.scripts?.[script], `package.json must expose pnpm ${script}`);
}

for (const script of [
  'dev:browser',
  'dev:browser:postgres:standalone',
  'dev:desktop',
  'dev:desktop:postgres:standalone',
  'check:pnpm-script-standard',
  'check:agent-workflow-standard',
]) {
  assert(packageJson.scripts?.[script], `package.json must expose pnpm ${script}`);
}

for (const [scriptName, command] of Object.entries(packageJson.scripts ?? {})) {
  const firstSegment = scriptName.split(':')[0];
  assert(
    !['knowledgebase', 'kb', 'desktop', 'tauri'].includes(firstSegment),
    `package.json script ${scriptName} must use SDKWork action-first naming`,
  );
  assert(
    !/(--hosting|--service-layout|\bself-hosted\b|\bcloud-hosted\b|\bunified-process\b|\bsplit-services\b|\bserviceLayout\b)/u.test(String(command)),
    `package.json script ${scriptName} must not use retired deployment command values`,
  );
}

for (const capabilityScript of ['api:materialize:check', 'sdk:check', 'db:validate', 'topology:validate']) {
  assert(packageJson.scripts?.[capabilityScript], `package.json must expose pnpm ${capabilityScript}`);
}

const cargoToml = readText('Cargo.toml');
assert(cargoToml.includes('sdkwork-web-core'), 'Cargo.toml must declare sdkwork-web-core');
assert(cargoToml.includes('sdkwork-web-axum'), 'Cargo.toml must declare sdkwork-web-axum');
assert(cargoToml.includes('sdkwork-iam-web-adapter'), 'Cargo.toml must declare sdkwork-iam-web-adapter');
assert(cargoToml.includes('sdkwork-database-config'), 'Cargo.toml must declare sdkwork-database-config');
assert(cargoToml.includes('sdkwork-database-sqlx'), 'Cargo.toml must declare sdkwork-database-sqlx');
assert(cargoToml.includes('sdkwork-database-repository'), 'Cargo.toml must declare sdkwork-database-repository');
assert(cargoToml.includes('sdkwork-api-knowledgebase-standalone-gateway'), 'Cargo.toml must include sdkwork-api-knowledgebase-standalone-gateway');
assert(
  cargoToml.includes('sdkwork-intelligence-knowledgebase-repository-sqlx'),
  'Cargo.toml must include repository-sqlx crate',
);
assert(cargoToml.includes('sdkwork-utils-rust'), 'Cargo.toml must declare sdkwork-utils-rust');
assert(cargoToml.includes('sdkwork-id-core'), 'Cargo.toml must declare sdkwork-id-core');
assert(!cargoToml.includes('sdkwork-discovery'), 'sdkwork-discovery is not required until RPC services exist');

const cargoWorkspace = readText('Cargo.toml');
const workspaceMembersBlock = cargoWorkspace.match(/members\s*=\s*\[([\s\S]*?)\]/u)?.[1] ?? '';
for (const member of [...workspaceMembersBlock.matchAll(/"([^"]+)"/gu)].map((match) => match[1])) {
  if (!member.startsWith('crates/')) {
    continue;
  }
  assert(
    fs.existsSync(path.join(repoRoot, member, 'specs/component.spec.json')),
    `${member}/specs/component.spec.json must exist per COMPONENT_SPEC.md`,
  );
}

const workflow = readJson('sdkwork.workflow.json');
const dependencyIds = new Set((workflow.dependencies || []).map((dependency) => dependency.id));
for (const dependencyId of [
  'sdkwork-appbase',
  'sdkwork-database',
  'sdkwork-web-framework',
  'sdkwork-utils',
  'sdkwork-sdk-generator',
  'sdkwork-app-topology',
  'sdkwork-drive',
  'sdkwork-kernel',
  'sdkwork-memory',
]) {
  assert(dependencyIds.has(dependencyId), `sdkwork.workflow.json must declare ${dependencyId}`);
}
assert(!dependencyIds.has('sdkwork-discovery'), 'sdkwork.workflow.json must not declare sdkwork-discovery until RPC exists');
const targetIds = new Set((workflow.targets || []).map((target) => target.id));
for (const targetId of [
  'linux-x64-standalone-server-tar-gz',
  'web-universal-cloud-browser-zip',
]) {
  assert(targetIds.has(targetId), `sdkwork.workflow.json must declare target ${targetId}`);
}
for (const target of workflow.targets || []) {
  assert(
    ['standalone', 'cloud'].includes(target.deploymentProfile),
    `sdkwork.workflow.json target ${target.id} must declare canonical deploymentProfile`,
  );
  assert(target.runtimeTarget, `sdkwork.workflow.json target ${target.id} must declare runtimeTarget`);
  assert(target.runner, `sdkwork.workflow.json target ${target.id} must declare runner`);
  assert(
    Array.isArray(target.outputGlobs) && target.outputGlobs.length > 0,
    `sdkwork.workflow.json target ${target.id} must declare outputGlobs`,
  );
}

const topologySpec = readJson('specs/topology.spec.json');
const expectedTopologyProfileIds = [
  'cloud.development',
  'cloud.production',
  'cloud.staging',
  'cloud.test',
  'standalone.development',
  'standalone.production',
  'standalone.staging',
  'standalone.test',
];
const expectedDeploymentProfileIds = expectedTopologyProfileIds
  .filter((profileId) => profileId.endsWith('.production'))
  .sort();
assert(
  topologySpec.vocabulary?.deploymentProfile?.allowed?.join(',') === 'standalone,cloud',
  'specs/topology.spec.json must use deploymentProfile standalone/cloud vocabulary',
);
assert(
  topologySpec.defaults?.developmentProfileId === 'standalone.development',
  'specs/topology.spec.json must default development to standalone.development',
);
assert(
  topologySpec.defaults?.productionProfileId === 'cloud.production',
  'specs/topology.spec.json must default production to cloud.production',
);
assert(
  JSON.stringify(Object.keys(topologySpec.profileFiles ?? {}).sort()) === JSON.stringify(expectedTopologyProfileIds),
  'specs/topology.spec.json profileFiles must declare only v5 topology profile ids',
);
const topologyEnvFiles = listFilesRecursive('etc/topology')
  .filter((relativePath) => relativePath.endsWith('.env'))
  .sort();
assert(
  JSON.stringify(topologyEnvFiles) === JSON.stringify(
    expectedTopologyProfileIds.map((profileId) => `etc/topology/${profileId}.env`).sort(),
  ),
  'etc/topology must contain only v5 deploymentProfile.environment env files',
);
for (const relativePath of topologyEnvFiles) {
  const profileId = path.basename(relativePath, '.env');
  assert(
    v5TopologyProfileIdPattern.test(profileId),
    `${relativePath} must use deploymentProfile.environment profile id`,
  );
  const profileEnvText = readText(relativePath);
  assert(
    !retiredTopologyPattern.test(profileEnvText),
    `${relativePath} must not contain retired topology vocabulary`,
  );
  if (profileId.startsWith('standalone.')) {
    const values = parseEnvValues(profileEnvText);
    assert(
      values.SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_URL
        === values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
      `${relativePath} backend SDK URL must use the standalone public ingress`,
    );
    assert(
      values.SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL
        === values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
      `${relativePath} open SDK URL must use the standalone public ingress`,
    );
    assert(
      values.SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_BIND
        === values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND,
      `${relativePath} backend bind must use the standalone public ingress`,
    );
    assert(
      values.SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_BIND
        === values.SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_INGRESS_BIND,
      `${relativePath} open bind must use the standalone public ingress`,
    );
    assert(
      values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_URL
        === values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
      `${relativePath} browser backend SDK URL must use the standalone public ingress`,
    );
    assert(
      values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL
        === values.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL,
      `${relativePath} browser open SDK URL must use the standalone public ingress`,
    );
  }
}

const deploymentYaml = readText('deployments/deploy.yaml');
assert(
  !retiredTopologyPattern.test(deploymentYaml),
  'deployments/deploy.yaml must not contain retired topology vocabulary',
);
const defaultDeploymentProfileId = deploymentYaml.match(/^defaultProfile:\s*(\S+)\s*$/mu)?.[1] ?? '';
assert(
  v5TopologyProfileIdPattern.test(defaultDeploymentProfileId),
  'deployments/deploy.yaml defaultProfile must use a v5 topology profile id',
);
const deploymentProfileIds = [...deploymentYaml.matchAll(/^  ([A-Za-z0-9.-]+):\s*$/gmu)]
  .map((match) => match[1])
  .sort();
assert(
  JSON.stringify(deploymentProfileIds) === JSON.stringify(expectedDeploymentProfileIds),
  'deployments/deploy.yaml profiles must match the production topology profile ids',
);
assert(
  deploymentProfileIds.includes(defaultDeploymentProfileId),
  'deployments/deploy.yaml defaultProfile must reference a declared production deployment profile',
);
assert(
  !retiredTopologyPattern.test(readText('deployments/kubernetes/networkpolicy.yaml')),
  'deployments/kubernetes/networkpolicy.yaml must not contain retired topology vocabulary',
);

const routerCrates = [
  'crates/sdkwork-routes-knowledgebase-open-api/Cargo.toml',
  'crates/sdkwork-routes-knowledgebase-app-api/Cargo.toml',
  'crates/sdkwork-routes-knowledgebase-backend-api/Cargo.toml',
];

for (const routerCrate of routerCrates) {
  assertCargoDependsOnWebFramework(routerCrate);
  const crateName = path.basename(path.dirname(routerCrate));
  assert(
    fs.existsSync(path.join(repoRoot, `crates/${crateName}/src/web_bootstrap.rs`)),
    `${crateName} must provide web_bootstrap.rs`,
  );
}

const repositorySqlxToml = readText('crates/sdkwork-intelligence-knowledgebase-repository-sqlx/Cargo.toml');
assert(
  repositorySqlxToml.includes('sdkwork-database-sqlx'),
  'repository-sqlx crate must depend on sdkwork-database-sqlx',
);
assert(
  repositorySqlxToml.includes('sdkwork-database-repository'),
  'repository-sqlx crate must depend on sdkwork-database-repository per DATABASE_SPEC.md',
);
assert(
  repositorySqlxToml.includes('migrate'),
  'repository-sqlx sqlx dependency must enable migrate feature',
);

const serviceToml = readText('crates/sdkwork-intelligence-knowledgebase-service/Cargo.toml');
assert(
  serviceToml.includes('sdkwork-utils-rust'),
  'service crate must depend on sdkwork-utils-rust for shared utility helpers',
);

const repositorySqlxUtilsToml = readText('crates/sdkwork-intelligence-knowledgebase-repository-sqlx/Cargo.toml');
assert(
  repositorySqlxUtilsToml.includes('sdkwork-utils-rust'),
  'repository-sqlx crate must depend on sdkwork-utils-rust',
);

const agentProviderToml = readText('crates/sdkwork-knowledgebase-agent-provider/Cargo.toml');
assert(
  agentProviderToml.includes('sdkwork-utils-rust'),
  'agent-provider crate must depend on sdkwork-utils-rust',
);

const repositoryBootstrap = readText(
  'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/db/bootstrap.rs',
);
assert(
  repositoryBootstrap.includes('sdkwork_database_config'),
  'repository bootstrap must use sdkwork-database-config',
);
assert(
  repositoryBootstrap.includes('sdkwork_database_sqlx'),
  'repository bootstrap must use sdkwork-database-sqlx',
);

const componentSpec = readJson('specs/component.spec.json');
assert(
  Array.isArray(componentSpec.contracts?.sdkDependencies),
  'specs/component.spec.json must explicitly declare sdkDependencies',
);

const routeManifestPaths = [
  'sdks/_route-manifests/open-api/sdkwork-routes-knowledgebase-open-api.route-manifest.json',
  'sdks/_route-manifests/app-api/sdkwork-routes-knowledgebase-app-api.route-manifest.json',
  'sdks/_route-manifests/backend-api/sdkwork-routes-knowledgebase-backend-api.route-manifest.json',
  'sdks/_route-manifests/internal-api/sdkwork-routes-knowledgebase-internal-api.route-manifest.json',
];

for (const relativePath of routeManifestPaths) {
  const manifest = readJson(relativePath);
  for (const route of manifest.routes ?? []) {
    assert(
      route.requestContext === 'WebRequestContext',
      `${relativePath} route ${route.method} ${route.path} must declare WebRequestContext`,
    );
    assert(
      ['open-api', 'app-api', 'backend-api', 'internal-api'].includes(route.apiSurface),
      `${relativePath} route ${route.method} ${route.path} must declare canonical apiSurface`,
    );
  }
}

assert(componentSpec.component.type === 'web-backend-service', 'component type must be web-backend-service');
assert(componentSpec.component.domain === 'intelligence', 'component domain must be intelligence');
assert(componentSpec.component.capability === 'knowledgebase', 'component capability must be knowledgebase');

const canonicalSpecs = (componentSpec.canonicalSpecs || []).map((entry) => entry.file);
for (const specFile of [
  'WEB_FRAMEWORK_SPEC.md',
  'WEB_BACKEND_SPEC.md',
  'DATABASE_SPEC.md',
  'DEPLOYMENT_SPEC.md',
  'API_SPEC.md',
  'SDK_SPEC.md',
]) {
  assert(canonicalSpecs.includes(specFile), `specs/component.spec.json must reference ${specFile}`);
}

const openapiPaths = [
  'sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json',
  'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
  'sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json',
];

for (const relativePath of openapiPaths) {
  const openapi = readJson(relativePath);
  let hasSurface = false;
  for (const pathItem of Object.values(openapi.paths ?? {})) {
    for (const operation of Object.values(pathItem ?? {})) {
      if (operation && typeof operation === 'object' && operation.operationId) {
        assert(
          operation['x-sdkwork-request-context'] === 'WebRequestContext',
          `${relativePath} operation ${operation.operationId} must declare WebRequestContext`,
        );
        assert(
          ['open-api', 'app-api', 'backend-api'].includes(operation['x-sdkwork-api-surface']),
          `${relativePath} operation ${operation.operationId} must declare canonical x-sdkwork-api-surface`,
        );
        hasSurface = true;
      }
    }
  }
  if (!hasSurface) {
    assert(false, `${relativePath} must declare x-sdkwork-api-surface on operations`);
  }
}

const requiredSkeletonPaths = [
  'apis/README.md',
  'apis/authority-manifest.json',
  'apis/rpc/README.md',
  'deployments/README.md',
  'etc/README.md',
  'scripts/README.md',
  'apps/README.md',
  'apps/sdkwork-knowledgebase-pc/AGENTS.md',
  'apps/sdkwork-knowledgebase-pc/agent.app.config.json',
  'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-core/specs/component.spec.json',
  'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-core/src/sdk/sdkContractTypes.ts',
  'pnpm-workspace.yaml',
  'specs/topology.spec.json',
  'specs/knowledge-engine-spi.spec.json',
  'specs/external-knowledge-engine-catalog.spec.json',
  'external/knowledge-engines/catalog.manifest.json',
];

for (const relativePath of requiredSkeletonPaths) {
  assert(
    fs.existsSync(path.join(repoRoot, relativePath)),
    `${relativePath} must exist per SDKWORK_WORKSPACE_SPEC.md skeleton`,
  );
}

const pcAppConfig = readJson('apps/sdkwork-knowledgebase-pc/agent.app.config.json');
assert(
  pcAppConfig.packages?.workspace === '../../pnpm-workspace.yaml',
  'apps/sdkwork-knowledgebase-pc/agent.app.config.json must reference repository root pnpm-workspace.yaml',
);

const pcCoreComponentSpec = readJson(
  'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-core/specs/component.spec.json',
);
const pcCoreSdkInventory = readText(
  'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-core/src/composition/sdk-inventory.ts',
);
const pcCoreSdkWorkspaces = (pcCoreComponentSpec.contracts?.sdkDependencies ?? []).map(
  (entry) => entry.workspace,
);
for (const workspace of pcCoreSdkWorkspaces) {
  assert(
    pcCoreSdkInventory.includes(`'${workspace}'`),
    `sdk-inventory.ts must list ${workspace} from pc-core component.spec.json#contracts.sdkDependencies`,
  );
}
assert(
  pcCoreComponentSpec.contracts?.sdkDependencies?.every((entry) => entry.surface && entry.credentialMode),
  'pc-core component.spec.json sdkDependencies must declare surface and credentialMode per APP_COMPOSITION_SPEC.md',
);

const requiredIngestMetadataAlignment = [
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-service/src/ports/knowledge_drive_object_ref_store.rs',
    symbols: ['managed_drive_object_ref_record', 'MANAGED_DRIVE_ACCESS_MODE'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-service/src/ports/markdown_index_metadata_store.rs',
    symbols: ['MarkdownIndexSourceBinding', 'Conflict(String)'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/postgres_knowledge_document_metadata_transaction.rs',
    symbols: ['create_or_get_source_in_transaction'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-service/src/ingest/api_markdown_ingest_pipeline.rs',
    symbols: [
      'MarkdownIndexSourceBinding::Create',
      'mark_failed',
      'replay_if_not_processable',
      'attach_drive_import_linkage',
      'complete_with_chunks_and_outbox',
    ],
    forbidden: ['KnowledgeSourceStore'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-service/src/ingest/service.rs',
    symbols: ['(IngestionJobState::Failed, IngestionJobState::Running)'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-service/src/ingest/markdown_index.rs',
    symbols: ['managed_drive_object_ref_record', 'PrepareMarkdownIndexMetadataRecord', 'drive_space_id: Option<&str>', 'ingest_linkage'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-service/src/imports/mod.rs',
    symbols: ['managed_drive_object_ref_record', 'create_or_prepare_drive_import_metadata'],
  },
  {
    file: 'crates/sdkwork-intelligence-knowledgebase-repository-sqlx/src/postgres_markdown_index_metadata_store.rs',
    symbols: ['create_or_get_source_in_transaction', 'MarkdownIndexSourceBinding'],
  },
];

for (const entry of requiredIngestMetadataAlignment) {
  const content = readText(entry.file);
  for (const symbol of entry.symbols ?? []) {
    assert(
      content.includes(symbol),
      `${entry.file} must declare ingest metadata alignment symbol ${symbol}`,
    );
  }
  for (const forbidden of entry.forbidden ?? []) {
    assert(
      !content.includes(forbidden),
      `${entry.file} must not retain legacy ${forbidden} after atomic markdown metadata binding`,
    );
  }
}

if (failures.length > 0) {
  process.stderr.write(
    `Architecture alignment failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}\n`,
  );
  if (warnings.length > 0) {
    process.stderr.write(
      `Warnings:\n${warnings.map((warning) => `- ${warning}`).join('\n')}\n`,
    );
  }
  process.exit(1);
}

if (warnings.length > 0) {
  process.stdout.write(
    `Architecture alignment passed with warnings:\n${warnings.map((warning) => `- ${warning}`).join('\n')}\n`,
  );
} else {
  process.stdout.write('Architecture alignment passed\n');
}
