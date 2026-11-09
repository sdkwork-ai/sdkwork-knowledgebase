import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, '..');
const checkOnly = process.argv.includes('--check');

const BACKEND_PERMISSION = 'knowledge.platform.manage';
const BACKEND_AUDIT_EVENT = 'knowledge.backend.admin_operation';

const targets = [
  {
    relativePath: 'sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json',
    permission: BACKEND_PERMISSION,
    auditEvent: BACKEND_AUDIT_EVENT,
    tenantScope: 'tenant',
    dataScope: 'organization',
  },
  {
    // First-party app-api is a consumer surface (tier 0-2). PERMISSION_STANDARD_SPEC
    // §Surface Authorization Tiers forbids per-route RBAC scopes here: the shared
    // framework policy deliberately never blocks app-api, and space/document access is
    // enforced by the service layer's ownership/ACL checks. Any `x-sdkwork-permission`
    // or `x-sdkwork-auth-tier` on this surface is stale metadata that misleads SDK
    // consumers, so this target strips both unconditionally.
    relativePath: 'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
    stripConsumerAuthorization: true,
    tenantScope: 'tenant',
    dataScope: 'organization',
  },
];

const httpMethods = new Set(['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace']);

function isProtectedOperation(operation) {
  const security = operation.security;
  return Array.isArray(security) && security.length > 0;
}

function applyPermissions(document, target) {
  let changed = false;

  for (const pathItem of Object.values(document.paths ?? {})) {
    if (!pathItem || typeof pathItem !== 'object') {
      continue;
    }
    for (const method of httpMethods) {
      const operation = pathItem[method];
      if (!operation || typeof operation !== 'object' || !isProtectedOperation(operation)) {
        continue;
      }

      if (target.stripConsumerAuthorization) {
        if ('x-sdkwork-permission' in operation) {
          delete operation['x-sdkwork-permission'];
          changed = true;
        }
        if ('x-sdkwork-auth-tier' in operation) {
          delete operation['x-sdkwork-auth-tier'];
          changed = true;
        }
      } else {
        if (operation['x-sdkwork-permission'] !== target.permission) {
          operation['x-sdkwork-permission'] = target.permission;
          changed = true;
        }
        // Only mutations are emitted to the admin audit trail; declaring an audit event
        // on a read operation would promise evidence the runtime never records.
        if (target.auditEvent && method !== 'get') {
          if (operation['x-sdkwork-audit-event'] !== target.auditEvent) {
            operation['x-sdkwork-audit-event'] = target.auditEvent;
            changed = true;
          }
        } else if (method === 'get' && 'x-sdkwork-audit-event' in operation) {
          delete operation['x-sdkwork-audit-event'];
          changed = true;
        }
      }

      if (operation['x-sdkwork-tenant-scope'] !== target.tenantScope) {
        operation['x-sdkwork-tenant-scope'] = target.tenantScope;
        changed = true;
      }
      if (operation['x-sdkwork-data-scope'] !== target.dataScope) {
        operation['x-sdkwork-data-scope'] = target.dataScope;
        changed = true;
      }
    }
  }

  return changed;
}

let drifted = false;

for (const target of targets) {
  const filePath = path.join(workspaceRoot, target.relativePath);
  const raw = await readFile(filePath, 'utf8');
  const document = JSON.parse(raw);
  const changed = applyPermissions(document, target);

  if (checkOnly) {
    if (changed) {
      drifted = true;
      console.error(`OpenAPI permission metadata drift: ${target.relativePath}`);
    }
    continue;
  }

  if (changed) {
    await writeFile(filePath, `${JSON.stringify(document, null, 2)}\n`, 'utf8');
    console.log(`Applied permission metadata: ${target.relativePath}`);
  }
}

if (checkOnly && drifted) {
  process.exit(1);
}
