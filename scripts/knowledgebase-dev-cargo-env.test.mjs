#!/usr/bin/env node

import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildCargoPrebuildArgs,
  collectCargoPackageIds,
  prebuildKnowledgebaseBackendPackages,
  resolveKnowledgebaseCargoDevEnv,
} from './lib/knowledgebase-dev-cargo-env.mjs';

test('Windows knowledgebase cargo env disables incremental by default', () => {
  const resolved = resolveKnowledgebaseCargoDevEnv({
    env: {},
    platform: 'win32',
  });
  assert.equal(
    resolved.CARGO_INCREMENTAL,
    '0',
    'Windows pnpm dev must disable incremental compilation to avoid rustc ICE on shared target locks',
  );
});

test('Windows cargo env overrides ambient CARGO_INCREMENTAL=1', () => {
  const resolved = resolveKnowledgebaseCargoDevEnv({
    env: { CARGO_INCREMENTAL: '1' },
    platform: 'win32',
  });
  assert.equal(
    resolved.CARGO_INCREMENTAL,
    '0',
    'ambient CARGO_INCREMENTAL must not re-enable Windows incremental for dual cargo run',
  );
});

test('SDKWORK_KNOWLEDGEBASE_ALLOW_CARGO_INCREMENTAL opts back into incremental', () => {
  const resolved = resolveKnowledgebaseCargoDevEnv({
    env: {
      CARGO_INCREMENTAL: '1',
      SDKWORK_KNOWLEDGEBASE_ALLOW_CARGO_INCREMENTAL: '1',
    },
    platform: 'win32',
  });
  assert.equal(resolved.CARGO_INCREMENTAL, '1');
});

test('non-Windows cargo env leaves incremental unset by default', () => {
  const resolved = resolveKnowledgebaseCargoDevEnv({
    env: { SDKWORK_DATABASE_URL: 'postgresql://localhost/db' },
    platform: 'linux',
  });
  assert.equal(resolved.CARGO_INCREMENTAL, undefined);
  assert.equal(resolved.SDKWORK_DATABASE_URL, 'postgresql://localhost/db');
});

test('collectCargoPackageIds dedupes orchestration -p crates', () => {
  const packages = collectCargoPackageIds([
    {
      args: ['run', '-p', 'sdkwork-api-knowledgebase-standalone-gateway', '--bin', 'sdkwork-api-knowledgebase-standalone-gateway'],
    },
    {
      args: ['run', '-p', 'sdkwork-knowledgebase-worker', '--bin', 'sdkwork-knowledgebase-worker'],
    },
    {
      args: ['run', '-p', 'sdkwork-api-knowledgebase-standalone-gateway', '--bin', 'sdkwork-api-knowledgebase-standalone-gateway'],
    },
  ]);
  assert.deepEqual(packages, [
    'sdkwork-api-knowledgebase-standalone-gateway',
    'sdkwork-knowledgebase-worker',
  ]);
});

test('buildCargoPrebuildArgs builds a single cargo build invocation', () => {
  assert.equal(buildCargoPrebuildArgs([]), null);
  assert.deepEqual(
    buildCargoPrebuildArgs([
      'sdkwork-api-knowledgebase-standalone-gateway',
      'sdkwork-knowledgebase-worker',
    ]),
    [
      'build',
      '-p',
      'sdkwork-api-knowledgebase-standalone-gateway',
      '-p',
      'sdkwork-knowledgebase-worker',
    ],
  );
});

test('prebuildKnowledgebaseBackendPackages invokes one cargo build for all packages', () => {
  const calls = [];
  const result = prebuildKnowledgebaseBackendPackages({
    processEntries: [
      {
        args: ['run', '-p', 'sdkwork-api-knowledgebase-standalone-gateway', '--bin', 'x'],
      },
      {
        args: ['run', '-p', 'sdkwork-knowledgebase-worker', '--bin', 'y'],
      },
    ],
    env: { CARGO_INCREMENTAL: '0' },
    platform: 'win32',
    spawn: (command, args, options) => {
      calls.push({ command, args, options });
      return { status: 0 };
    },
  });

  assert.equal(result.skipped, false);
  assert.deepEqual(result.packageIds, [
    'sdkwork-api-knowledgebase-standalone-gateway',
    'sdkwork-knowledgebase-worker',
  ]);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].command, 'cargo.exe');
  assert.deepEqual(calls[0].args, [
    'build',
    '-p',
    'sdkwork-api-knowledgebase-standalone-gateway',
    '-p',
    'sdkwork-knowledgebase-worker',
  ]);
  assert.equal(calls[0].options.env.CARGO_INCREMENTAL, '0');
});
