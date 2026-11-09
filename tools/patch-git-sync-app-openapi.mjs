#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  commandEnvelope,
  createdResponse,
} from './lib/openapi-envelope.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const targets = [
  path.join(root, 'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json'),
  path.join(root, 'apis/app-api/knowledgebase-app-api.openapi.json'),
];

const problemRef = { $ref: '#/components/schemas/ProblemDetail' };
const errorResponses = {
  400: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  401: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  403: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  404: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  409: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  429: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  500: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
};

const int64StringSchema = {
  type: 'string',
  format: 'uint64',
  pattern: '^[0-9]+$',
  'x-sdkwork-int64-string': true,
};

const gitSyncPath = {
  post: {
    operationId: 'gitSyncs.create',
    tags: ['knowledge'],
    summary: 'Sync knowledge space documents to a Git repository',
    security: [{ AuthToken: [], AccessToken: [] }],
    requestBody: {
      required: true,
      content: {
        'application/json': {
          schema: { $ref: '#/components/schemas/KnowledgeGitSyncRequest' },
        },
      },
    },
    responses: {
      ...errorResponses,
      201: createdResponse(commandEnvelope('#/components/schemas/KnowledgeGitSyncResult')),
    },
    'x-sdkwork-owner': 'sdkwork-knowledgebase',
    'x-sdkwork-api-authority': 'sdkwork-knowledgebase-app-api',
    'x-sdkwork-request-context': 'WebRequestContext',
    'x-sdkwork-api-surface': 'app-api',
    'x-sdkwork-source-route-crate': 'sdkwork-routes-knowledgebase-app-api',
    'x-sdkwork-rate-limit-tier': 'auth-critical',
    'x-sdkwork-auth-mode': 'dual-token',
  },
};

const gitSyncSchemas = {
  KnowledgeGitSyncRequest: {
    type: 'object',
    required: ['spaceId', 'repoUrl', 'commitMessage', 'idempotencyKey'],
    properties: {
      spaceId: int64StringSchema,
      repoUrl: { type: 'string', minLength: 1 },
      branch: { type: ['string', 'null'] },
      commitMessage: { type: 'string', minLength: 1 },
      idempotencyKey: { type: 'string', minLength: 1, maxLength: 128 },
      gitAccessToken: { type: ['string', 'null'] },
    },
  },
  KnowledgeGitSyncResult: {
    type: 'object',
    required: ['accepted', 'status', 'hash', 'syncedCount'],
    properties: {
      accepted: { type: 'boolean', const: true },
      status: { type: 'string', enum: ['completed'] },
      hash: { type: 'string', minLength: 1 },
      syncedCount: { type: 'integer', format: 'uint32', minimum: 0 },
    },
  },
};

for (const openapiPath of targets) {
  const spec = JSON.parse(await readFile(openapiPath, 'utf8'));
  spec.paths['/app/v3/api/knowledge/git_syncs'] = gitSyncPath;
  Object.assign(spec.components.schemas, gitSyncSchemas);
  await writeFile(openapiPath, `${JSON.stringify(spec, null, 2)}\n`, 'utf8');
  console.log(`patched ${openapiPath}`);
}
