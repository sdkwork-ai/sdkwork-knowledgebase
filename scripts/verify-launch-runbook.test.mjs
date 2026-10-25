import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it } from 'node:test';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');

function readRepoFile(relativePath) {
  return readFileSync(path.join(repoRoot, relativePath), 'utf8');
}

function readJson(relativePath) {
  return JSON.parse(readRepoFile(relativePath));
}

describe('knowledgebase launch runbook alignment', () => {
  it('documents production launch sequencing and verification gates', () => {
    const runbook = readRepoFile('deployments/runbooks/production-launch.md');
    for (const section of [
      'Pre-flight',
      'Database bootstrap',
      'Deployment smoke',
      'Observability',
      'Release artifacts',
      'Rollback',
    ]) {
      assert.match(runbook, new RegExp(section, 'i'), `missing runbook section: ${section}`);
    }
    assert.match(runbook, /backup-restore\.md/);
    assert.match(runbook, /pnpm db:bootstrap/);
    assert.match(runbook, /pnpm test:smoke/);
    assert.match(runbook, /SDKWORK_KNOWLEDGEBASE_LOG_FORMAT=json/);
  });

  it('keeps backup-restore runbook verification checklist intact', () => {
    const backupRunbook = readRepoFile('deployments/runbooks/backup-restore.md');
    assert.match(backupRunbook, /pg_dump/);
    assert.match(backupRunbook, /\/readyz/);
  });

  it('defaults structured JSON logging in production topology', () => {
    const topology = readRepoFile('etc/topology/cloud.production.env');
    assert.match(topology, /^SDKWORK_KNOWLEDGEBASE_LOG_FORMAT=json/m);
  });

  it('documents OTEL enablement without exposing metrics on public ingress', () => {
    const topology = readRepoFile('etc/topology/cloud.production.env');
    const ingress = readRepoFile('deployments/kubernetes/ingress.yaml');
    assert.match(topology, /OTEL_EXPORTER_OTLP_ENDPOINT/);
    assert.doesNotMatch(ingress, /\/metrics/);
  });

  it('keeps public health smoke separate from internal metrics smoke', () => {
    const smokeScript = readRepoFile('scripts/smoke-knowledgebase-api.test.mjs');
    const runbook = readRepoFile('deployments/runbooks/production-launch.md');
    const deploymentReadme = readRepoFile('deployments/README.md');

    assert.match(smokeScript, /const DEFAULT_PROBE_PATHS = \['\/livez', '\/readyz'\]/);
    assert.match(smokeScript, /SDKWORK_KNOWLEDGEBASE_SMOKE_METRICS_URLS?/);
    assert.match(runbook, /public smoke checks only probe `\/livez` and `\/readyz`/i);
    assert.match(runbook, /SDKWORK_KNOWLEDGEBASE_SMOKE_METRICS_URLS?/);
    assert.match(deploymentReadme, /internal metrics smoke/i);
  });

  it('indexes Playwright author and search launch flows', () => {
    const playwrightConfig = readRepoFile('apps/sdkwork-knowledgebase-pc/playwright.config.ts');
    assert.match(playwrightConfig, /\.flow\\.spec\\.ts/);
    assert.match(playwrightConfig, /shell\\.smoke\\.spec\\.ts/);
  });

  it('requires supply-chain release gates in app manifest', () => {
    const manifest = readJson('agent.app.config.json');
    assert.equal(manifest.security.checksumRequired, true);
    assert.equal(manifest.security.signatureRequired, true);
    assert.equal(manifest.security.sbomRequired, true);
    const webPackage = manifest.artifacts.installConfig.packages.find(
      (entry) => entry.id === 'web-universal-cloud-browser-zip',
    );
    assert.ok(webPackage, 'web-universal-cloud-browser-zip package must exist');
    assert.equal(
      webPackage.enabled,
      false,
      'web package must stay disabled until checksum/signature/SBOM/provenance evidence exists',
    );
    assert.match(webPackage.metadata?.releaseStatus ?? '', /prelaunch-artifact-pending/);
  });

  it('documents canonical prelaunch package release gates without legacy package ids', () => {
    const runbook = readRepoFile('deployments/runbooks/production-launch.md');
    assert.match(runbook, /web-universal-cloud-browser-zip/);
    assert.doesNotMatch(runbook, /web-production/);
    for (const evidence of [
      'checksum',
      'signature',
      'SBOM',
      'provenance',
      'attestation',
      'workflow run',
      'rollout',
      'rollback',
      'live smoke',
    ]) {
      assert.match(runbook, new RegExp(evidence, 'i'), `missing release gate evidence: ${evidence}`);
    }
    assert.match(runbook, /prelaunch-artifact-pending/);
  });

  it('keeps the app manifest prelaunch-gated until release evidence exists', () => {
    const manifest = readJson('agent.app.config.json');
    assert.equal(manifest.publish.status, 'DRAFT');
    assert.equal(manifest.publish.preLaunch, true);
    assert.equal(manifest.publish.metadata?.releaseStatus, 'prelaunch-gated');
    assert.equal(manifest.release.defaultChannel, 'DEV');
    assert.deepEqual(Object.keys(manifest.release.latest).sort(), ['DEV']);
    assert.ok(
      manifest.release.notes.every((note) => note.releaseChannel !== 'STABLE'),
      'prelaunch manifest must not expose STABLE release notes',
    );
    assert.ok(
      manifest.release.notes.every((note) => !Object.hasOwn(note, 'publishedAt')),
      'prelaunch release notes must not carry publishedAt',
    );
  });

  it('does not enable placeholder catalog media for release projection', () => {
    const manifest = readJson('agent.app.config.json');
    const mediaEntries = [
      manifest.media.icons.primary,
      ...(manifest.media.icons.platform ?? []),
      ...(manifest.media.screenshots ?? []),
      ...(manifest.media.previews ?? []),
    ].filter(Boolean);
    assert.ok(mediaEntries.length >= 3);
    for (const entry of mediaEntries) {
      if (entry.metadata?.generatedPlaceholder) {
        assert.equal(entry.enabled, false, `${entry.id} placeholder media must be disabled`);
        assert.match(entry.metadata.releaseStatus ?? '', /prelaunch-placeholder/);
      }
    }
  });

  it('uses canonical package ids for prelaunch install packages', () => {
    const manifest = readJson('agent.app.config.json');
    const packageIds = manifest.artifacts.installConfig.packages.map((entry) => entry.id).sort();
    assert.deepEqual(packageIds, [
      'linux-x64-standalone-desktop-appimage',
      'macos-universal-standalone-desktop-dmg',
      'web-universal-cloud-browser-zip',
      'windows-x64-standalone-desktop-zip',
    ]);
    assert.equal(manifest.publish.defaultPackageId, 'web-universal-cloud-browser-zip');
    assert.equal(
      manifest.artifacts.installConfig.defaultPackageId,
      'web-universal-cloud-browser-zip',
    );
  });

  it('keeps desktop bundles prelaunch-disabled until CI packaging ships', () => {
    const manifest = readJson('agent.app.config.json');
    const desktopPackages = manifest.artifacts.installConfig.packages.filter((entry) =>
      String(entry.platform ?? '').startsWith('DESKTOP_'),
    );
    assert.ok(desktopPackages.length >= 3);
    for (const pkg of desktopPackages) {
      assert.equal(pkg.enabled, false, `${pkg.id} must stay disabled prelaunch`);
      assert.match(pkg.metadata?.reason ?? '', /desktop CI/i);
    }
  });

  it('indexes all four SDK families for release consumption', () => {
    const componentSpec = readJson('specs/component.spec.json');
    const families = componentSpec.contracts.sdkClients.map((entry) => entry.family).sort();
    assert.deepEqual(families, [
      'sdkwork-knowledgebase-app-sdk',
      'sdkwork-knowledgebase-backend-sdk',
      'sdkwork-knowledgebase-internal-sdk',
      'sdkwork-knowledgebase-sdk',
    ]);
  });

  it('blocks WeChat demo fallback in production builds', () => {
    const wechatService = readRepoFile(
      'apps/sdkwork-knowledgebase-pc/packages/sdkwork-knowledgebase-pc-knowledgebase/src/services/wechat.ts',
    );
    assert.match(wechatService, /function requireWechatSdk/);
    assert.match(wechatService, /isKnowledgebaseApiAvailable/);
    assert.doesNotMatch(wechatService, /shouldUseKnowledgebaseDemoFallback|assertWechatDemoFallbackAllowed/);
  });

  it('does not ship debug-only Playwright flows in launch CI', () => {
    const e2eDir = path.join(repoRoot, 'apps/sdkwork-knowledgebase-pc/e2e');
    const debugFlow = path.join(e2eDir, 'debug.flow.spec.ts');
    assert.throws(() => readFileSync(debugFlow, 'utf8'), /ENOENT/);
  });
});
