import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  bindKnowledgebaseSessionStore,
  configureKnowledgebaseAppSdk,
  configureKnowledgebaseDriveAppSdk,
  readLocalDocumentContent,
  setKnowledgebaseApiEnabled,
  type SessionSnapshot,
  type SessionStore,
  writeLocalDocumentContent,
} from 'sdkwork-knowledgebase-pc-core';

import {
  createDocument,
  getDocumentContent,
  getDocuments,
  getKnowledgeBases,
  saveDocumentContent,
} from './knowledgebaseDocumentApiBridge';
import type { DocumentMeta } from './document';
import { activateEphemeralFixedKnowledgebaseWorkspace } from '../workspaceMode';
import { forgetKnowledgeSpaceKnowledgeMode } from './knowledgeSpaceKnowledgeMode';

const EMPTY_SESSION_STORE: SessionStore = createStaticSessionStore({});

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {
      // Test fake; session mutation is covered by pc-core session tests.
    },
    clearSession() {
      // Test fake; session clearing is covered by pc-core session tests.
    },
    subscribe() {
      return () => {
        // Test fake; listener lifecycle is not relevant for document bridge orchestration.
      };
    },
  };
}

function installWindowStorage(tenantId: string, spaceId: string | null): void {
  const localStorage = new Map<string, string>();
  const sessionStorage = new Map<string, string>();
  if (spaceId !== null) {
    localStorage.set(
      `sdkwork.knowledgebase.spaces.v1.${tenantId}`,
      JSON.stringify([
        {
          spaceId,
          kbType: 'team',
          createdAt: '2026-07-09T00:00:00.000Z',
        },
      ]),
    );
  }

  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage: storageFromMap(localStorage),
      sessionStorage: storageFromMap(sessionStorage),
    },
  });
}

function storageFromMap(values: Map<string, string>): Storage {
  return {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key: string) {
      return values.get(key) ?? null;
    },
    key(index: number) {
      return Array.from(values.keys())[index] ?? null;
    },
    removeItem(key: string) {
      values.delete(key);
    },
    setItem(key: string, value: string) {
      values.set(key, value);
    },
  };
}

function bindTenantContext(tenantId: string): void {
  bindKnowledgebaseSessionStore(
    createStaticSessionStore({
      context: {
        tenantId,
        userId: 'user-1',
      },
    }),
  );
}

afterEach(() => {
  bindKnowledgebaseSessionStore(EMPTY_SESSION_STORE);
  setKnowledgebaseApiEnabled(false);
  forgetKnowledgeSpaceKnowledgeMode();
  Reflect.deleteProperty(globalThis, 'window');
});

describe('KnowledgebaseDocumentApiBridge.createDocument', () => {
  it('uses ingest-created document identity for new documents with content', async () => {
    const documentsCreate = vi.fn();
    const ingestsCreate = vi.fn(async () => ({
      id: 'ingest-1',
      state: 'succeeded',
    }));
    const browserList = vi.fn(async () => ({
      spaceId: '42',
      parentId: null,
      view: 'files',
      pageSize: 100,
      items: [
        {
          id: 'browser-doc-991',
          nodeType: 'document',
          name: 'Research Note',
          path: '/Research Note.md',
          documentId: '991',
          mimeType: 'text/markdown',
          updatedAt: '2026-07-09T00:00:00.000Z',
          permissions: {
            canRead: true,
            canWrite: true,
            canDelete: true,
          },
        },
      ],
      pageInfo: {
        mode: 'cursor',
        hasMore: false,
        nextCursor: null,
      },
    }));

    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '42');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          documents: {
            create: documentsCreate,
          },
          ingests: {
            create: ingestsCreate,
          },
          spaces: {
            browser: {
              list: browserList,
            },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    const created = await createDocument({
      kbId: '42',
      title: 'Research Note',
      type: 'markdown',
      content: '# Research Note',
    });

    expect(documentsCreate).not.toHaveBeenCalled();
    expect(ingestsCreate).toHaveBeenCalledWith({
      spaceId: '42',
      title: 'Research Note',
      payloadMarkdown: '# Research Note',
      idempotencyKey: expect.stringMatching(/^pc-create-42-/),
    });
    expect(browserList).toHaveBeenCalledWith('42', {
      view: 'files',
      parentId: null,
      cursor: null,
      pageSize: 100,
    });
    expect(created.id).toBe('991');
    expect(created.title).toBe('Research Note');
    expect(created.kbId).toBe('42');
    expect(readLocalDocumentContent('tenant-1', '991')).toBe('# Research Note');
  });

  it('does not access browser storage for an active fixed group workspace', async () => {
    const localStorage = createStorageSpy();
    const sessionStorage = createStorageSpy();
    const directContentLookup = vi.fn(async () => ({
      documentId: '991',
      contentMarkdown: '# Group-only document',
      contentVersion: 'group-v1',
    }));
    const directDocumentLookup = vi.fn();
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: { localStorage, sessionStorage },
    });
    bindTenantContext('tenant-1');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          documents: {
            retrieve: directDocumentLookup,
            content: {
              list: directContentLookup,
            },
          },
          ingests: {
            create: async () => ({ id: 'ingest-group-1', state: 'succeeded' }),
          },
          retrievals: {
            create: async () => ({
              hits: [
                {
                  documentId: '991',
                  title: 'Group source',
                  content: '# Group-only document',
                },
              ],
            }),
          },
          spaces: {
            // The group workspace space is chunk-indexed: the document body is
            // read through retrieval, which the gateway serves for RAG mode.
            retrieve: async () => ({ id: '420', knowledgeMode: 'rag' }),
            browser: {
              list: async () => ({
                spaceId: '420',
                parentId: null,
                view: 'files',
                pageSize: 100,
                items: [
                  {
                    id: 'browser-doc-991',
                    nodeType: 'document',
                    name: 'Group source',
                    path: '/Group source.md',
                    documentId: '991',
                    mimeType: 'text/markdown',
                    updatedAt: '2026-07-13T00:00:00.000Z',
                    permissions: {
                      canRead: true,
                      canWrite: true,
                      canDelete: true,
                    },
                  },
                  {
                    id: 'browser-doc-992',
                    nodeType: 'document',
                    name: 'Group draft',
                    path: '/Group draft.md',
                    documentId: '992',
                    mimeType: 'text/markdown',
                    updatedAt: '2026-07-13T00:00:00.000Z',
                    permissions: {
                      canRead: true,
                      canWrite: true,
                      canDelete: true,
                    },
                  },
                ],
                pageInfo: {
                  mode: 'cursor',
                  hasMore: false,
                  nextCursor: null,
                },
              }),
            },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    const releaseWorkspace = activateEphemeralFixedKnowledgebaseWorkspace('420');
    try {
      await expect(getDocumentContent('991')).resolves.toBe('# Group-only document');
      await expect(saveDocumentContent('991', '# Updated group document')).resolves.toMatchObject({ saved: true });
      await expect(createDocument({
        kbId: '420',
        title: 'Group draft',
        type: 'markdown',
        content: '# New group draft',
      })).resolves.toMatchObject({ id: '992', kbId: '420' });
    } finally {
      releaseWorkspace();
    }

    expect(directContentLookup).not.toHaveBeenCalled();
    expect(directDocumentLookup).not.toHaveBeenCalled();
    expectStorageUnused(localStorage);
    expectStorageUnused(sessionStorage);
  });
});

describe('KnowledgebaseDocumentApiBridge.getDocuments', () => {
  it('lists authored notes that have no drive node next to the drive tree', async () => {
    const browserList = vi.fn(async () => ({
      spaceId: '77',
      parentId: null,
      view: 'files',
      pageSize: 100,
      items: [
        {
          id: 'browser-doc-991',
          nodeType: 'document',
          name: 'Drive source',
          path: '/Drive source.md',
          documentId: '991',
          mimeType: 'text/markdown',
          updatedAt: '2026-07-09T00:00:00.000Z',
          permissions: { canRead: true, canWrite: true, canDelete: true },
        },
      ],
      pageInfo: { mode: 'cursor', hasMore: false, nextCursor: null },
    }));
    const documentsList = vi.fn(async () => ({
      items: [
        {
          id: '991',
          spaceId: '77',
          title: 'Drive source',
          mimeType: 'text/markdown',
          originalFileDriveNodeId: 'drive-991',
          currentVersionId: 'v-991',
        },
        {
          id: '1200',
          spaceId: '77',
          title: '未命名笔记',
          mimeType: 'text/plain',
          originalFileDriveNodeId: null,
          currentVersionId: null,
        },
        {
          id: '1201',
          spaceId: '77',
          title: 'Uploaded PDF',
          mimeType: 'application/pdf',
          originalFileDriveNodeId: 'drive-1201',
          currentVersionId: 'v-1201',
        },
      ],
      pageInfo: { mode: 'offset', hasMore: false, nextCursor: null },
    }));

    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '77');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          documents: { list: documentsList },
          spaces: { browser: { list: browserList } },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    const tree = await getDocuments('77');

    const ids = tree.map((item) => item.id);
    // The note has no drive node, so the browser tree alone would hide it; it
    // must still be listed (newest first) exactly once. The drive-backed
    // document and the upload are not repeated by the notes listing.
    expect(ids).toEqual(['1200', '991']);
    const note = tree[0] as DocumentMeta;
    expect(note.title).toBe('未命名笔记');
    expect(note.type).toBe('richtext');
    expect(note.kbId).toBe('77');
    expect(documentsList).toHaveBeenCalledWith(expect.objectContaining({ spaceId: '77' }));
  });

  it('keeps the drive tree usable when the notes listing fails', async () => {
    const browserList = vi.fn(async () => ({
      spaceId: '78',
      parentId: null,
      view: 'files',
      pageSize: 100,
      items: [
        {
          id: 'browser-doc-992',
          nodeType: 'document',
          name: 'Only drive document',
          path: '/Only drive document.md',
          documentId: '992',
          mimeType: 'text/markdown',
          updatedAt: '2026-07-09T00:00:00.000Z',
          permissions: { canRead: true, canWrite: true, canDelete: true },
        },
      ],
      pageInfo: { mode: 'cursor', hasMore: false, nextCursor: null },
    }));

    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '78');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          documents: {
            list: async () => {
              throw new Error('notes listing unavailable');
            },
          },
          spaces: { browser: { list: browserList } },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    await expect(getDocuments('78')).resolves.toEqual([
      expect.objectContaining({ id: '992', title: 'Only drive document' }),
    ]);
  });
});

function createStorageSpy(): Storage {  return {
    get length() {
      return 0;
    },
    clear: vi.fn(),
    getItem: vi.fn(() => null),
    key: vi.fn(() => null),
    removeItem: vi.fn(),
    setItem: vi.fn(),
  };
}

function expectStorageUnused(storage: Storage): void {
  expect(storage.getItem).not.toHaveBeenCalled();
  expect(storage.setItem).not.toHaveBeenCalled();
  expect(storage.removeItem).not.toHaveBeenCalled();
}

/** Build one Drive content response for a UTF-8 text object. */
function driveTextContentResponse(text: string): Record<string, unknown> {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return {
    nodeId: 'drive-okf-700',
    encoding: 'base64',
    content: btoa(binary),
    sizeBytes: String(bytes.byteLength),
    returnedRangeStart: '0',
    returnedRangeLength: String(bytes.byteLength),
    hasMore: false,
  };
}

describe('KnowledgebaseDocumentApiBridge content reads honour the space knowledge mode', () => {
  it('reads an OKF concept body from its Drive file without RAG retrieval', async () => {
    const retrievalsCreate = vi.fn();
    const conceptRetrieve = vi.fn(async () => ({
      title: 'Release Checklist',
      description: 'Steps that gate a release.',
      conceptId: 'release-checklist',
      tags: [],
    }));
    const browserList = vi.fn(async () => ({
      spaceId: '42',
      parentId: null,
      view: 'okf_bundle',
      pageSize: 100,
      items: [
        {
          id: 'drive-okf-700',
          nodeType: 'okf_concept',
          name: 'Release Checklist',
          path: 'okf/release-checklist.md',
          driveNodeId: 'drive-okf-700',
          conceptId: '700',
          mimeType: 'text/markdown',
          updatedAt: '2026-11-01T00:00:00.000Z',
          permissions: { canRead: true, canWrite: true, canDelete: true },
        },
      ],
      pageInfo: { mode: 'cursor', hasMore: false, nextCursor: null },
    }));
    const driveContentRetrieve = vi.fn(async () =>
      driveTextContentResponse('# Release Checklist\n\nGate every release on this list.'),
    );

    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '42');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          retrievals: { create: retrievalsCreate },
          okf: { concepts: { retrieve: conceptRetrieve } },
          spaces: {
            retrieve: async () => ({ id: '42', knowledgeMode: 'okf_bundle' }),
            browser: { list: browserList },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    configureKnowledgebaseDriveAppSdk({
      client: {
        drive: { nodes: { content: { retrieve: driveContentRetrieve } } },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    await expect(getDocumentContent('okf:42:700')).resolves.toBe(
      '# Release Checklist\n\nGate every release on this list.',
    );
    expect(driveContentRetrieve).toHaveBeenCalledWith(
      'drive-okf-700',
      expect.objectContaining({ encoding: 'base64' }),
    );
    // RAG retrieval is rejected for an OKF-bundle space, so it must never run.
    expect(retrievalsCreate).not.toHaveBeenCalled();
  });

  it('reads a text document from Drive instead of retrieval when the space is not RAG', async () => {
    const retrievalsCreate = vi.fn();
    const driveContentRetrieve = vi.fn(async () =>
      driveTextContentResponse('# Drive source body'),
    );

    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '42');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          retrievals: { create: retrievalsCreate },
          documents: {
            content: {
              list: async () => {
                throw new Error('no authoritative content version');
              },
            },
            retrieve: async () => ({
              id: '991',
              spaceId: '42',
              title: 'Drive source',
              currentVersionId: 'v-991',
            }),
          },
          spaces: {
            retrieve: async () => ({ id: '42', knowledgeMode: 'okf_bundle' }),
            browser: {
              list: async () => ({
                spaceId: '42',
                parentId: null,
                view: 'files',
                pageSize: 100,
                items: [
                  {
                    id: 'drive-991',
                    nodeType: 'document',
                    name: 'Drive source',
                    path: 'sources/raw/Drive source.md',
                    driveNodeId: 'drive-991',
                    documentId: '991',
                    mimeType: 'text/markdown',
                    updatedAt: '2026-11-01T00:00:00.000Z',
                    permissions: { canRead: true, canWrite: true, canDelete: true },
                  },
                ],
                pageInfo: { mode: 'cursor', hasMore: false, nextCursor: null },
              }),
            },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    configureKnowledgebaseDriveAppSdk({
      client: {
        drive: { nodes: { content: { retrieve: driveContentRetrieve } } },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    await expect(getDocumentContent('991')).resolves.toBe('# Drive source body');
    expect(retrievalsCreate).not.toHaveBeenCalled();
  });

  it('keeps reading a RAG space document through retrieval', async () => {
    const retrievalsCreate = vi.fn(async () => ({
      hits: [
        {
          documentId: '992',
          title: 'RAG source',
          content: '# RAG source body',
        },
      ],
    }));

    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '42');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          retrievals: { create: retrievalsCreate },
          documents: {
            content: {
              list: async () => {
                throw new Error('no authoritative content version');
              },
            },
            retrieve: async () => ({
              id: '992',
              spaceId: '42',
              title: 'RAG source',
              currentVersionId: 'v-1',
            }),
          },
          spaces: {
            retrieve: async () => ({ id: '42', knowledgeMode: 'rag' }),
            browser: {
              list: async () => ({
                spaceId: '42',
                parentId: null,
                view: 'files',
                pageSize: 100,
                items: [],
                pageInfo: { mode: 'cursor', hasMore: false, nextCursor: null },
              }),
            },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    await expect(getDocumentContent('992')).resolves.toBe('# RAG source body');
    expect(retrievalsCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        query: 'RAG source',
        bindings: [expect.objectContaining({ spaceId: '42' })],
      }),
    );
  });
});

describe('KnowledgebaseDocumentApiBridge.getDocumentContent', () => {
  it('refuses a cached body when the authoritative read is denied', async () => {
    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '42');
    writeLocalDocumentContent('tenant-1', '99', '# Cached secret', 'v1');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          documents: {
            content: {
              list: async () => {
                throw { status: 403, code: 'knowledge_space_access_denied' };
              },
            },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    await expect(getDocumentContent('99')).rejects.toMatchObject({ status: 403 });
    // The denied copy is evicted so a later offline read cannot resurrect it.
    expect(readLocalDocumentContent('tenant-1', '99')).toBeUndefined();
  });

  it('serves the cached body when the authoritative read is transiently unavailable', async () => {
    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '42');
    writeLocalDocumentContent('tenant-1', '99', '# Offline copy', 'v1');
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          documents: {
            content: {
              list: async () => {
                throw { status: 503 };
              },
            },
          },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);

    await expect(getDocumentContent('99')).resolves.toBe('# Offline copy');
  });
});

describe('KnowledgebaseDocumentApiBridge.getKnowledgeBases', () => {
  function configureSpaceSdk(
    retrieve: (spaceId: string) => Promise<{ id: string; name: string }>,
    create: () => Promise<{ id: string; name: string }>,
  ): void {
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          spaces: { retrieve, create },
        },
      } as never,
      setTokenManager() {
        // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
      },
    });
    setKnowledgebaseApiEnabled(true);
  }

  it('drops a space this account may not open and provisions its own', async () => {
    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '77');
    const retrieve = vi.fn(async (spaceId: string) => {
      if (spaceId === '77') {
        throw {
          status: 403,
          code: 'knowledge_space_access_denied',
          detail: 'actor 9 does not have access to space 77',
        };
      }
      return { id: spaceId, name: '我的知识库' };
    });
    const create = vi.fn(async () => ({ id: '501', name: '我的知识库' }));
    configureSpaceSdk(retrieve, create);

    const grouped = await getKnowledgeBases();

    expect(create).toHaveBeenCalledWith({
      name: '我的知识库',
      description: 'Default knowledge space',
    });
    expect(grouped.personal.map((kb) => kb.id)).toEqual(['501']);
    expect(grouped.unavailableSpaceIds).toEqual([]);

    // The denied reference is gone: the next round reads only the account's own space.
    const nextRetrieve = vi.fn(async (spaceId: string) => ({ id: spaceId, name: '我的知识库' }));
    configureSpaceSdk(nextRetrieve, create);
    await getKnowledgeBases();
    expect(nextRetrieve).not.toHaveBeenCalledWith('77');
  });

  it('keeps a space reference through a transient read failure', async () => {
    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', '77');
    const retrieve = vi.fn(async () => {
      throw { status: 503, code: 'service_unavailable' };
    });
    const create = vi.fn(async () => ({ id: '501', name: '我的知识库' }));
    configureSpaceSdk(retrieve, create);

    const grouped = await getKnowledgeBases();

    expect(grouped.unavailableSpaceIds).toEqual(['77']);
    expect(create).not.toHaveBeenCalled();
  });

  it('creates one default space for concurrent first listings', async () => {
    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', null);
    let created = 0;
    const create = vi.fn(async () => {
      created += 1;
      const id = `50${created}`;
      await new Promise((resolve) => setTimeout(resolve, 0));
      return { id, name: '我的知识库' };
    });
    const retrieve = vi.fn(async (spaceId: string) => ({ id: spaceId, name: '我的知识库' }));
    configureSpaceSdk(retrieve, create);

    const [first, second] = await Promise.all([getKnowledgeBases(), getKnowledgeBases()]);

    expect(create).toHaveBeenCalledTimes(1);
    expect(first.personal.map((kb) => kb.id)).toEqual(['501']);
    expect(second.personal.map((kb) => kb.id)).toEqual(['501']);
  });

  it('retries a failed default-space creation on the next round', async () => {
    bindTenantContext('tenant-1');
    installWindowStorage('tenant-1', null);
    const create = vi.fn(async () => {
      throw { status: 503, code: 'service_unavailable' };
    });
    configureSpaceSdk(vi.fn(async (spaceId: string) => ({ id: spaceId, name: 'x' })), create);

    await expect(getKnowledgeBases()).rejects.toMatchObject({ status: 503 });
    await expect(getKnowledgeBases()).rejects.toMatchObject({ status: 503 });

    // The single-flight entry is a request de-duplicator, not a cached failure.
    expect(create).toHaveBeenCalledTimes(2);
  });
});
