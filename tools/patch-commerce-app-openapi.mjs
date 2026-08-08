#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  commandEnvelope,
  createdResponse,
  jsonResponse,
  listEnvelope,
  listPaginationQueryParams,
  resourceEnvelope,
} from './lib/openapi-envelope.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const checkOnly = process.argv.includes('--check');
let drifted = false;
const targets = [
  path.join(root, 'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json'),
  path.join(root, 'apis/app-api/knowledgebase-app-api.openapi.json'),
];

const problemRef = { $ref: '#/components/schemas/ProblemDetails' };
const errorResponses = {
  400: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
  401: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
  403: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
  404: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
  409: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
  429: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
  500: { description: 'Error', content: { 'application/json': { schema: problemRef }, 'application/problem+json': { schema: problemRef } } },
};

const sdkworkExtensions = {
  'x-sdkwork-owner': 'sdkwork-knowledgebase',
  'x-sdkwork-api-authority': 'sdkwork-knowledgebase-app-api',
  'x-sdkwork-request-context': 'WebRequestContext',
  'x-sdkwork-api-surface': 'app-api',
  'x-sdkwork-source-route-crate': 'sdkwork-routes-knowledgebase-app-api',
  'x-sdkwork-rate-limit-tier': 'auth-critical',
  'x-sdkwork-auth-mode': 'dual-token',
};

// Commerce paths are applied after permission materialization, so each operation must
// carry the same authorization metadata as the authored manifest routes.
const commercePermissionExtensions = {
  'market.listings.list': {
    'x-sdkwork-permission': 'knowledge.market.read',
    'x-sdkwork-tenant-scope': 'tenant',
    'x-sdkwork-data-scope': 'organization',
  },
  'market.subscriptions.create': {
    'x-sdkwork-permission': 'knowledge.market.write',
    'x-sdkwork-tenant-scope': 'tenant',
    'x-sdkwork-data-scope': 'organization',
  },
  'market.subscriptions.delete': {
    'x-sdkwork-permission': 'knowledge.market.write',
    'x-sdkwork-tenant-scope': 'tenant',
    'x-sdkwork-data-scope': 'organization',
  },
  'mediaTasks.create': {
    'x-sdkwork-permission': 'knowledge.media.write',
    'x-sdkwork-tenant-scope': 'tenant',
    'x-sdkwork-data-scope': 'organization',
  },
};

const int64StringSchema = {
  type: 'string',
  format: 'uint64',
  pattern: '^[0-9]+$',
  'x-sdkwork-int64-string': true,
};
const nullableInt64StringSchema = {
  type: ['string', 'null'],
  format: 'uint64',
  pattern: '^[0-9]+$',
  'x-sdkwork-int64-string': true,
};

const commercePaths = {
  '/app/v3/api/knowledge/market/listings': {
    get: {
      operationId: 'market.listings.list',
      tags: ['knowledge'],
      summary: 'List knowledge market catalog listings',
      security: [{ AuthToken: [], AccessToken: [] }],
      parameters: [...listPaginationQueryParams],
      responses: {
        ...errorResponses,
        200: jsonResponse(listEnvelope('#/components/schemas/KnowledgeMarketCatalogItem')),
      },
      ...sdkworkExtensions,
      ...commercePermissionExtensions['market.listings.list'],
    },
  },
  '/app/v3/api/knowledge/market/subscriptions': {
    post: {
      operationId: 'market.subscriptions.create',
      tags: ['knowledge'],
      summary: 'Subscribe to a knowledge market listing',
      security: [{ AuthToken: [], AccessToken: [] }],
      requestBody: {
        required: true,
        content: {
          'application/json': {
            schema: { $ref: '#/components/schemas/KnowledgeMarketSubscriptionRequest' },
          },
        },
      },
      responses: {
        ...errorResponses,
        201: createdResponse(commandEnvelope('#/components/schemas/KnowledgeMarketSubscriptionResult')),
      },
      ...sdkworkExtensions,
      ...commercePermissionExtensions['market.subscriptions.create'],
    },
  },
  '/app/v3/api/knowledge/market/subscriptions/{listingId}': {
    delete: {
      operationId: 'market.subscriptions.delete',
      tags: ['knowledge'],
      summary: 'Unsubscribe from a knowledge market listing',
      security: [{ AuthToken: [], AccessToken: [] }],
      parameters: [
        {
          name: 'listingId',
          in: 'path',
          required: true,
          schema: int64StringSchema,
        },
      ],
      responses: {
        ...errorResponses,
        204: { description: 'No Content' },
      },
      ...sdkworkExtensions,
      ...commercePermissionExtensions['market.subscriptions.delete'],
    },
  },
  '/app/v3/api/knowledge/media_tasks': {
    post: {
      operationId: 'mediaTasks.create',
      tags: ['knowledge'],
      summary: 'Create a knowledge media task (image generation or speech-to-text)',
      security: [{ AuthToken: [], AccessToken: [] }],
      requestBody: {
        required: true,
        content: {
          'application/json': {
            schema: { $ref: '#/components/schemas/KnowledgeMediaTaskRequest' },
          },
        },
      },
      responses: {
        ...errorResponses,
        201: createdResponse(commandEnvelope('#/components/schemas/KnowledgeMediaTaskResult')),
      },
      ...sdkworkExtensions,
      ...commercePermissionExtensions['mediaTasks.create'],
    },
  },
};

const commerceSchemas = {
  KnowledgeDriveImportRequest: {
    type: 'object',
    required: ['spaceId', 'title', 'driveSpaceId', 'driveNodeId', 'idempotencyKey'],
    properties: {
      spaceId: int64StringSchema,
      title: { type: 'string', minLength: 1 },
      driveSpaceId: { type: 'string', minLength: 1, maxLength: 128 },
      driveNodeId: { type: 'string', minLength: 1, maxLength: 128 },
      idempotencyKey: { type: 'string', minLength: 1, maxLength: 128 },
      language: { type: ['string', 'null'] },
    },
  },
  KnowledgeDriveObjectRef: {
    type: 'object',
    required: ['id', 'spaceId', 'driveSpaceId', 'driveNodeId', 'sizeBytes', 'objectRole', 'accessMode'],
    properties: {
      id: int64StringSchema,
      spaceId: int64StringSchema,
      driveSpaceId: { type: ['string', 'null'] },
      driveNodeId: { type: ['string', 'null'] },
      logicalPath: { type: ['string', 'null'] },
      contentType: { type: ['string', 'null'] },
      sizeBytes: int64StringSchema,
      checksumSha256Hex: { type: ['string', 'null'] },
      objectRole: { type: 'string', minLength: 1 },
      accessMode: { type: 'string', minLength: 1 },
    },
  },
  KnowledgeMarketCatalogItem: {
    type: 'object',
    required: [
      'id',
      'title',
      'icon',
      'description',
      'author',
      'tags',
      'subscribersCount',
      'documentsCount',
      'provider',
      'modelName',
      'isSubscribed',
    ],
    properties: {
      id: { type: 'string', minLength: 1 },
      title: { type: 'string', minLength: 1 },
      icon: { type: 'string', minLength: 1 },
      description: { type: 'string' },
      author: { type: 'string', minLength: 1 },
      tags: { type: 'array', items: { type: 'string' } },
      subscribersCount: { type: 'integer', format: 'uint32', minimum: 0 },
      documentsCount: { type: 'integer', format: 'uint32', minimum: 0 },
      provider: { type: 'string', minLength: 1 },
      modelName: { type: 'string', minLength: 1 },
      isSubscribed: { type: 'boolean' },
    },
  },
  KnowledgeMarketSubscriptionRequest: {
    type: 'object',
    required: ['listingId'],
    properties: {
      listingId: int64StringSchema,
    },
  },
  KnowledgeMarketSubscriptionResult: {
    type: 'object',
    required: ['accepted', 'status'],
    properties: {
      accepted: { type: 'boolean', const: true },
      status: { type: 'string', enum: ['completed'] },
    },
  },
  KnowledgeMediaTaskType: {
    type: 'string',
    enum: ['generate_image', 'speech_to_text'],
  },
  KnowledgeMediaTaskRequest: {
    type: 'object',
    required: ['spaceId', 'taskType'],
    properties: {
      spaceId: int64StringSchema,
      taskType: { $ref: '#/components/schemas/KnowledgeMediaTaskType' },
      prompt: { type: ['string', 'null'] },
      aspectMode: { type: ['string', 'null'] },
      styleMode: { type: ['string', 'null'] },
      sourceUrl: { type: ['string', 'null'] },
      documentId: nullableInt64StringSchema,
    },
  },
  KnowledgeMediaTaskResult: {
    type: 'object',
    required: ['accepted', 'status', 'suggestions', 'similars'],
    properties: {
      accepted: { type: 'boolean', const: true },
      status: { type: 'string', enum: ['completed'] },
      url: { type: ['string', 'null'] },
      resolution: { type: ['string', 'null'] },
      text: { type: ['string', 'null'] },
      suggestions: { type: 'array', items: { type: 'string' } },
      similars: { type: 'array', items: { type: 'string' } },
    },
  },
};

for (const openapiPath of targets) {
  const before = await readFile(openapiPath, 'utf8');
  const spec = JSON.parse(before);
  delete spec.paths['/app/v3/api/knowledge/upload_sessions'];
  delete spec.paths['/app/v3/api/knowledge/upload_sessions/{sessionId}/complete'];
  for (const schemaName of [
    'CreateKnowledgeUploadSessionRequest',
    'CompleteKnowledgeUploadSessionRequest',
    'KnowledgeUploadSession',
    'KnowledgeUploadSessionStatus',
  ]) {
    delete spec.components.schemas[schemaName];
  }
  Object.assign(spec.paths, commercePaths);
  Object.assign(spec.components.schemas, commerceSchemas);
  const desired = `${JSON.stringify(spec, null, 2)}\n`;
  if (desired === before) {
    continue;
  }
  if (checkOnly) {
    drifted = true;
    console.error(`Commerce App OpenAPI drift: ${path.relative(root, openapiPath)}`);
  } else {
    await writeFile(openapiPath, desired, 'utf8');
    console.log(`patched ${openapiPath}`);
  }
}

if (drifted) process.exit(1);
