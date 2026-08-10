import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const readJson = (relativePath) => JSON.parse(readFileSync(path.join(repoRoot, relativePath), 'utf8'));
const readText = (relativePath) => readFileSync(path.join(repoRoot, relativePath), 'utf8');

const topology = readJson('specs/topology.spec.json');
const deployManifest = readText('deployments/deploy.yaml');

// APP_RUNTIME_TOPOLOGY_NAMING.md section 9.2: knowledgebase role on
// sdkwork.com; auxiliary surfaces knowledgebase-admin / knowledge follow the
// same suffix formula.
const expectedHosts = {
  public: {
    production: 'knowledgebase.sdkwork.com',
    development: 'knowledgebase-dev.sdkwork.com',
    test: 'knowledgebase-test.sdkwork.com',
    staging: 'knowledgebase-staging.sdkwork.com',
  },
  admin: {
    production: 'knowledgebase-admin.sdkwork.com',
    development: 'knowledgebase-admin-dev.sdkwork.com',
    test: 'knowledgebase-admin-test.sdkwork.com',
    staging: 'knowledgebase-admin-staging.sdkwork.com',
  },
  open: {
    production: 'knowledge.sdkwork.com',
    development: 'knowledge-dev.sdkwork.com',
    test: 'knowledge-test.sdkwork.com',
    staging: 'knowledge-staging.sdkwork.com',
  },
  gateway: {
    production: 'api.sdkwork.com',
    development: 'api-dev.sdkwork.com',
    test: 'api-test.sdkwork.com',
    staging: 'api-staging.sdkwork.com',
  },
};

// KB PUBLIC_HTTP_URL semantics: app API base URL including /app/v3/api path;
// BACKEND includes /backend/v3/api; OPEN includes /knowledge/v3/api.
const urlSuffixes = {
  public: '/app/v3/api',
  admin: '/backend/v3/api',
  open: '/knowledge/v3/api',
};

// cloudPublicHosts must register all four surfaces with per-environment hosts.
for (const [surfaceId, role] of [
  ['application.public-ingress', 'public'],
  ['application.backend-http', 'admin'],
  ['application.open-http', 'open'],
  ['platform.api-gateway', 'gateway'],
]) {
  const entry = topology.cloudPublicHosts?.[surfaceId];
  assert.ok(entry, `topology must register ${surfaceId}`);
  assert.equal(entry.httpHost, expectedHosts[role].production, `${surfaceId} production host`);
  for (const environment of ['development', 'test', 'staging']) {
    assert.equal(
      entry.environments?.[environment]?.httpHost,
      expectedHosts[role][environment],
      `${surfaceId} ${environment} host`,
    );
  }
}

const topologyEnvFiles = [
  'cloud.development.env', 'cloud.test.env', 'cloud.staging.env', 'cloud.production.env',
  'standalone.development.env', 'standalone.test.env', 'standalone.staging.env', 'standalone.production.env',
];

for (const environment of ['development', 'test', 'staging', 'production']) {
  const profileSource = readText(`etc/topology/cloud.${environment}.env`);
  assert.match(profileSource, new RegExp(`SDKWORK_KNOWLEDGEBASE_ENVIRONMENT=${environment}`, 'u'));
  const development = environment === 'development';
  for (const [key, role] of [
    ['APPLICATION_PUBLIC_HTTP_URL', 'public'],
    ['APPLICATION_BACKEND_HTTP_URL', 'admin'],
    ['APPLICATION_OPEN_HTTP_URL', 'open'],
    ['PLATFORM_API_GATEWAY_HTTP_URL', 'gateway'],
  ]) {
    // cloud.development folds: PUBLIC has no path suffix (DEV_SAME_ORIGIN_API)
    // and BACKEND/OPEN resolve through the platform gateway origin (api-dev).
    let expected;
    if (key === 'PLATFORM_API_GATEWAY_HTTP_URL') {
      expected = `https://${expectedHosts[role][environment]}`;
    } else if (development) {
      expected = key === 'APPLICATION_PUBLIC_HTTP_URL'
        ? `https://${expectedHosts[role][environment]}`
        : `https://${expectedHosts.gateway[environment]}${urlSuffixes[role]}`;
    } else {
      expected = `https://${expectedHosts[role][environment]}${urlSuffixes[role]}`;
    }
    assert.ok(
      profileSource.includes(`SDKWORK_KNOWLEDGEBASE_${key}=${expected}`),
      `cloud ${environment} ${key} must be ${expected}`,
    );
    assert.ok(
      profileSource.includes(`VITE_SDKWORK_KNOWLEDGEBASE_${key}=${expected}`),
      `cloud ${environment} VITE ${key} must be ${expected}`,
    );
  }
}

// Standalone profiles fold to loopback URLs and must not reference cloud
// hostnames (standalone.development may keep its local dev origins).
for (const environment of ['test', 'staging', 'production']) {
  const profileSource = readText(`etc/topology/standalone.${environment}.env`);
  assert.doesNotMatch(profileSource, /\.sdkwork\.com/u, `standalone ${environment} must not reference cloud hostnames`);
  assert.match(profileSource, /127\.0\.0\.1/u, `standalone ${environment} must fold to loopback URLs`);
}

// Retired vocabulary must not appear in source config.
const workspaceConfigText = [
  ...topologyEnvFiles.map((name) => readText(`etc/topology/${name}`)),
  readText('etc/sdkwork.deployment.config.json'),
  readText('specs/topology.spec.json'),
  deployManifest,
].join('\n');
assert.doesNotMatch(workspaceConfigText, /testapikb\.sdkwork\.com/u, 'testapikb.sdkwork.com is a docker-only test domain and must not appear in source config');

// deploy.yaml cloud expose domain + aliases must all belong to the registered
// host set (standalone.production uses an internal customer domain, exempt).
const cloudSection = deployManifest.split('standalone.production:')[0] ?? deployManifest;
const hostSets = new Set([
  ...Object.values(expectedHosts.public),
  ...Object.values(expectedHosts.admin),
  ...Object.values(expectedHosts.open),
]);
const exposeBlocks = [...cloudSection.matchAll(/domain:\s*([^\s]+)[\s\S]*?(?=\n\s{4}- domain:|\n\s{2}cloud\.|\n\s{2}standalone\.|$)/gu)];
assert.ok(exposeBlocks.length >= 3, 'deploy.yaml must declare cloud test/staging/production exposes');
for (const block of exposeBlocks) {
  const domain = block[1];
  assert.ok(hostSets.has(domain), `expose domain ${domain} must be registered in cloudPublicHosts`);
  for (const aliasMatch of block[0].matchAll(/aliases:\s*\n((?:\s+- [^\n]+\n?)+)/gu)) {
    for (const aliasLine of aliasMatch[1].matchAll(/^\s+- ([^\n]+)$/gmu)) {
      const alias = aliasLine[1].trim();
      assert.ok(hostSets.has(alias), `expose alias ${alias} must be registered in cloudPublicHosts`);
    }
  }
}

console.log('sdkwork-knowledgebase web domain routing standard passed');
