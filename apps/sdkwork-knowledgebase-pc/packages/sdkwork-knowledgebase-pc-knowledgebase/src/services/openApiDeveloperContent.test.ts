import { describe, expect, it } from 'vitest';

import {
  buildOpenApiCurlExample,
  OPEN_API_ENDPOINTS,
  OPEN_API_PREFIX,
  type OpenApiEndpointDoc,
} from './openApiDeveloperContent';

describe('open api developer content', () => {
  it('exposes exactly the eight routes of the open-api manifest', () => {
    expect(OPEN_API_ENDPOINTS).toHaveLength(8);
    expect(OPEN_API_ENDPOINTS.map((endpoint) => endpoint.operationId).sort()).toEqual([
      'contextPacks.create',
      'documents.list',
      'documents.retrieve',
      'ingests.create',
      'ingests.retrieve',
      'retrievals.create',
      'retrievals.retrieve',
      'spaces.browser.list',
    ]);
  });

  it('marks every POST command endpoint as idempotent and every GET as not', () => {
    for (const endpoint of OPEN_API_ENDPOINTS) {
      expect(endpoint.idempotent).toBe(endpoint.method === 'POST');
    }
  });

  it('uses the /knowledge/v3/api prefix shared with the spec servers entry', () => {
    expect(OPEN_API_PREFIX).toBe('/knowledge/v3/api');
    for (const endpoint of OPEN_API_ENDPOINTS) {
      expect(endpoint.path.startsWith('/')).toBe(true);
      expect(endpoint.path.includes(' ')).toBe(false);
    }
  });

  it('builds a GET curl without body or idempotency header', () => {
    const getEndpoint = OPEN_API_ENDPOINTS.find(
      (endpoint) => endpoint.method === 'GET' && endpoint.path.includes('{documentId}'),
    ) as OpenApiEndpointDoc;
    const curl = buildOpenApiCurlExample('https://api.example.com', getEndpoint);
    expect(curl).toContain("curl 'https://api.example.com/knowledge/v3/api/documents/{documentId}'");
    expect(curl).toContain("'x-api-key: <你的 Open API Key>'");
    expect(curl).not.toContain('Idempotency-Key');
    expect(curl).not.toContain('-d');
  });

  it('builds a POST curl with idempotency header and JSON body', () => {
    const postEndpoint = OPEN_API_ENDPOINTS[0];
    expect(postEndpoint.method).toBe('POST');
    const curl = buildOpenApiCurlExample('https://api.example.com/', postEndpoint);
    expect(curl).toContain("curl -X POST 'https://api.example.com/knowledge/v3/api/retrievals'");
    expect(curl).toContain("'Idempotency-Key: <UUID>'");
    expect(curl).toContain("-d '{ ... }'");
  });

  it('trims trailing slashes from the base url exactly once', () => {
    const endpoint = OPEN_API_ENDPOINTS[0];
    const withSlash = buildOpenApiCurlExample('https://api.example.com//', endpoint);
    const withoutSlash = buildOpenApiCurlExample('https://api.example.com', endpoint);
    expect(withSlash).toBe(withoutSlash);
  });

  it('does not double-append the prefix when the base already ends with it', () => {
    const endpoint = OPEN_API_ENDPOINTS[0];
    const prefixed = buildOpenApiCurlExample('https://api.example.com/knowledge/v3/api', endpoint);
    const bare = buildOpenApiCurlExample('https://api.example.com', endpoint);
    expect(prefixed).toBe(bare);
    expect(prefixed.match(/knowledge\/v3\/api/g)).toHaveLength(1);
  });
});
