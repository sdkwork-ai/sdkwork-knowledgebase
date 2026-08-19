import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  APP_API_AUTHENTICATED_ONLY_MUTATIONS,
  APP_API_PERMISSION_BY_OPERATION,
} from '../scripts/patch-app-route-permissions.mjs';

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
    relativePath: 'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
    permissionByOperation: APP_API_PERMISSION_BY_OPERATION,
    tenantScope: 'tenant',
    dataScope: 'organization',
  },
];

const httpMethods = new Set(['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace']);

function isProtectedOperation(operation) {
  const security = operation.security;
  return Array.isArray(security) && security.length > 0;
}

function applyPermissions(document, { permission, permissionByOperation, auditEvent, tenantScope, dataScope }) {
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

      if (permissionByOperation) {
        // GET operations and authenticated-only mutations (for example
        // spaces.create) stay dual-token without an RBAC permission.
        if (
          method === 'get' ||
          APP_API_AUTHENTICATED_ONLY_MUTATIONS.has(operation.operationId)
        ) {
          if ('x-sdkwork-permission' in operation) {
            delete operation['x-sdkwork-permission'];
            changed = true;
          }
        } else {
          const operationPermission = permissionByOperation[operation.operationId];
          if (!operationPermission) {
            throw new Error(
              `missing app-api permission mapping for operation ${operation.operationId}`,
            );
          }
          if (operation['x-sdkwork-permission'] !== operationPermission) {
            operation['x-sdkwork-permission'] = operationPermission;
            changed = true;
          }
        }
      } else {
        if (operation['x-sdkwork-permission'] !== permission) {
          operation['x-sdkwork-permission'] = permission;
          changed = true;
        }
      }
      if (auditEvent && operation['x-sdkwork-audit-event'] !== auditEvent) {
        operation['x-sdkwork-audit-event'] = auditEvent;
        changed = true;
      }
      if (operation['x-sdkwork-tenant-scope'] !== tenantScope) {
        operation['x-sdkwork-tenant-scope'] = tenantScope;
        changed = true;
      }
      if (operation['x-sdkwork-data-scope'] !== dataScope) {
        operation['x-sdkwork-data-scope'] = dataScope;
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
