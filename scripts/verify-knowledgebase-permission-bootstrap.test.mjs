#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function readJson(relativePath) {
  return JSON.parse(readFileSync(path.join(repoRoot, relativePath), 'utf8'));
}

function readText(relativePath) {
  return readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function expandWildcard(scope, catalogCodes) {
  const expanded = new Set();
  for (const entry of scope) {
    if (entry === '*') {
      for (const code of catalogCodes) {
        expanded.add(code);
      }
      continue;
    }
    if (entry.endsWith('.*')) {
      const prefix = entry.slice(0, -2);
      for (const code of catalogCodes) {
        if (code.startsWith(`${prefix}.`)) {
          expanded.add(code);
        }
      }
      continue;
    }
    expanded.add(entry);
  }
  return expanded;
}

describe('knowledgebase permission bootstrap alignment', () => {
  it('bootstrap scope covers app route manifest permissions via wildcard inheritance', () => {
    const appManifest = readJson('specs/iam.module.manifest.json');
    const appConfig = readJson('sdkwork.app.config.json');
    const routeManifest = readText(
      'crates/sdkwork-routes-knowledgebase-app-api/src/http_route_manifest.rs',
    );

    const catalogCodes = appManifest.permissions.catalog.map((entry) => entry.code);
    const routePermissions = [
      ...routeManifest.matchAll(/"knowledge\.[^"]+"/g),
    ].map((match) => match[0].slice(1, -1));
    const bootstrapScope = appConfig.backend.accessTokenPermissionScope ?? [];
    const effective = expandWildcard(bootstrapScope, catalogCodes);

    for (const permission of new Set(routePermissions)) {
      assert.ok(
        effective.has(permission),
        `bootstrap scope must cover route permission ${permission}`,
      );
    }
  });

  it('standalone gateway composes IAM without coupling the host-neutral assembly', () => {
    const gatewayMain = readText(
      'crates/sdkwork-api-knowledgebase-standalone-gateway/src/bin/app_main.rs',
    );
    const gatewayCargo = readText(
      'crates/sdkwork-api-knowledgebase-standalone-gateway/Cargo.toml',
    );
    const assemblyBootstrap = readText(
      'crates/sdkwork-api-knowledgebase-assembly/src/bootstrap.rs',
    );
    const assemblyCargo = readText('crates/sdkwork-api-knowledgebase-assembly/Cargo.toml');

    assert.match(
      gatewayMain,
      /sdkwork_api_iam_assembly::assemble_app_api_contribution/,
    );
    assert.match(gatewayMain, /ApiModuleRegistry::new\(\)/);
    assert.match(gatewayMain, /\.try_compose\(/);
    assert.match(gatewayCargo, /^sdkwork_api_iam_assembly\.workspace\s*=\s*true$/m);
    assert.doesNotMatch(assemblyBootstrap, /sdkwork_api_iam_assembly/);
    assert.doesNotMatch(assemblyCargo, /sdkwork[_-]api[_-]iam[_-]assembly/);
    assert.doesNotMatch(assemblyCargo, /sdkwork-routes-iam-app-api/);
  });

  it('pc surface declares permissionComposition inheritance', () => {
    const componentSpec = readJson('apps/sdkwork-knowledgebase-pc/specs/component.spec.json');
    const composition = componentSpec.contracts.permissionComposition;

    assert.equal(composition.inheritanceMode, 'module-catalog-with-overrides');
    assert.match(
      composition.bootstrapAccessTokenScope.inheritFrom,
      /sdkwork\.app\.config\.json#backend\.accessTokenPermissionScope/,
    );
  });

  it('development topology binds runtime tenant to sdkwork.app.config.json tenantId', () => {
    const appConfig = readJson('sdkwork.app.config.json');
    const tenantId = appConfig.backend.tenantId;
    const developmentProfiles = [
      'etc/topology/standalone.development.env',
      'etc/topology/cloud.development.env',
    ];

    for (const profilePath of developmentProfiles) {
      const profile = readText(profilePath);
      assert.match(
        profile,
        new RegExp(`SDKWORK_KNOWLEDGEBASE_TENANT_ID=${tenantId}`),
        `${profilePath} must bind runtime tenant ${tenantId}`,
      );
    }
  });
});
