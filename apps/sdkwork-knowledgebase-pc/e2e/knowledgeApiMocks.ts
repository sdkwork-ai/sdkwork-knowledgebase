import type { Page, Route } from '@playwright/test';

const E2E_SPACE = {
  id: 1,
  uuid: '00000000-0000-0000-0000-000000000001',
  name: 'E2E Knowledge Base',
  description: null,
  driveSpaceId: 'drv-e2e-1',
  status: 'active',
  okfBundleInitialized: true,
  knowledgeMode: 'okf_bundle',
};

const E2E_TRACE_ID = '00000000-0000-4000-8000-000000000001';

export const E2E_SOURCE_DOCUMENT = {
  id: '1001',
  numericId: 1001,
  title: 'E2E Source Document',
  content: 'Launch readiness verification content for retrieval citations.',
};

export const E2E_SESSION_STORAGE_KEY = 'sdkwork-knowledgebase-pc-session';
export const E2E_SPACE_REGISTRY_KEY = 'sdkwork.knowledgebase.spaces.v1.1';

export interface E2eMockTelemetry {
  ingestPayloads: string[];
  createdDocumentIds: number[];
  requestedPaths: string[];
}

export function createE2eMockTelemetry(): E2eMockTelemetry {
  return {
    ingestPayloads: [],
    createdDocumentIds: [],
    requestedPaths: [],
  };
}

export function buildE2eSessionPayload() {
  return {
    authToken: 'e2e-auth-token',
    accessToken: 'e2e-access-token',
    context: {
      tenantId: '1',
      userId: '42',
      actorId: '42',
    },
    user: {
      id: '42',
      displayName: 'E2E User',
      email: 'e2e@sdkwork.local',
    },
  };
}

export function buildE2eSpaceRegistryPayload() {
  return [
    {
      spaceId: 1,
      kbType: 'personal',
      icon: '📘',
      createdAt: '2026-01-01T00:00:00.000Z',
    },
  ];
}

export async function seedKnowledgebaseE2eSession(page: Page): Promise<void> {
  const session = buildE2eSessionPayload();
  const registry = buildE2eSpaceRegistryPayload();

  await page.addInitScript(
    ({ sessionKey, registryKey, sessionValue, registryValue }) => {
      const sessionJson = JSON.stringify(sessionValue);
      const registryJson = JSON.stringify(registryValue);
      window.localStorage.setItem(sessionKey, sessionJson);
      window.localStorage.setItem(registryKey, registryJson);
      window.sessionStorage.setItem(sessionKey, sessionJson);
      window.sessionStorage.setItem(registryKey, registryJson);
    },
    {
      sessionKey: E2E_SESSION_STORAGE_KEY,
      registryKey: E2E_SPACE_REGISTRY_KEY,
      sessionValue: session,
      registryValue: registry,
    },
  );
}

function json(route: Route, status: number, body: unknown) {
  return route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify(body),
  });
}

function sdkworkItem(route: Route, status: number, item: unknown) {
  return sdkworkData(route, status, { item });
}

function sdkworkData(route: Route, status: number, data: unknown) {
  return json(route, status, {
    code: 0,
    data,
    traceId: E2E_TRACE_ID,
  });
}

function sdkworkProblem(
  route: Route,
  status: number,
  code: number,
  title: string,
  detail: string,
) {
  return route.fulfill({
    status,
    contentType: 'application/problem+json',
    body: JSON.stringify({
      type: 'about:blank',
      title,
      status,
      detail,
      code,
      traceId: E2E_TRACE_ID,
    }),
  });
}

function createMockState(telemetry?: E2eMockTelemetry) {
  let nextDocumentId = E2E_SOURCE_DOCUMENT.numericId;
  let nextIngestId = 1;
  const agentProfileId = 'e2e-agent-profile-1';
  const documents = new Map<number, {
    content: string;
    id: number;
    mimeType: string;
    spaceId: number;
    title: string;
  }>();

  documents.set(E2E_SOURCE_DOCUMENT.numericId, {
    content: E2E_SOURCE_DOCUMENT.content,
    id: E2E_SOURCE_DOCUMENT.numericId,
    spaceId: 1,
    title: E2E_SOURCE_DOCUMENT.title,
    mimeType: 'text/markdown',
  });

  return {
    agentProfileId,
    documents,
    nextDocumentId,
    nextIngestId,
    telemetry,
  };
}

type MockState = ReturnType<typeof createMockState>;

function buildRetrievalHits(query: string) {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return [];
  }

  return [
    {
      documentId: E2E_SOURCE_DOCUMENT.id,
      spaceId: '1',
      title: E2E_SOURCE_DOCUMENT.title,
      content: E2E_SOURCE_DOCUMENT.content,
      retrievalMethod: 'vector',
      score: 0.92,
      citation: {
        sourceUri: null,
      },
    },
  ];
}

async function handleKnowledgeRoute(route: Route, state: MockState): Promise<void> {
  const request = route.request();
  const url = new URL(request.url());
  const { pathname } = url;
  const method = request.method().toUpperCase();
  if (method === 'POST' && /\/knowledge\/spaces\/?$/.test(pathname)) {
    const body = request.postDataJSON() as { name?: string; description?: string | null } | null;
    await sdkworkItem(route, 201, {
      id: E2E_SPACE.id,
      uuid: E2E_SPACE.uuid,
      name: body?.name?.trim() || E2E_SPACE.name,
      description: body?.description ?? E2E_SPACE.description,
      driveSpaceId: E2E_SPACE.driveSpaceId,
      status: E2E_SPACE.status,
      okfBundleInitialized: E2E_SPACE.okfBundleInitialized,
      knowledgeMode: E2E_SPACE.knowledgeMode,
    });
    return;
  }

  if (method === 'GET' && pathname.endsWith('/knowledge/spaces')) {
    // Space collection listing: backs getKnowledgeBases (notes workspace KB
    // resolution, KB view sidebar).
    await sdkworkData(route, 200, {
      items: [E2E_SPACE],
      pageInfo: { mode: 'offset', nextCursor: null, hasMore: false },
    });
    return;
  }

  if (method === 'GET' && /\/knowledge\/spaces\/\d+$/.test(pathname)) {
    await sdkworkItem(route, 200, E2E_SPACE);
    return;
  }

  if (method === 'GET' && /\/knowledge\/spaces\/\d+\/browser$/.test(pathname)) {
    const items = [...state.documents.values()].map((document) => ({
      id: `node-${document.id}`,
      nodeType: 'document',
      name: document.title,
      parentId: null,
      path: `/sources/raw/${document.title}`,
      driveSpaceId: E2E_SPACE.driveSpaceId,
      driveNodeId: `drive-node-${document.id}`,
      documentId: String(document.id),
      documentVersionId: String(document.id),
      conceptId: null,
      conceptRevisionId: null,
      mimeType: document.mimeType,
      sizeBytes: String(new TextEncoder().encode(document.content).byteLength),
      ingestState: 'succeeded',
      parseState: 'succeeded',
      indexState: 'succeeded',
      okfState: 'succeeded',
      childrenCount: '0',
      updatedAt: '2026-01-01T00:00:00.000Z',
      permissions: {
        canRead: true,
        canUpload: true,
        canRename: true,
        canMove: true,
        canDelete: true,
        canReview: true,
        canPublish: true,
      },
    }));
    await sdkworkData(route, 200, {
      spaceId: '1',
      driveSpaceId: E2E_SPACE.driveSpaceId,
      parentId: 'node-raw-root',
      view: 'files',
      pageSize: 100,
      items,
      pageInfo: {
        mode: 'cursor',
        hasMore: false,
        nextCursor: null,
      },
    });
    return;
  }

  if (method === 'GET' && /\/knowledge\/spaces\/\d+\/context_bindings$/.test(pathname)) {
    await json(route, 200, { items: [] });
    return;
  }

  if (method === 'GET' && /\/knowledge\/spaces\/\d+\/members$/.test(pathname)) {
    await json(route, 200, { members: [] });
    return;
  }

  if (method === 'GET' && pathname.endsWith('/knowledge/agent_profiles')) {
    await json(route, 200, { items: [{ profileId: state.agentProfileId }] });
    return;
  }

  if (method === 'POST' && pathname.endsWith('/knowledge/agent_profiles')) {
    await json(route, 200, {
      profileId: state.agentProfileId,
      name: 'E2E Agent Profile',
      knowledgeMode: 'okf_bundle',
      modelProviderId: 'provider.model.knowledgebase-contract',
      modelId: 'contract',
      agentImplementationId: 'plugin.intelligence.knowledgebase-contract',
      status: 'active',
    });
    return;
  }

  if (method === 'GET' && /\/knowledge\/agent_profiles\/[^/]+$/.test(pathname)) {
    await json(route, 200, {
      profileId: state.agentProfileId,
      name: 'E2E Agent Profile',
      knowledgeMode: 'okf_bundle',
      modelProviderId: 'provider.model.knowledgebase-contract',
      modelId: 'contract',
      agentImplementationId: 'plugin.intelligence.knowledgebase-contract',
      status: 'active',
    });
    return;
  }

  if (method === 'POST' && /\/knowledge\/agent_profiles\/[^/]+\/bindings$/.test(pathname)) {
    await json(route, 200, { profileId: state.agentProfileId, spaceId: '1', enabled: true });
    return;
  }

  if (method === 'POST' && /\/knowledge\/agent_profiles\/[^/]+\/chat$/.test(pathname)) {
    const body = request.postDataJSON() as { message?: string } | null;
    await json(route, 200, {
      answer: `E2E synthesized answer for: ${body?.message ?? 'query'}`,
      sessionId: 'e2e-session-1',
    });
    return;
  }

  if (method === 'GET' && pathname.endsWith('/knowledge/documents')) {
    // Offset-paginated space listing (documents.list): drives the notes
    // workspace list and the knowledge document tree.
    const url = new URL(request.url());
    const spaceId = Number(url.searchParams.get('space_id') ?? '1');
    const pageSize = Number(url.searchParams.get('page_size') ?? '100');
    const offset = Number(url.searchParams.get('cursor') ?? '0');
    const all = [...state.documents.values()].filter(
      (document) => document.spaceId === spaceId,
    );
    const items = all
      .slice(offset, offset + pageSize)
      .map((document) => ({
        ...document,
        visibility: 'private',
        language: 'en',
      }));
    const nextCursor = offset + pageSize < all.length ? String(offset + pageSize) : null;
    await sdkworkData(route, 200, {
      items,
      pageInfo: {
        mode: 'offset',
        nextCursor,
        hasMore: nextCursor !== null,
      },
    });
    return;
  }

  if (method === 'POST' && pathname.endsWith('/knowledge/documents')) {
    const body = request.postDataJSON() as { title?: string; spaceId?: number; mimeType?: string } | null;
    state.nextDocumentId += 1;
    const created = {
      content: '',
      id: state.nextDocumentId,
      spaceId: body?.spaceId ?? 1,
      title: body?.title?.trim() || 'New Document',
      mimeType: body?.mimeType ?? 'text/plain',
      visibility: 'private',
      language: 'en',
    };
    state.documents.set(created.id, created);
    state.telemetry?.createdDocumentIds.push(created.id);
    await sdkworkItem(route, 201, created);
    return;
  }

  if (method === 'GET' && /\/knowledge\/documents\/\d+\/content$/.test(pathname)) {
    const documentId = Number(pathname.split('/').at(-2));
    const document = state.documents.get(documentId);
    if (!document) {
      await sdkworkProblem(route, 404, 40401, 'Document not found', 'The document does not exist.');
      return;
    }
    await sdkworkItem(route, 200, {
      documentId: String(document.id),
      contentMarkdown: document.content,
      contentSource: 'e2e-memory',
      contentVersion: `e2e-${document.id}`,
    });
    return;
  }

  if (method === 'GET' && /\/knowledge\/documents\/\d+$/.test(pathname)) {
    const documentId = Number(pathname.split('/').pop());
    const document = state.documents.get(documentId);
    if (!document) {
      await sdkworkProblem(route, 404, 40401, 'Document not found', 'The document does not exist.');
      return;
    }
    await sdkworkItem(route, 200, {
      ...document,
      visibility: 'private',
      language: 'en',
    });
    return;
  }

  if (method === 'PATCH' && /\/knowledge\/documents\/\d+$/.test(pathname)) {
    // Notes workspace title autosave: documents.update renames in place.
    const documentId = Number(pathname.split('/').pop());
    const document = state.documents.get(documentId);
    if (!document) {
      await sdkworkProblem(route, 404, 40401, 'Document not found', 'The document does not exist.');
      return;
    }
    const body = request.postDataJSON() as { title?: string; mimeType?: string } | null;
    if (body?.title !== undefined) {
      document.title = body.title;
    }
    if (body?.mimeType !== undefined) {
      document.mimeType = body.mimeType;
    }
    await sdkworkItem(route, 200, {
      ...document,
      visibility: 'private',
      language: 'en',
    });
    return;
  }

  if (method === 'POST' && pathname.endsWith('/knowledge/ingests')) {
    const body = request.postDataJSON() as { payloadMarkdown?: string; title?: string } | null;
    if (body?.payloadMarkdown) {
      state.telemetry?.ingestPayloads.push(body.payloadMarkdown);
    }
    // Mirror the real Create-binding ingest pipeline: the markdown lands in a
    // document with the request's title, so content survives a page reload
    // exactly as it does against the real backend.
    if (body?.payloadMarkdown && typeof body.title === 'string') {
      for (const document of state.documents.values()) {
        if (document.title === body.title) {
          document.content = body.payloadMarkdown;
          break;
        }
      }
    }
    const ingestId = state.nextIngestId;
    state.nextIngestId += 1;
    await sdkworkItem(route, 202, {
      id: ingestId,
      state: 'succeeded',
      errorMessage: null,
    });
    return;
  }

  if (method === 'GET' && /\/knowledge\/ingests\/\d+$/.test(pathname)) {
    await sdkworkItem(route, 200, {
      id: Number(pathname.split('/').pop()),
      state: 'succeeded',
      errorMessage: null,
    });
    return;
  }

  if (method === 'POST' && pathname.endsWith('/knowledge/retrievals')) {
    const body = request.postDataJSON() as { query?: string } | null;
    await json(route, 200, {
      retrievalId: '1',
      hits: buildRetrievalHits(body?.query ?? ''),
    });
    return;
  }

  if (method === 'POST' && pathname.endsWith('/knowledge/okf/queries')) {
    const body = request.postDataJSON() as { query?: string } | null;
    await json(route, 200, {
      answerMarkdown: `### E2E retrieval insight\n\nBased on **《${E2E_SOURCE_DOCUMENT.title}》**, ${body?.query ?? 'your question'} aligns with launch readiness verification.\n\n- Cite [1] for source navigation\n- Follow-up: drill backup restore runbook`,
    });
    return;
  }

  if (method === 'GET' && pathname.endsWith('/knowledge/agent_profiles')) {
    await json(route, 200, { items: [] });
    return;
  }

  await sdkworkProblem(
    route,
    501,
    50001,
    'E2E route not implemented',
    `${method} ${pathname} has no Knowledgebase E2E mock.`,
  );
}

export async function mockKnowledgebaseAppApi(
  page: Page,
  telemetry: E2eMockTelemetry = createE2eMockTelemetry(),
): Promise<E2eMockTelemetry> {
  const state = createMockState(telemetry);

  page.on('request', (request) => {
    const url = new URL(request.url());
    telemetry.requestedPaths.push(`${request.method().toUpperCase()} ${url.pathname}`);
  });

  await page.route('**/app/v3/api/**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    await sdkworkProblem(
      route,
      501,
      50001,
      'E2E route not implemented',
      `${request.method().toUpperCase()} ${url.pathname} has no E2E mock.`,
    );
  });

  await page.route('**/app/v3/api/knowledge/**', async (route) => {
    await handleKnowledgeRoute(route, state);
  });

  await page.route('**/app/v3/api/iam/**', async (route) => {
    await json(route, 200, {
      id: '42',
      displayName: 'E2E User',
      email: 'e2e@sdkwork.local',
    });
  });

  await page.route('**/app/v3/api/drive/**', async (route) => {
    const request = route.request();
    const method = request.method().toUpperCase();
    const drivePath = new URL(request.url()).pathname;
    // Drive node rename (drive.nodes.update): project the new node name onto
    // the underlying mock document so renames survive reloads, mirroring the
    // real drive-backed metadata flow for browser-listed documents.
    const renameMatch = /^(?:PATCH|PUT) \/app\/v3\/api\/drive\/nodes\/drive-node-(\d+)$/.exec(
      `${method} ${drivePath}`,
    );
    if (renameMatch) {
      const body = request.postDataJSON() as { nodeName?: string } | null;
      const documentId = Number(renameMatch[1]);
      const document = state.documents.get(documentId);
      if (document && body?.nodeName !== undefined) {
        document.title = body.nodeName;
      }
      await json(route, 200, {
        id: `drive-node-${documentId}`,
        nodeName: body?.nodeName ?? document?.title ?? '',
        nodeType: 'document',
      });
      return;
    }
    await json(route, 200, { items: [], nodes: [], nextCursor: null });
  });

  return telemetry;
}

export async function setupKnowledgebaseE2ePage(page: Page, telemetry?: E2eMockTelemetry) {
  await seedKnowledgebaseE2eSession(page);
  return mockKnowledgebaseAppApi(page, telemetry ?? createE2eMockTelemetry());
}
