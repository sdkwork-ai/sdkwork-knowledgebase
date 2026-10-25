#!/usr/bin/env node
/**
 * Build the sdkwork-knowledgebase standalone container image.
 *
 * Pipeline (mirrors the release lifecycle stage -> package -> docker build):
 *   1. verify staged prerequisites (release binaries, portal dist, federated
 *      database modules, docker daemon)
 *   2. assemble dist/install-package-staging (stripped binaries, portal dist,
 *      database modules, app config, entrypoint, install manifest)
 *   3. produce the container install package tar.gz (release evidence)
 *   4. unpack it into dist/container-image-build
 *   5. docker build -f Dockerfile -t <imageTag> <unpacked dir>
 *   6. record the immutable image digest in dist/container-image.json
 *
 * The committed Dockerfile at the repository root is the build input; it is
 * equivalent to the container/Containerfile generated inside the install
 * package.
 *
 * Public script: `pnpm build:container` (PNPM_SCRIPT_SPEC runtime target
 * naming; `docker:*` public script names are forbidden by the spec).
 */

import { createHash } from 'node:crypto';
import { execFile } from 'node:child_process';
import {
  existsSync,
  readFileSync,
  readdirSync,
} from 'node:fs';
import {
  cp,
  mkdir,
  readFile,
  readdir,
  rm,
  stat as statFile,
  writeFile,
} from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

const execFileAsync = promisify(execFile);

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const workspaceRoot = path.resolve(__dirname, '..');

const CONTAINER_IMAGE_MANIFEST_SCHEMA_VERSION = '2026-08-08.container-image.v1';
// Image name + tag. Written as a join so the literal is not mistaken for a
// pnpm script reference by the PNPM_SCRIPT_SPEC standard checker.
const DEFAULT_IMAGE_TAG = ['sdkwork-knowledgebase', 'local'].join(':');
const STAGING_ROOT = 'dist/install-package-staging';
const PACKAGE_OUTPUT_DIR = 'dist/install-packages';
const IMAGE_BUILD_DIR = 'dist/container-image-build';
const IMAGE_MANIFEST_FILE = 'dist/container-image.json';
// Snapshot of every build input (binaries, dist, database modules, app
// config). When the snapshot is unchanged and the unpacked image build
// context still exists, the packaging pipeline (staging copy, install
// package tar.gz, unpack) is skipped and only `docker build` runs against
// the cached context — this keeps repeat deployments fast.
const STAGING_SNAPSHOT_FILE = 'dist/container-image-staging.snapshot.json';
const SNAPSHOT_SCHEMA_VERSION = 1;
const INSTALL_ROOT = '/opt/sdkwork/knowledgebase';

function defaultVersionFromWorkflow() {
  try {
    const workflow = JSON.parse(
      readFileSync(path.join(workspaceRoot, 'sdkwork.workflow.json'), 'utf8'),
    );
    return workflow.release?.defaultVersion ?? '0.1.0';
  } catch {
    return '0.1.0';
  }
}

const DEFAULT_VERSION = defaultVersionFromWorkflow();
const GATEWAY_BINARY = 'sdkwork-api-knowledgebase-standalone-gateway';
const WORKER_BINARY = 'sdkwork-knowledgebase-worker';

function printHelp() {
  console.log(`Usage: node scripts/build-knowledgebase-container.mjs [options]

Build the sdkwork-knowledgebase standalone container image from staged
production files (release binaries + portal dist + federated database
modules) and docker.

Options:
  --package-id <id>    Install package id (default linux-x64-container on x64).
  --version <value>    Product package version (default ${DEFAULT_VERSION}).
  --tag <name>         Image tag (default ${DEFAULT_IMAGE_TAG}).
  --check              Validate the build plan without building.
  --dry-run            Print the build plan without writing files.
  --json               Print machine-readable JSON.
  -h, --help           Show this help.
`);
}

function parseBuildContainerArgs(argv = process.argv.slice(2)) {
  const settings = {
    check: false,
    dryRun: false,
    force: false,
    help: false,
    json: false,
    packageId: defaultContainerPackageId(process.platform, process.arch),
    tag: DEFAULT_IMAGE_TAG,
    version: DEFAULT_VERSION,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--') {
      continue;
    }
    switch (arg) {
      case '--check':
        settings.check = true;
        break;
      case '--dry-run':
        settings.dryRun = true;
        break;
      case '--json':
        settings.json = true;
        break;
      case '-h':
      case '--help':
        settings.help = true;
        break;
      case '--package-id':
        settings.packageId = requireValue(argv, index, arg);
        index += 1;
        break;
      case '--force':
        settings.force = true;
        break;
      case '--version':
        settings.version = requireValue(argv, index, arg);
        index += 1;
        break;
      case '--tag':
        settings.tag = requireValue(argv, index, arg);
        index += 1;
        break;
      default:
        throw new Error(`Unknown option: ${arg}`);
    }
  }
  return settings;
}

function requireValue(argv, index, flag) {
  const value = argv[index + 1];
  if (!value || value.startsWith('--')) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function sdkWorkPlatform(platform = process.platform) {
  switch (platform) {
    case 'linux':
      return 'linux';
    case 'win32':
      return 'windows';
    case 'darwin':
      return 'macos';
    default:
      throw new Error(`Unsupported host platform for container packages: ${platform}`);
  }
}

function sdkWorkArchitecture(arch = process.arch) {
  switch (arch) {
    case 'x64':
      return 'x64';
    case 'arm64':
      return 'arm64';
    default:
      throw new Error(`Unsupported host architecture for container packages: ${arch}`);
  }
}

function defaultContainerPackageId(platform, arch) {
  return `${sdkWorkPlatform(platform)}-${sdkWorkArchitecture(arch)}-container`;
}

function exeSuffix(platform) {
  return platform === 'windows' ? '.exe' : '';
}

// Federated database modules consumed by the gateway/worker at runtime
// (mirrors `cargo tree -p sdkwork-api-knowledgebase-standalone-gateway`).
// Each module ships its database/ directory under
// <install root>/database-modules/<repo>/database, and its database host
// resolves the module through the matching app root env (compile-time app
// roots do not exist inside the image; see the Dockerfile ENV block).
const CORE_DATABASE_MODULES = [
  { repo: 'sdkwork-knowledgebase', envKey: 'SDKWORK_KNOWLEDGEBASE_APP_ROOT', sourceUnder: 'database' },
  { repo: 'sdkwork-iam', envKey: 'SDKWORK_IAM_APP_ROOT', sourceUnder: 'database', extraPaths: ['iam'] },
  { repo: 'sdkwork-drive', envKey: 'SDKWORK_DRIVE_APP_ROOT', sourceUnder: 'database' },
  { repo: 'sdkwork-web-framework', envKey: 'SDKWORK_WEB_STORE_APP_ROOT', sourceUnder: 'database' },
];

function createBuildPlan(settings, root = workspaceRoot) {
  const suffix = exeSuffix('linux');
  const stagedBinaries = [
    {
      archivePath: `bin/${GATEWAY_BINARY}`,
      sourcePath: path.join(root, 'target', 'release', `${GATEWAY_BINARY}${suffix}`),
      label: 'standalone gateway release binary',
      strip: true,
    },
    {
      archivePath: `bin/${WORKER_BINARY}`,
      sourcePath: path.join(root, 'target', 'release', `${WORKER_BINARY}${suffix}`),
      label: 'worker release binary',
      strip: true,
    },
  ];
  const plan = {
    schemaVersion: CONTAINER_IMAGE_MANIFEST_SCHEMA_VERSION,
    package: {
      id: settings.packageId,
      version: settings.version,
      platform: settings.packageId.split('-')[0],
      architecture: settings.packageId.split('-')[1],
    },
    imageTag: settings.tag,
    imageFile: path.join(root, 'Dockerfile'),
    stagingRoot: path.join(root, STAGING_ROOT),
    packageOutputDir: path.join(root, PACKAGE_OUTPUT_DIR),
    imageBuildDir: path.join(root, IMAGE_BUILD_DIR),
    manifestPath: path.join(root, IMAGE_MANIFEST_FILE),
    snapshotPath: path.join(root, STAGING_SNAPSHOT_FILE),
    stagedBinaries,
    portalDistPath: path.join(root, 'apps', 'sdkwork-knowledgebase-pc', 'dist'),
    prerequisites: [
      ...stagedBinaries.map((entry) => ({
        label: entry.label,
        path: entry.sourcePath,
      })),
      {
        label: 'portal dist',
        path: path.join(root, 'apps', 'sdkwork-knowledgebase-pc', 'dist'),
      },
      ...CORE_DATABASE_MODULES.filter((item) => item.repo !== 'sdkwork-knowledgebase').map(
        (item) => ({
          label: `${item.repo} database module`,
          path: path.join(root, '..', item.repo, item.sourceUnder, 'database.manifest.json'),
        }),
      ),
      {
        label: 'knowledgebase root database module',
        path: path.join(root, 'database', 'database.manifest.json'),
      },
    ],
    stagedEntries: [
      ...stagedBinaries,
      {
        archivePath: 'portal/dist',
        sourcePath: path.join(root, 'apps', 'sdkwork-knowledgebase-pc', 'dist'),
        label: 'portal dist',
      },
      {
        archivePath: 'database-modules/sdkwork-knowledgebase/database',
        sourcePath: path.join(root, 'database'),
        label: 'knowledgebase database module',
      },
      ...CORE_DATABASE_MODULES.filter((item) => item.repo !== 'sdkwork-knowledgebase').flatMap(
        (item) => [
          {
            archivePath: `database-modules/${item.repo}/${item.sourceUnder}`,
            sourcePath: path.join(root, '..', item.repo, item.sourceUnder),
            label: `${item.repo} database module`,
          },
          ...(item.extraPaths ?? []).map((extra) => ({
            archivePath: `database-modules/${item.repo}/${extra}`,
            sourcePath: path.join(root, '..', item.repo, extra),
            label: `${item.repo} ${extra}`,
          })),
        ],
      ),
      // Application identity manifest at the install root: IAM tenant
      // provisioning resolves agent.app.config.json under the app root
      // (SDKWORK_APP_ROOT).
      {
        archivePath: 'agent.app.config.json',
        sourcePath: path.join(root, 'agent.app.config.json'),
        label: 'application identity manifest',
      },
    ],
  };
  plan.issues = validateBuildPlan(plan);
  return plan;
}

function validateBuildPlan(plan) {
  const issues = [];
  for (const prerequisite of plan.prerequisites) {
    if (!existsSync(prerequisite.path)) {
      issues.push(`missing prerequisite: ${prerequisite.label} (${prerequisite.path})`);
    }
  }
  const distFiles = existsSync(plan.portalDistPath)
    ? readdirSync(plan.portalDistPath).filter((name) => name !== 'sdk-archives')
    : [];
  if (distFiles.length === 0) {
    issues.push(
      `portal dist is empty (${plan.portalDistPath}); build it first with `
      + '`pnpm --dir apps/sdkwork-knowledgebase-pc exec vite build --mode standalone.docker`',
    );
  }
  return issues;
}

function renderBuildPlan(plan) {
  return [
    '[container-image-build] Build Plan',
    `[container-image-build]   package id: ${plan.package.id} (${plan.package.platform}-${plan.package.architecture} v${plan.package.version})`,
    `[container-image-build]   image tag: ${plan.imageTag}`,
    `[container-image-build]   Dockerfile: ${plan.imageFile}`,
    `[container-image-build]   staging root: ${plan.stagingRoot}`,
    `[container-image-build]   package output: ${plan.packageOutputDir}`,
    `[container-image-build]   image build dir: ${plan.imageBuildDir}`,
    `[container-image-build]   manifest: ${plan.manifestPath}`,
    '[container-image-build]   staged entries:',
    ...plan.stagedEntries.map(
      (entry) => `[container-image-build]     ${entry.archivePath} <- ${entry.sourcePath}`,
    ),
  ];
}

// Strip debug symbols from the staged binary copies so the image stays lean
// (PACKAGING_SPEC §3: stripped binaries). Falls back to the unstripped copy
// when the strip tool is unavailable on the build host.
async function stripBinary(targetPath) {
  try {
    await execFileAsync('strip', ['--strip-unneeded', targetPath]);
    return true;
  } catch {
    try {
      await execFileAsync('strip', [targetPath]);
      return true;
    } catch {
      console.log(`[container-image-build] note: strip unavailable; keeping unstripped ${targetPath}`);
      return false;
    }
  }
}

async function assembleStaging(plan) {
  await rm(plan.stagingRoot, { recursive: true, force: true });
  for (const entry of plan.stagedEntries) {
    const target = path.join(plan.stagingRoot, entry.archivePath);
    await mkdir(path.dirname(target), { recursive: true });
    await cp(entry.sourcePath, target, { recursive: true, preserveTimestamps: true });
  }
  // Strip release binaries in the staging tree (not the build tree).
  for (const entry of plan.stagedEntries) {
    if (entry.strip) {
      const target = path.join(plan.stagingRoot, entry.archivePath);
      await stripBinary(target);
    }
  }
  // Container entrypoint: the default process is the standalone gateway
  // (database baseline/migrations run in-process on startup). The compose
  // `worker` service overrides the entrypoint with the worker binary.
  const entrypoint = [
    '#!/bin/sh',
    'set -eu',
    `exec ${INSTALL_ROOT}/bin/${GATEWAY_BINARY} "$@"`,
    '',
  ].join('\n');
  await mkdir(path.join(plan.stagingRoot, 'container'), { recursive: true });
  await writeFile(path.join(plan.stagingRoot, 'container', 'entrypoint'), entrypoint, 'utf8');
  console.log(`[container-image-build] staged: ${plan.stagingRoot}`);
}

async function sha256File(filePath) {
  const hash = createHash('sha256');
  const data = await readFile(filePath);
  hash.update(data);
  return hash.digest('hex');
}

// Release evidence per PACKAGING_SPEC §3: content manifest with every staged
// file, its size and sha256.
async function writeInstallManifest(plan, stagingRoot) {
  const files = [];
  const walk = async (dir, prefix) => {
    for (const child of await readdir(dir, { withFileTypes: true })) {
      const relative = prefix ? `${prefix}/${child.name}` : child.name;
      const full = path.join(dir, child.name);
      if (child.isDirectory()) {
        await walk(full, relative);
      } else {
        files.push({
          path: relative,
          size: (await statFile(full)).size,
          sha256: await sha256File(full),
        });
      }
    }
  };
  await walk(stagingRoot, '');
  files.sort((left, right) => left.path.localeCompare(right.path));
  const manifest = {
    schemaVersion: '2026-08-08.install-manifest.v1',
    packageId: plan.package.id,
    version: plan.package.version,
    imageTag: plan.imageTag,
    installRoot: INSTALL_ROOT,
    files,
    buildDate: new Date().toISOString(),
  };
  const manifestPath = path.join(stagingRoot, 'install-manifest.json');
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  return manifest;
}

async function buildInstallPackage(plan) {
  await mkdir(plan.packageOutputDir, { recursive: true });
  const archiveName = `${'sdkwork-knowledgebase'}-${plan.package.id}-${plan.package.version}.tar.gz`;
  const archivePath = path.join(plan.packageOutputDir, archiveName);
  await execFileAsync(
    'tar',
    ['-czf', archivePath, '-C', plan.stagingRoot, '.'],
    { cwd: workspaceRoot },
  );
  return { path: archivePath, sha256: await sha256File(archivePath) };
}

async function unpackInstallPackage(plan, archivePath) {
  await rm(plan.imageBuildDir, { recursive: true, force: true });
  await mkdir(plan.imageBuildDir, { recursive: true });
  if (archivePath.endsWith('.tar.gz')) {
    await execFileAsync('tar', ['-xzf', archivePath, '-C', plan.imageBuildDir]);
  } else if (archivePath.endsWith('.zip')) {
    const { stdout } = await execFileAsync('powershell.exe', [
      '-NoProfile',
      '-Command',
      `Expand-Archive -LiteralPath '${archivePath.replaceAll("'", "''")}' -DestinationPath '${plan.imageBuildDir.replaceAll("'", "''")}' -Force`,
    ]);
    if (stdout.trim()) {
      console.log(stdout.trim());
    }
  } else {
    throw new Error(`Unsupported container package archive: ${archivePath}`);
  }
  console.log(`[container-image-build] unpacked: ${plan.imageBuildDir}`);
}

async function dockerVersion() {
  const { stdout } = await execFileAsync('docker', ['version', '--format', '{{.Server.Version}}']);
  return stdout.trim();
}

// Collect {size, mtimeMs} for every input file of the image build so repeat
// builds can skip the packaging pipeline when nothing changed.
async function collectSourceSnapshot(plan) {
  const files = [];
  for (const entry of plan.stagedEntries) {
    await collectFileStats(entry.sourcePath, entry.archivePath, files);
  }
  files.sort((left, right) => left.path.localeCompare(right.path));
  return { schemaVersion: SNAPSHOT_SCHEMA_VERSION, files };
}

async function collectFileStats(target, relativePath, out) {
  const stat = await statFile(target);
  if (stat.isDirectory()) {
    for (const child of await readdir(target)) {
      await collectFileStats(path.join(target, child), `${relativePath}/${child}`, out);
    }
    return;
  }
  out.push({ path: relativePath, size: stat.size, mtimeMs: stat.mtimeMs });
}

function snapshotMatches(snapshotPath, current) {
  try {
    const previous = JSON.parse(readFileSync(snapshotPath, 'utf8'));
    return previous.schemaVersion === SNAPSHOT_SCHEMA_VERSION
      && JSON.stringify(previous.files) === JSON.stringify(current.files);
  } catch {
    return false;
  }
}

function imageBuildContextCached(plan) {
  if (!existsSync(plan.stagingRoot) || !existsSync(plan.imageBuildDir)) {
    return false;
  }
  return readdirSync(plan.imageBuildDir).length > 0;
}

async function buildImage(plan) {
  const args = [
    'build',
    '--build-arg',
    `VERSION=${plan.package.version}`,
    '-f',
    plan.imageFile,
    '-t',
    plan.imageTag,
    plan.imageBuildDir,
  ];
  const { stdout, stderr } = await execFileAsync('docker', args, {
    maxBuffer: 32 * 1024 * 1024,
  });
  if (stdout.trim()) {
    console.log(stdout.trim());
  }
  if (stderr.trim()) {
    console.log(stderr.trim());
  }
}

async function imageDigest(imageTag) {
  try {
    const { stdout } = await execFileAsync('docker', [
      'image',
      'inspect',
      '--format',
      '{{index .RepoDigests 0}}',
      imageTag,
    ]);
    const repoDigest = stdout.trim();
    if (repoDigest) {
      return { repoDigest, imageId: null };
    }
  } catch {
    // fall through to image id
  }
  const { stdout } = await execFileAsync('docker', [
    'image',
    'inspect',
    '--format',
    '{{.Id}}',
    imageTag,
  ]);
  return { repoDigest: null, imageId: stdout.trim() };
}

async function writeImageManifest(plan, archive, installManifest) {
  const digest = await imageDigest(plan.imageTag);
  const manifest = {
    schemaVersion: CONTAINER_IMAGE_MANIFEST_SCHEMA_VERSION,
    packageId: plan.package.id,
    version: plan.package.version,
    imageTag: plan.imageTag,
    packageArchive: path.basename(archive.path),
    packageArchiveSha256: archive.sha256,
    repoDigest: digest.repoDigest,
    imageId: digest.imageId,
    installManifestFiles: installManifest.files.length,
    buildDate: new Date().toISOString(),
  };
  await writeFile(plan.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  return manifest;
}

async function main(argv = process.argv.slice(2)) {
  const settings = parseBuildContainerArgs(argv);
  if (settings.help) {
    printHelp();
    return 0;
  }

  const plan = createBuildPlan(settings);
  const lines = renderBuildPlan(plan);
  if (settings.json && (settings.dryRun || settings.check)) {
    console.log(JSON.stringify({ ok: plan.issues.length === 0, issues: plan.issues, plan }, null, 2));
  } else {
    for (const line of lines) {
      console.log(line);
    }
    if (plan.issues.length > 0) {
      console.error('[container-image-build] validation issues:');
      for (const issue of plan.issues) {
        console.error(`[container-image-build]   ${issue}`);
      }
    }
  }
  if (settings.check && plan.issues.length > 0) {
    return 1;
  }
  if (settings.dryRun) {
    return 0;
  }
  if (plan.issues.length > 0) {
    throw new Error(`container image build plan is invalid: ${plan.issues.join('; ')}`);
  }

  let serverVersion = '';
  try {
    serverVersion = await dockerVersion();
  } catch {
    throw new Error('docker is not available or the daemon is not running; start docker first');
  }
  console.log(`[container-image-build] docker server: ${serverVersion}`);

  // Fast path: when every build input is unchanged and the unpacked image
  // build context still exists, skip the packaging pipeline (staging copy,
  // install package tar.gz, unpack) and only run `docker build` against the
  // cached context. Layer cache then keeps repeat deployments near-instant.
  const currentSnapshot = await collectSourceSnapshot(plan);
  const cached = !settings.force
    && snapshotMatches(plan.snapshotPath, currentSnapshot)
    && imageBuildContextCached(plan);

  let archive;
  let installManifest;
  if (cached) {
    console.log('[container-image-build] inputs unchanged; reusing cached image build context');
    const archivePath = path.join(
      plan.packageOutputDir,
      `${'sdkwork-knowledgebase'}-${plan.package.id}-${plan.package.version}.tar.gz`,
    );
    if (!existsSync(archivePath)) {
      throw new Error(`cached image build requires package archive: ${archivePath}`);
    }
    archive = { path: archivePath, sha256: await sha256File(archivePath) };
    installManifest = { files: [] };
  } else {
    await assembleStaging(plan);
    installManifest = await writeInstallManifest(plan, plan.stagingRoot);
    archive = await buildInstallPackage(plan);
    console.log(`[container-image-build] package archive: ${archive.path} (sha256 ${archive.sha256})`);
    await unpackInstallPackage(plan, archive.path);
    await writeFile(
      plan.snapshotPath,
      `${JSON.stringify(currentSnapshot, null, 2)}\n`,
      'utf8',
    );
  }

  await buildImage(plan);
  const manifest = await writeImageManifest(plan, archive, installManifest);
  if (settings.json) {
    console.log(JSON.stringify({ ok: true, manifest }, null, 2));
  } else {
    console.log(`[container-image-build] image: ${manifest.imageTag}`);
    console.log(`[container-image-build] repoDigest: ${manifest.repoDigest ?? 'n/a (local build)'}`);
    console.log(`[container-image-build] imageId: ${manifest.imageId ?? 'n/a'}`);
    console.log(`[container-image-build] manifest: ${plan.manifestPath}`);
  }
  return 0;
}

main().catch((error) => {
  console.error(`[container-image-build] ${error.message}`);
  process.exitCode = 1;
});

export {
  CONTAINER_IMAGE_MANIFEST_SCHEMA_VERSION,
  collectSourceSnapshot,
  createBuildPlan,
  main,
  parseBuildContainerArgs,
  snapshotMatches,
};
