import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, '..');

const targets = [
  'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
  'sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json',
  'sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json',
];

const problemResponse = {
  description: 'Error',
  content: {
    'application/json': {
      schema: { $ref: '#/components/schemas/ProblemDetails' },
    },
    'application/problem+json': {
      schema: { $ref: '#/components/schemas/ProblemDetails' },
    },
  },
};

const standardErrorStatuses = ['400', '401', '403', '404', '409', '429', '500'];

const rateLimitTierByOperationId = {
  'retrievals.create': 'write-heavy',
  'contextPacks.create': 'write-heavy',
  'ingests.create': 'write-heavy',
  'driveImports.create': 'write-heavy',
  'agentProfiles.chat.create': 'write-heavy',
  'agentProfiles.retrievalPreview.create': 'write-heavy',
  'okf.bundle.import.create': 'write-heavy',
  'okf.bundle.export.create': 'write-heavy',
  'spaces.delete': 'write-heavy',
  'documents.delete': 'write-heavy',
};

function humanizeOperationId(operationId) {
  return operationId
    .split('.')
    .map((segment) =>
      segment
        .replace(/([A-Z])/g, ' $1')
        .replace(/_/g, ' ')
        .trim(),
    )
    .join(' ')
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function ensureStandardResponses(operation) {
  operation.responses ??= {};
  for (const status of standardErrorStatuses) {
    if (!operation.responses[status]) {
      operation.responses[status] = structuredClone(problemResponse);
    }
  }
}

function syncOperation(operationId, operation) {
  if (!operation.summary) {
    operation.summary =
      operation.description?.trim() || humanizeOperationId(operationId);
  }
  if (!operation.description) {
    operation.description = operation.summary;
  }
  ensureStandardResponses(operation);
  const tier = rateLimitTierByOperationId[operationId];
  if (tier) {
    operation['x-sdkwork-rate-limit-tier'] = tier;
  }
}

for (const relativePath of targets) {
  const filePath = path.join(workspaceRoot, relativePath);
  const spec = JSON.parse(await readFile(filePath, 'utf8'));
  let updated = 0;

  for (const pathItem of Object.values(spec.paths ?? {})) {
    for (const operation of Object.values(pathItem)) {
      if (!operation || typeof operation !== 'object' || !operation.operationId) {
        continue;
      }
      syncOperation(operation.operationId, operation);
      updated += 1;
    }
  }

  await writeFile(filePath, `${JSON.stringify(spec, null, 2)}\n`, 'utf8');
  console.log(`Synced metadata for ${updated} operations in ${relativePath}`);
}
