import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, '..');
const checkOnly = process.argv.includes('--check');
const pendingChanges = [];

const targets = [
  {
    manifestRs: 'crates/sdkwork-routes-knowledgebase-app-api/src/manifest.rs',
    // Auth reality lives in the HttpRoute builder declarations; entries built
    // via `HttpRoute::public(...)` (directly or through a const helper) are
    // unauthenticated on the wire and MUST NOT be emitted as dual-token, or
    // manifest-consuming gateways would 401 callers that cannot present
    // SDKWork tokens (e.g. WeChat server callbacks).
    authSourceRs: 'crates/sdkwork-routes-knowledgebase-app-api/src/http_route_manifest.rs',
    jsonFile: 'sdks/_route-manifests/app-api/sdkwork-routes-knowledgebase-app-api.route-manifest.json',
    apiSurface: 'app-api',
  },
  {
    manifestRs: 'crates/sdkwork-routes-knowledgebase-backend-api/src/manifest.rs',
    jsonFile: 'sdks/_route-manifests/backend-api/sdkwork-routes-knowledgebase-backend-api.route-manifest.json',
    apiSurface: 'backend-api',
  },
  {
    manifestRs: 'crates/sdkwork-routes-knowledgebase-open-api/src/manifest.rs',
    jsonFile: 'sdks/_route-manifests/open-api/sdkwork-routes-knowledgebase-open-api.route-manifest.json',
    apiSurface: 'open-api',
  },
  {
    manifestRs: 'crates/sdkwork-routes-knowledgebase-internal-api/src/http_route_manifest.rs',
    jsonFile: 'sdks/_route-manifests/internal-api/sdkwork-routes-knowledgebase-internal-api.route-manifest.json',
    apiSurface: 'internal-api',
    parser: 'http-route-builder',
    initialManifest: {
      schemaVersion: 1,
      kind: 'sdkwork.route.manifest',
      packageName: 'sdkwork-routes-knowledgebase-internal-api',
      surface: 'internal-api',
      owner: 'sdkwork-knowledgebase',
      domain: 'knowledgebase',
      capability: 'wiki-public-provider',
      apiAuthority: 'sdkwork-knowledgebase-internal-api',
      sdkFamily: 'sdkwork-knowledgebase-internal-sdk',
      prefix: '/internal/v3/api',
      source: {
        crateRoot: 'crates/sdkwork-routes-knowledgebase-internal-api',
        crateImport: 'sdkwork_routes_knowledgebase_internal_api',
        openApiAuthority:
          'apis/internal-api/knowledgebase/sdkwork-knowledgebase-internal-api.openapi.yaml',
      },
      routes: [],
    },
  },
];

function parseRoutes(manifestSource) {
  const routes = [];
  const entryPattern =
    /method:\s*"([A-Z]+)"[\s\S]*?path:\s*"([^"]+)"[\s\S]*?operation_id:\s*"([^"]+)"/g;
  for (const match of manifestSource.matchAll(entryPattern)) {
    routes.push({
      method: match[1],
      path: match[2],
      operationId: match[3],
    });
  }
  return routes;
}

function parseHttpRouteBuilders(manifestSource) {
  const routes = [];
  const entryPattern =
    /HttpRoute::ingress_token\(\s*HttpMethod::([A-Za-z]+),\s*"([^"]+)",\s*"[^"]+",\s*"([^"]+)",\s*\)/g;
  for (const match of manifestSource.matchAll(entryPattern)) {
    routes.push({
      method: match[1].toUpperCase(),
      path: match[2],
      operationId: match[3],
    });
  }
  return routes;
}

/**
 * Collects the operation ids declared as unauthenticated (`HttpRoute::public`)
 * in the route builder source — either directly or through a const helper that
 * returns `HttpRoute::public(...)`, e.g. the WeChat callback route builder.
 */
function parsePublicRouteOperationIds(authSourceRs) {
  const operationIds = new Set();
  const directPattern =
    /HttpRoute::public\(\s*HttpMethod::[A-Za-z]+,\s*"[^"]+",\s*"[^"]*",\s*"([^"]+)"/g;
  for (const match of authSourceRs.matchAll(directPattern)) {
    operationIds.add(match[1]);
  }
  // Const helpers: `const fn <name>(...) -> HttpRoute { HttpRoute::public(...) }`
  // — capture the helper name, then its call sites' operation id argument.
  const helperNames = new Set();
  const helperPattern =
    /const\s+fn\s+([a-z_0-9]+)\([^)]*\)\s*->\s*HttpRoute\s*\{[^}]*HttpRoute::public\(/g;
  for (const match of authSourceRs.matchAll(helperPattern)) {
    helperNames.add(match[1]);
  }
  for (const helperName of helperNames) {
    const callPattern = new RegExp(`${helperName}\\(\\s*HttpMethod::[A-Za-z]+,\\s*"[^"]+",\\s*"[^"]+"`, 'g');
    for (const match of authSourceRs.matchAll(callPattern)) {
      operationIds.add(match[0].match(/"([^"]+)"\s*$/)?.[1] ?? '');
    }
  }
  operationIds.delete('');
  return operationIds;
}

function buildRoute(route, manifest, apiSurface, publicOperationIds) {
  if (publicOperationIds.has(route.operationId)) {
    return {
      method: route.method,
      path: route.path,
      operationId: route.operationId,
      tags: ['knowledge'],
      auth: {
        mode: 'anonymous',
        required: false,
      },
      handler: {
        module: 'crate::routes',
        name: null,
      },
      ownership: {
        owner: manifest.owner,
        apiAuthority: manifest.apiAuthority,
      },
      requestContext: 'WebRequestContext',
      apiSurface,
    };
  }
  const authMode = apiSurface === 'open-api'
    ? 'bearer'
    : apiSurface === 'internal-api'
      ? 'ingress-token'
      : 'dual-token';
  return {
    method: route.method,
    path: route.path,
    operationId: route.operationId,
    tags: apiSurface === 'internal-api' ? ['knowledgebaseInternalWiki'] : ['knowledge'],
    auth: {
      mode: authMode,
      required: true,
    },
    handler: {
      module: 'crate::routes',
      name: null,
    },
    ownership: {
      owner: manifest.owner,
      apiAuthority: manifest.apiAuthority,
    },
    requestContext: 'WebRequestContext',
    apiSurface,
  };
}

for (const target of targets) {
  const manifestSource = await readFile(path.join(workspaceRoot, target.manifestRs), 'utf8');
  const jsonPath = path.join(workspaceRoot, target.jsonFile);
  let current = '';
  try {
    current = await readFile(jsonPath, 'utf8');
  } catch (error) {
    if (error?.code !== 'ENOENT' || !target.initialManifest) {
      throw error;
    }
  }
  const manifest = current
    ? JSON.parse(current)
    : structuredClone(target.initialManifest);
  const routes = target.parser === 'http-route-builder'
    ? parseHttpRouteBuilders(manifestSource)
    : parseRoutes(manifestSource);
  if (routes.length === 0) {
    throw new Error(`No routes parsed from ${target.manifestRs}`);
  }
  let publicOperationIds = new Set();
  if (target.authSourceRs) {
    const authSourceRs = await readFile(path.join(workspaceRoot, target.authSourceRs), 'utf8');
    publicOperationIds = parsePublicRouteOperationIds(authSourceRs);
  }
  manifest.routes = routes.map((route) =>
    buildRoute(route, manifest, target.apiSurface, publicOperationIds));
  const desired = `${JSON.stringify(manifest, null, 2)}\n`;
  if (current === desired) {
    continue;
  }

  pendingChanges.push(target.jsonFile);
  if (!checkOnly) {
    await mkdir(path.dirname(jsonPath), { recursive: true });
    await writeFile(jsonPath, desired, 'utf8');
    console.log(`Synced ${routes.length} routes to ${target.jsonFile}`);
  }
}

if (checkOnly && pendingChanges.length > 0) {
  console.error(JSON.stringify({ ok: false, mode: 'check', pendingChanges }, null, 2));
  process.exit(1);
}

console.log(JSON.stringify({ ok: true, mode: checkOnly ? 'check' : 'apply' }, null, 2));
