import assert from 'node:assert/strict';
import { test } from 'node:test';

import { resolveLifecycleEnvironment } from '../src/bootstrap/environment';

function envWith(overrides: Record<string, unknown>) {
  return { ...overrides } as unknown as Record<string, unknown>;
}

test('lifecycle environment accepts the sanctioned profile build modes', () => {
  // The official build pipeline invokes vite with profile-shaped modes
  // (`standalone.<env>` / `cloud.<env>`); these must not abort the boot.
  for (const mode of [
    'standalone.production',
    'standalone.development',
    'cloud.staging',
    'cloud.test',
    'STANDALONE.PRODUCTION',
  ]) {
    const lifecycle = resolveLifecycleEnvironment(envWith({ MODE: mode, PROD: true }));
    assert.match(lifecycle, /^(development|test|staging|production)$/u);
  }
});

test('profile demo mode maps to the staging lifecycle posture', () => {
  assert.equal(
    resolveLifecycleEnvironment(envWith({ MODE: 'standalone.demo' })),
    'staging',
  );
});

test('lifecycle environment rejects unknown modes fail-closed', () => {
  assert.throws(
    () => resolveLifecycleEnvironment(envWith({ MODE: 'nonsense' })),
    /must be development, test, staging, production/u,
  );
});

test('explicit lifecycle env overrides the build mode', () => {
  assert.equal(
    resolveLifecycleEnvironment(
      envWith({
        MODE: 'standalone.development',
        VITE_SDKWORK_KNOWLEDGEBASE_H5_ENVIRONMENT: 'production',
      }),
    ),
    'production',
  );
});
