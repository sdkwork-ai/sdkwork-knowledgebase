#!/usr/bin/env node
/**
 * Windows-side launcher for the WSL standalone docker deployment.
 *
 * Resolves the WSL Ubuntu distribution and the workspace mount path, then
 * runs scripts/wsl-deploy.sh inside WSL with stdio inherited so the full
 * pipeline (toolchain install -> workspace sync -> cargo/vite/image builds ->
 * compose -> nginx -> hosts -> verification) runs interactively.
 *
 * Usage: pnpm deploy:docker:wsl [-- <wsl-deploy options>]
 *   e.g. pnpm deploy:docker:wsl -- --no-cargo
 *        pnpm deploy:docker:wsl -- --verify-only
 */

import { execFile } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const workspaceRoot = path.resolve(__dirname, '..');

const PREFERRED_DISTROS = ['Ubuntu-22.04', 'Ubuntu-24.04', 'Ubuntu'];

async function listWslDistros() {
  try {
    const { stdout } = await execFileAsync('wsl.exe', ['-l', '-v'], {
      windowsHide: true,
      encoding: 'utf16le',
    });
    return stdout;
  } catch {
    try {
      const { stdout } = await execFileAsync('wsl.exe', ['-l', '-v'], {
        windowsHide: true,
      });
      return stdout;
    } catch (error) {
      throw new Error(`wsl.exe failed: ${error.message}. Install WSL first.`);
    }
  }
}

function resolveDistro(raw) {
  const text = raw.replace(/\u0000/g, '');
  const lines = text.split(/\r?\n/u).map((line) => line.trim());
  for (const name of PREFERRED_DISTROS) {
    const hit = lines.find((line) => line.startsWith(name) && !line.includes('Stopped'));
    if (hit) {
      return hit.split(/\s+/u)[0].replaceAll('*', '').trim();
    }
  }
  const running = lines.find((line) => line.includes('Running') && line.includes('Ubuntu'));
  if (running) {
    return running.split(/\s+/u)[0].replaceAll('*', '').trim();
  }
  throw new Error(
    'No running WSL Ubuntu distribution found. Start one first: wsl.exe -d Ubuntu-22.04',
  );
}

function toWslPath(winPath) {
  const normalized = path.resolve(winPath).replaceAll('\\', '/');
  const drive = normalized.match(/^([A-Za-z]):/u);
  if (drive) {
    return `/mnt/${drive[1].toLowerCase()}${normalized.slice(2)}`;
  }
  return normalized;
}

function printHostsInstructions() {
  console.log(`
[deploy] Windows hosts not writable from this shell.
Run once in an elevated (Administrator) PowerShell:

  Add-Content -Path C:\\Windows\\System32\\drivers\\etc\\hosts -Value "\`n# sdkwork-knowledgebase docker test domains\`n127.0.0.1 testapikb.sdkwork.com testapikb.birdcoder.com testapikb.noaper.com testapikb.dtupay.com"
`);
}

async function tryUpdateWindowsHosts() {
  const hostsPath = 'C:\\Windows\\System32\\drivers\\etc\\hosts';
  const line = '127.0.0.1 testapikb.sdkwork.com testapikb.birdcoder.com testapikb.noaper.com testapikb.dtupay.com';
  try {
    const fs = await import('node:fs');
    const content = fs.readFileSync(hostsPath, 'utf8');
    if (content.includes('testapikb.sdkwork.com')) {
      console.log('[deploy] Windows hosts already contains the test domains');
      return;
    }
    fs.appendFileSync(
      hostsPath,
      `\n# sdkwork-knowledgebase docker test domains\n${line}\n`,
      'utf8',
    );
    console.log('[deploy] Windows hosts updated');
  } catch (error) {
    printHostsInstructions();
  }
}

async function main() {
  const args = process.argv.slice(2);
  const restIndex = args.indexOf('--');
  const deployArgs = restIndex >= 0 ? args.slice(restIndex + 1) : [];

  const distro = resolveDistro(await listWslDistros());
  const scriptPath = toWslPath(path.join(workspaceRoot, 'scripts', 'wsl-deploy.sh'));
  const sourceRoot = toWslPath(path.join(workspaceRoot, '..'));

  console.log(`[deploy] WSL distro: ${distro}`);
  console.log(`[deploy] workspace source: ${sourceRoot}`);
  console.log(`[deploy] invoking: wsl.exe -d ${distro} -- bash ${scriptPath} --source ${sourceRoot} ${deployArgs.join(' ')}`);

  const child = execFile(
    'wsl.exe',
    ['-d', distro, '--', 'bash', scriptPath, '--source', sourceRoot, ...deployArgs],
    { stdio: 'inherit', windowsHide: false },
    (error) => {
      if (error) {
        console.error(`[deploy] WSL deployment failed (exit ${error.code ?? 'unknown'})`);
        process.exitCode = typeof error.code === 'number' ? error.code : 1;
      }
    },
  );
  child.on('exit', async (code) => {
    if (code === 0) {
      await tryUpdateWindowsHosts();
    }
  });
}

main().catch((error) => {
  console.error(`[deploy] ${error.message}`);
  process.exitCode = 1;
});
