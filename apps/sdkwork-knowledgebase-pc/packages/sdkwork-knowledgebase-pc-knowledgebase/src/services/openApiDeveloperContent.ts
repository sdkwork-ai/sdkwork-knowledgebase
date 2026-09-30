/**
 * Static content for the Open API developer panel.
 *
 * Endpoint list mirrors the authoritative route manifest
 * (`sdks/_route-manifests/open-api/sdkwork-routes-knowledgebase-open-api.route-manifest.json`)
 * and the OpenAPI authority
 * (`sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json`).
 * Keep both in sync when the open-api surface changes.
 */

export type OpenApiEndpointMethod = 'GET' | 'POST';

export interface OpenApiEndpointDoc {
  method: OpenApiEndpointMethod;
  /** Path relative to the Open API base URL (`/knowledge/v3/api`). */
  path: string;
  operationId: string;
  summary: string;
  /** POST command endpoints accept an `Idempotency-Key` header. */
  idempotent: boolean;
}

export const OPEN_API_PREFIX = '/knowledge/v3/api';

export const OPEN_API_ENDPOINTS: OpenApiEndpointDoc[] = [
  {
    method: 'POST',
    path: '/retrievals',
    operationId: 'retrievals.create',
    summary: '创建知识检索请求',
    idempotent: true,
  },
  {
    method: 'GET',
    path: '/retrievals/{retrievalId}',
    operationId: 'retrievals.retrieve',
    summary: '查询检索结果',
    idempotent: false,
  },
  {
    method: 'POST',
    path: '/context_packs',
    operationId: 'contextPacks.create',
    summary: '创建知识上下文包',
    idempotent: true,
  },
  {
    method: 'POST',
    path: '/ingests',
    operationId: 'ingests.create',
    summary: '创建摄取任务',
    idempotent: true,
  },
  {
    method: 'GET',
    path: '/ingests/{ingestId}',
    operationId: 'ingests.retrieve',
    summary: '查询摄取任务状态',
    idempotent: false,
  },
  {
    method: 'GET',
    path: '/documents',
    operationId: 'documents.list',
    summary: '列出知识文档',
    idempotent: false,
  },
  {
    method: 'GET',
    path: '/documents/{documentId}',
    operationId: 'documents.retrieve',
    summary: '查询单个知识文档',
    idempotent: false,
  },
  {
    method: 'GET',
    path: '/spaces/{spaceId}/browser',
    operationId: 'spaces.browser.list',
    summary: '列出知识空间浏览器视图',
    idempotent: false,
  },
];

export function buildOpenApiCurlExample(
  openApiBaseUrl: string,
  endpoint: OpenApiEndpointDoc = OPEN_API_ENDPOINTS[0],
): string {
  const base = openApiBaseUrl.replace(/\/+$/, '');
  const fullBase = base.endsWith(OPEN_API_PREFIX) ? base : `${base}${OPEN_API_PREFIX}`;
  const command =
    endpoint.method === 'POST'
      ? `curl -X POST '${fullBase}${endpoint.path}'`
      : `curl '${fullBase}${endpoint.path}'`;
  const headers = [
    "  -H 'x-api-key: <你的 Open API Key>'",
  ];
  if (endpoint.idempotent) {
    headers.push("  -H 'Idempotency-Key: <UUID>'");
  }
  headers.push("  -H 'Content-Type: application/json'");
  const body = endpoint.method === 'POST' ? "  \\\n  -d '{ ... }'" : '';
  return [command, ...headers].join(' \\\n') + body;
}
