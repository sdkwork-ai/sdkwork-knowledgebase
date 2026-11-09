#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  commandEnvelope,
  jsonResponse,
} from './lib/openapi-envelope.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const openapiPath = path.join(
  root,
  'sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json',
);

const problemRef = { $ref: '#/components/schemas/ProblemDetail' };
const errorResponses = {
  400: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  401: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
  403: { description: 'Error', content: { 'application/problem+json': { schema: problemRef } } },
};

function op(id, summary, methodExtras) {
  return {
    operationId: id,
    tags: ['knowledge'],
    summary,
    security: [{ AuthToken: [], AccessToken: [] }],
    responses: {
      ...errorResponses,
      ...methodExtras.responses,
    },
    'x-sdkwork-owner': 'sdkwork-knowledgebase',
    'x-sdkwork-api-authority': 'sdkwork-knowledgebase-app-api',
    'x-sdkwork-request-context': 'WebRequestContext',
    'x-sdkwork-api-surface': 'app-api',
    'x-sdkwork-source-route-crate': 'sdkwork-routes-knowledgebase-app-api',
    'x-sdkwork-rate-limit-tier': methodExtras.rateLimit ?? 'standard',
    'x-sdkwork-auth-mode': 'dual-token',
    ...(methodExtras.body
      ? {
          requestBody: {
            required: true,
            content: {
              'application/json': {
                schema: { $ref: `#/components/schemas/${methodExtras.body}` },
              },
            },
          },
        }
      : {}),
  };
}

const spec = JSON.parse(await readFile(openapiPath, 'utf8'));

spec.paths['/app/v3/api/knowledge/wechat/official_accounts'] = {
  get: op('wechat.officialAccounts.list', 'List WeChat official accounts', {
    rateLimit: 'standard',
    responses: {
      200: {
        description: 'OK',
        content: {
          'application/json': {
            schema: { $ref: '#/components/schemas/KnowledgeWechatOfficialAccountList' },
          },
        },
      },
    },
  }),
  put: op('wechat.officialAccounts.update', 'Replace WeChat official accounts', {
    rateLimit: 'auth-critical',
    body: 'KnowledgeWechatReplaceOfficialAccountsRequest',
    responses: {
      200: {
        description: 'OK',
        content: {
          'application/json': {
            schema: { $ref: '#/components/schemas/KnowledgeWechatOfficialAccountList' },
          },
        },
      },
    },
  }),
};

spec.paths['/app/v3/api/knowledge/wechat/applets'] = {
  get: op('wechat.applets.list', 'List WeChat applets', {
    rateLimit: 'standard',
    responses: {
      200: {
        description: 'OK',
        content: {
          'application/json': {
            schema: { $ref: '#/components/schemas/KnowledgeWechatAppletList' },
          },
        },
      },
    },
  }),
  put: op('wechat.applets.update', 'Replace WeChat applets', {
    rateLimit: 'auth-critical',
    body: 'KnowledgeWechatReplaceAppletsRequest',
    responses: {
      200: {
        description: 'OK',
        content: {
          'application/json': {
            schema: { $ref: '#/components/schemas/KnowledgeWechatAppletList' },
          },
        },
      },
    },
  }),
};

spec.paths['/app/v3/api/knowledge/wechat/articles/publish'] = {
  post: op('wechat.articles.publish', 'Publish WeChat articles', {
    rateLimit: 'auth-critical',
    body: 'KnowledgeWechatArticlesPublishRequest',
      responses: {
      200: jsonResponse(commandEnvelope('#/components/schemas/KnowledgeWechatOperationResult')),
    },
  }),
};

spec.paths['/app/v3/api/knowledge/wechat/articles/preview'] = {
  post: op('wechat.articles.preview', 'Preview WeChat articles', {
    rateLimit: 'auth-critical',
    body: 'KnowledgeWechatArticlesPreviewRequest',
    responses: {
      200: jsonResponse(commandEnvelope('#/components/schemas/KnowledgeWechatOperationResult')),
    },
  }),
};

Object.assign(spec.components.schemas, {
  KnowledgeWechatOfficialAccount: {
    type: 'object',
    required: ['id', 'name', 'type', 'avatar', 'appId'],
    properties: {
      id: { type: 'string' },
      name: { type: 'string' },
      type: { type: 'string' },
      avatar: { type: 'string' },
      description: { type: 'string' },
      appId: { type: 'string' },
      appSecret: { type: 'string' },
      serverUrl: { type: 'string' },
      token: { type: 'string' },
      encodingAesKey: { type: 'string' },
      encryptMode: { type: 'string' },
      domainVerifyFileName: { type: 'string' },
      domainVerifyFileContent: { type: 'string' },
      jsSecureDomains: { type: 'array', items: { type: 'string' } },
      webAuthDomains: { type: 'array', items: { type: 'string' } },
      businessDomains: { type: 'array', items: { type: 'string' } },
      group: { type: 'string' },
    },
  },
  KnowledgeWechatOfficialAccountList: {
    type: 'object',
    required: ['accounts'],
    properties: {
      accounts: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeWechatOfficialAccount' },
      },
    },
  },
  KnowledgeWechatReplaceOfficialAccountsRequest: {
    type: 'object',
    required: ['accounts'],
    properties: {
      accounts: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeWechatOfficialAccount' },
      },
    },
  },
  KnowledgeWechatApplet: {
    type: 'object',
    required: ['id', 'name', 'appId', 'path', 'avatar'],
    properties: {
      id: { type: 'string' },
      name: { type: 'string' },
      appId: { type: 'string' },
      originalId: { type: 'string' },
      appSecret: { type: 'string' },
      path: { type: 'string' },
      avatar: { type: 'string' },
      group: { type: 'string' },
      description: { type: 'string' },
      requestDomain: { type: 'array', items: { type: 'string' } },
      socketDomain: { type: 'array', items: { type: 'string' } },
      uploadDomain: { type: 'array', items: { type: 'string' } },
      downloadDomain: { type: 'array', items: { type: 'string' } },
      udpDomain: { type: 'array', items: { type: 'string' } },
      tcpDomain: { type: 'array', items: { type: 'string' } },
      businessDomain: { type: 'array', items: { type: 'string' } },
      domainVerifyFileName: { type: 'string' },
      domainVerifyFileContent: { type: 'string' },
      msgToken: { type: 'string' },
      msgEncodingAESKey: { type: 'string' },
      msgDataFormat: { type: 'string' },
      msgEncryptMode: { type: 'string' },
    },
  },
  KnowledgeWechatAppletList: {
    type: 'object',
    required: ['applets'],
    properties: {
      applets: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeWechatApplet' },
      },
    },
  },
  KnowledgeWechatReplaceAppletsRequest: {
    type: 'object',
    required: ['applets'],
    properties: {
      applets: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeWechatApplet' },
      },
    },
  },
  KnowledgeWechatArticle: {
    type: 'object',
    required: ['id', 'title', 'author'],
    properties: {
      id: { type: 'string' },
      title: { type: 'string' },
      author: { type: 'string' },
      content: { type: 'string' },
      cover: { type: 'string' },
      abstract: { type: 'string' },
    },
  },
  KnowledgeWechatArticlesPublishRequest: {
    type: 'object',
    required: ['accountIds', 'articles'],
    properties: {
      accountIds: { type: 'array', items: { type: 'string' } },
      articles: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeWechatArticle' },
      },
      sendNotification: { type: 'boolean' },
      groupNotification: { type: 'boolean' },
      selectedGroupId: { type: 'string' },
      scheduleTime: { type: 'string' },
    },
  },
  KnowledgeWechatArticlesPreviewRequest: {
    type: 'object',
    required: ['accountId', 'wechatIds', 'articles'],
    properties: {
      accountId: { type: 'string' },
      wechatIds: { type: 'array', items: { type: 'string' } },
      articles: {
        type: 'array',
        items: { $ref: '#/components/schemas/KnowledgeWechatArticle' },
      },
    },
  },
  KnowledgeWechatOperationResult: {
    type: 'object',
    required: ['accepted', 'status'],
    properties: {
      accepted: { type: 'boolean', const: true },
      status: { type: 'string', enum: ['completed'] },
    },
  },
});

await writeFile(openapiPath, `${JSON.stringify(spec, null, 2)}\n`, 'utf8');
console.log('Patched WeChat operations into app OpenAPI spec.');
