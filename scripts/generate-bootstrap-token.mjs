#!/usr/bin/env node
/**
 * Generate a credential-entry bootstrap Access-Token JWT for the standalone
 * container deployment.
 *
 * The static portal bundle is built with `--mode standalone.development`, which
 * injects `process.env.SDKWORK_ACCESS_TOKEN` (see vite.config.ts) so the
 * browser sends the token on credential-entry requests (registrations/
 * sessions). The gateway resolves it through the IAM development
 * authentication fallback; signing with the same tenant signing master
 * secret keeps the token valid under any signature-verifying posture too.
 *
 * Claims follow sdkwork-web-core's DefaultAccessTokenParser contract:
 * token_version (1), tenant_id, app_id, login_scope (tenant), optional
 * organization_id/user_id/session_id.
 *
 * Usage:
 *   node scripts/generate-bootstrap-token.mjs \
 *     --tenant 100001 --app sdkwork-knowledgebase-pc \
 *     --secret sdkwork-knowledgebase-dev-signing-secret [--days 30]
 */

import { createHmac, createSign, randomBytes } from 'node:crypto';
import process from 'node:process';

function requireValue(argv, index, flag) {
  const value = argv[index + 1];
  if (!value || value.startsWith('--')) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function parseArgs(argv = process.argv.slice(2)) {
  const settings = {
    tenant: '100001',
    app: 'sdkwork-knowledgebase-pc',
    secret: 'sdkwork-knowledgebase-dev-signing-secret',
    days: 30,
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case '--tenant':
        settings.tenant = requireValue(argv, index, arg);
        index += 1;
        break;
      case '--app':
        settings.app = requireValue(argv, index, arg);
        index += 1;
        break;
      case '--secret':
        settings.secret = requireValue(argv, index, arg);
        index += 1;
        break;
      case '--days':
        settings.days = Number.parseInt(requireValue(argv, index, arg), 10);
        index += 1;
        break;
      case '-h':
      case '--help':
        settings.help = true;
        break;
      default:
        throw new Error(`Unknown option: ${arg}`);
    }
  }
  return settings;
}

function base64url(value) {
  return Buffer.from(value).toString('base64url');
}

function signHmacSha256(payload, secret) {
  const signature = createHmac('sha256', secret).update(payload).digest();
  return base64url(signature);
}

function main() {
  const settings = parseArgs();
  if (settings.help) {
    console.log(
      'Usage: node scripts/generate-bootstrap-token.mjs [--tenant 100001] [--app sdkwork-knowledgebase-pc]'
      + ' [--secret <tenant-signing-master-secret>] [--days 30]',
    );
    return;
  }
  const now = Math.floor(Date.now() / 1000);
  const header = { alg: 'HS256', typ: 'JWT' };
  const claims = {
    token_version: '1',
    tenant_id: settings.tenant,
    app_id: settings.app,
    login_scope: 'tenant',
    environment: 'development',
    deployment_mode: 'standalone',
    subject_type: 'service',
    session_id: `bootstrap-${randomBytes(8).toString('hex')}`,
    iat: now,
    exp: now + settings.days * 86400,
    iss: 'sdkwork-knowledgebase-standalone',
  };
  const encodedHeader = base64url(JSON.stringify(header));
  const encodedClaims = base64url(JSON.stringify(claims));
  const signingInput = `${encodedHeader}.${encodedClaims}`;
  const token = `${signingInput}.${signHmacSha256(signingInput, settings.secret)}`;
  process.stdout.write(`${token}\n`);
}

main();
