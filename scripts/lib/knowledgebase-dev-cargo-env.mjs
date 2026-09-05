#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

import { normalizeText, REPO_ROOT } from './knowledgebase-topology.mjs';

export function cargoCommand(platform = process.platform) {
  return platform === 'win32' ? 'cargo.exe' : 'cargo';
}

/**
 * Windows rustc can ICE under incremental compilation when multiple cargo
 * processes share one target directory (gateway + worker during pnpm dev).
 * Force CARGO_INCREMENTAL=0 on win32 so ambient shell/toolchain defaults cannot
 * re-enable the flaky path. Opt back in with
 * SDKWORK_KNOWLEDGEBASE_ALLOW_CARGO_INCREMENTAL=1.
 */
export function resolveKnowledgebaseCargoDevEnv({
  env = {},
  platform = process.platform,
} = {}) {
  const allowIncremental =
    normalizeText(env.SDKWORK_KNOWLEDGEBASE_ALLOW_CARGO_INCREMENTAL) === '1';
  if (platform === 'win32' && !allowIncremental) {
    return {
      ...env,
      CARGO_INCREMENTAL: '0',
    };
  }
  return { ...env };
}

export function collectCargoPackageIds(processEntries = []) {
  const packages = [];
  const seen = new Set();
  for (const entry of processEntries) {
    const args = Array.isArray(entry?.args) ? entry.args : [];
    for (let index = 0; index < args.length; index += 1) {
      if (args[index] !== '-p' && args[index] !== '--package') {
        continue;
      }
      const packageId = normalizeText(args[index + 1]);
      if (!packageId || seen.has(packageId)) {
        continue;
      }
      seen.add(packageId);
      packages.push(packageId);
    }
  }
  return packages;
}

export function buildCargoPrebuildArgs(packageIds = []) {
  if (!Array.isArray(packageIds) || packageIds.length === 0) {
    return null;
  }
  return ['build', ...packageIds.flatMap((packageId) => ['-p', packageId])];
}

export function prebuildKnowledgebaseBackendPackages({
  processEntries = [],
  env = process.env,
  cwd = REPO_ROOT,
  platform = process.platform,
  spawn = spawnSync,
} = {}) {
  const packageIds = collectCargoPackageIds(processEntries);
  const args = buildCargoPrebuildArgs(packageIds);
  if (!args) {
    return { skipped: true, packageIds: [], status: 0 };
  }

  const result = spawn(cargoCommand(platform), args, {
    cwd: path.resolve(cwd),
    env: { ...process.env, ...env },
    stdio: 'inherit',
    windowsHide: true,
  });
  const status = result.status ?? 1;
  if (status !== 0) {
    throw new Error(
      `cargo prebuild failed for ${packageIds.join(', ')} with code ${status}`,
    );
  }
  return { skipped: false, packageIds, status };
}
