import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

const SDKWORK_ACCESS_TOKEN_ENV_KEY = 'SDKWORK_ACCESS_TOKEN';
const SDKWORK_TOKEN_VERSION_CURRENT = 1;
const DEFAULT_IAM_TENANT_ID = '100001';
const DEFAULT_IAM_ORGANIZATION_ID = '0';

function normalizeText(value) {
  const normalized = String(value ?? '').trim();
  return normalized || undefined;
}

// Mirrors @sdkwork/runtime-bootstrap createTestJwt for local Node dev env scripts.
// Inlined so knowledgebase-dev.mjs does not depend on sdkwork-iam node_modules resolution.
function createTestJwt(claims) {
  const payload = {
    token_version: SDKWORK_TOKEN_VERSION_CURRENT,
    ...claims,
  };
  const header = btoa(JSON.stringify({ alg: 'none', typ: 'JWT' })).replace(/=+$/g, '');
  const encodedPayload = btoa(JSON.stringify(payload)).replace(/=+$/g, '');
  return `${header}.${encodedPayload}.signature`;
}

function resolveAppIdFromManifest(manifest) {
  const appKey = manifest?.app?.key?.trim();
  if (appKey) {
    return appKey;
  }
  const legacyAppId = manifest?.app?.id?.trim();
  if (legacyAppId) {
    return legacyAppId;
  }
  throw new Error('sdkwork.app.config.json app.key is required for IAM bootstrap access token generation');
}

function resolveTenantIdFromManifest(manifest) {
  return normalizeText(manifest?.backend?.tenantId) ?? DEFAULT_IAM_TENANT_ID;
}

function resolveOrganizationIdFromManifest(manifest) {
  return normalizeText(manifest?.backend?.organizationId) ?? DEFAULT_IAM_ORGANIZATION_ID;
}

function resolveAccessTokenPermissionScope(manifest) {
  const scope = manifest?.backend?.accessTokenPermissionScope
    ?? manifest?.backend?.permissionScope
    ?? [];
  return Array.isArray(scope) ? scope.map((entry) => String(entry).trim()).filter(Boolean) : [];
}

export function readApplicationManifest(manifestPath) {
  return JSON.parse(readFileSync(manifestPath, 'utf8'));
}

export function createDevBootstrapAccessTokenJwt(options = {}) {
  const manifest = options.manifest ?? {};
  const tenantId = normalizeText(options.tenantId) ?? resolveTenantIdFromManifest(manifest);
  const organizationId = normalizeText(options.organizationId) ?? resolveOrganizationIdFromManifest(manifest);
  const appId = normalizeText(options.appId) ?? resolveAppIdFromManifest(manifest);
  const nowUnixSeconds = Math.floor(Date.now() / 1000);
  const permissionScope = resolveAccessTokenPermissionScope(manifest);

  const claims = {
    app_id: appId,
    deployment_mode: options.deploymentMode ?? 'saas',
    environment: options.environment ?? 'dev',
    exp: nowUnixSeconds + 86_400,
    login_scope: organizationId === '0' ? 'TENANT' : 'ORGANIZATION',
    organization_id: organizationId,
    runtime_target: options.runtimeTarget ?? 'browser',
    session_id: options.sessionId ?? 'bootstrap-local-dev',
    tenant_id: tenantId,
    token_kind: 'access',
    user_id: options.userId ?? '0',
  };

  if (permissionScope.length > 0) {
    // token-claims-gate: credential-entry-exception
    //
    // This is the ONE bounded exception the contract allows, and it is defined
    // by APP_PERMISSION_COMPOSITION_SPEC: the bootstrap access token scope only
    // gates credential-entry and pre-login SDK transport. It is NOT user session
    // RBAC — that is server-resolved (IAM_SPEC §5.6) and MUST NOT be signed in.
    // Keep it bounded to the manifest's small `accessTokenPermissionScope` list;
    // never widen it with tenant or user grants, which is what produced HTTP 431.
    claims.permission_scope = permissionScope.join(',');
  }

  return createTestJwt(claims);
}

export function buildBootstrapAccessTokenEnvRecord(existingAccessToken, options = {}) {
  const normalized = normalizeText(existingAccessToken);
  return {
    [SDKWORK_ACCESS_TOKEN_ENV_KEY]: normalized || createDevBootstrapAccessTokenJwt(options),
  };
}

export function mergeBootstrapAccessTokenEnv(env = {}, options = {}) {
  return {
    ...env,
    ...buildBootstrapAccessTokenEnvRecord(env[SDKWORK_ACCESS_TOKEN_ENV_KEY], options),
  };
}

export function mergeBootstrapAccessTokenEnvFromManifest({
  env = {},
  manifestPath,
  ...options
} = {}) {
  const manifest = readApplicationManifest(manifestPath);
  return mergeBootstrapAccessTokenEnv(env, { ...options, manifest });
}

export function resolveRepoApplicationManifestPath(repoRoot, manifestPath) {
  const normalizedRepoRoot = normalizeText(repoRoot);
  if (!normalizedRepoRoot) {
    throw new Error('resolveRepoApplicationManifestPath requires repoRoot');
  }

  const explicitManifestPath = normalizeText(manifestPath);
  if (explicitManifestPath) {
    return path.isAbsolute(explicitManifestPath)
      ? explicitManifestPath
      : path.join(normalizedRepoRoot, explicitManifestPath);
  }

  const defaultManifestPath = path.join(normalizedRepoRoot, 'sdkwork.app.config.json');
  if (existsSync(defaultManifestPath)) {
    return defaultManifestPath;
  }

  throw new Error(
    `sdkwork.app.config.json not found under ${normalizedRepoRoot}; pass manifestPath explicitly`,
  );
}

export function mergeRepoDevBootstrapAccessTokenEnv({
  repoRoot,
  env = {},
  manifestPath,
  ...options
} = {}) {
  const resolvedManifestPath = resolveRepoApplicationManifestPath(repoRoot, manifestPath);
  return mergeBootstrapAccessTokenEnvFromManifest({
    env,
    manifestPath: resolvedManifestPath,
    ...options,
  });
}

export { SDKWORK_ACCESS_TOKEN_ENV_KEY };
