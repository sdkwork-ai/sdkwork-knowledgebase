import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  bindKnowledgebaseSessionStore,
  configureKnowledgebaseAppSdk,
  type SessionSnapshot,
  type SessionStore,
} from 'sdkwork-knowledgebase-pc-core';

import {
  activateWikiPublication,
  changeWikiSourceFileVisibility,
  isWikiPublishRowEligible,
  listWikiSourceFiles,
  newWikiIdempotencyKey,
  pauseWikiPublication,
  publishWikiSourceFile,
  retrieveWikiPublication,
  unpublishWikiSourceFile,
  WIKI_ACTIVATE_ELIGIBLE_STATUSES,
  WIKI_PAUSE_ELIGIBLE_STATUSES,
  WIKI_PUBLISH_ELIGIBLE_STATUSES,
} from './knowledgeWikiAdminService';

const EMPTY_SESSION_STORE = createStaticSessionStore({});

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {},
    clearSession() {},
    subscribe: () => () => {},
  };
}

interface WikiClientHarness {
  activateCalls: Array<{ spaceId: string; body: Record<string, unknown>; params: Record<string, unknown> }>;
  pauseCalls: Array<{ spaceId: string; body: Record<string, unknown>; params: Record<string, unknown> }>;
  retrieveCalls: string[];
  listCalls: Array<{ spaceId: string; params: Record<string, unknown> }>;
  publishCalls: Array<{ spaceId: string; uuid: string; body: Record<string, unknown>; params: Record<string, unknown> }>;
  unpublishCalls: Array<{ spaceId: string; uuid: string; body: Record<string, unknown>; params: Record<string, unknown> }>;
  visibilityCalls: Array<{ spaceId: string; uuid: string; body: Record<string, unknown>; params: Record<string, unknown> }>;
}

function configureWikiClient(harness: WikiClientHarness): void {
  bindKnowledgebaseSessionStore(
    createStaticSessionStore({ context: { tenantId: 'tenant-1', userId: 'user-1' } }),
  );
  configureKnowledgebaseAppSdk(
    {
      client: {
        knowledge: {
          wikiPublications: {
            retrieve: async (spaceId: string) => {
              harness.retrieveCalls.push(spaceId);
              return { uuid: 'pub-1', spaceId, status: 'ready', version: 'v7' };
            },
            activate: async (spaceId: string, body: Record<string, unknown>, params: Record<string, unknown>) => {
              harness.activateCalls.push({ spaceId, body, params });
              return { uuid: 'pub-1', spaceId, status: 'active', version: 'v8' };
            },
            pause: async (spaceId: string, body: Record<string, unknown>, params: Record<string, unknown>) => {
              harness.pauseCalls.push({ spaceId, body, params });
              return { uuid: 'pub-1', spaceId, status: 'paused', version: 'v9' };
            },
          },
          wikiSourceFiles: {
            list: async (spaceId: string, params: Record<string, unknown>) => {
              harness.listCalls.push({ spaceId, params });
              return {
                items: [
                  {
                    uuid: 'file-1',
                    sourcePath: 'index.md',
                    sourceState: 'ready',
                    publicationState: 'published',
                    visibility: 'public',
                    indexState: 'ready',
                    version: 'v3',
                  },
                ],
                pageInfo: { nextCursor: 'cursor-next', hasMore: true },
              };
            },
            publish: async (spaceId: string, uuid: string, body: Record<string, unknown>, params: Record<string, unknown>) => {
              harness.publishCalls.push({ spaceId, uuid, body, params });
              return { publication: { version: 'v8' }, sourceFile: { uuid, publicationState: 'published' } };
            },
            unpublish: async (spaceId: string, uuid: string, body: Record<string, unknown>, params: Record<string, unknown>) => {
              harness.unpublishCalls.push({ spaceId, uuid, body, params });
              return { publication: { version: 'v8' }, sourceFile: { uuid, publicationState: 'unpublished' } };
            },
            visibility: {
              update: async (spaceId: string, uuid: string, body: Record<string, unknown>, params: Record<string, unknown>) => {
                harness.visibilityCalls.push({ spaceId, uuid, body, params });
                return { publication: { version: 'v8' }, sourceFile: { uuid, visibility: body.visibility } };
              },
            },
          },
        },
      } as never,
      setTokenManager() {},
    },
  );
}

const wikiHarness = (): WikiClientHarness => ({
  activateCalls: [],
  pauseCalls: [],
  retrieveCalls: [],
  listCalls: [],
  publishCalls: [],
  unpublishCalls: [],
  visibilityCalls: [],
});

afterEach(() => {
  bindKnowledgebaseSessionStore(EMPTY_SESSION_STORE);
});

describe('wiki idempotency keys', () => {
  it('generates unique RFC-4122-shaped keys per call', () => {
    const first = newWikiIdempotencyKey();
    const second = newWikiIdempotencyKey();
    expect(first).not.toBe(second);
    expect(first).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  });
});

describe('wiki publication lifecycle commands', () => {
  it('retrieves the publication for the given space', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const publication = await retrieveWikiPublication('42');

    expect(harness.retrieveCalls).toEqual(['42']);
    expect(publication).toMatchObject({ uuid: 'pub-1', status: 'ready' });
  });

  it('sends expectedVersion and a fresh idempotency key on activate', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const publication = await activateWikiPublication('42', 'v7');

    expect(harness.activateCalls).toHaveLength(1);
    expect(harness.activateCalls[0]).toMatchObject({ spaceId: '42', body: { expectedVersion: 'v7' } });
    expect(String(harness.activateCalls[0].params.idempotencyKey)).toBeTruthy();
    expect(publication).toMatchObject({ status: 'active', version: 'v8' });
  });

  it('sends expectedVersion and a fresh idempotency key on pause', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const publication = await pauseWikiPublication('42', 'v8');

    expect(harness.pauseCalls).toHaveLength(1);
    expect(harness.pauseCalls[0]).toMatchObject({ spaceId: '42', body: { expectedVersion: 'v8' } });
    expect(String(harness.pauseCalls[0].params.idempotencyKey)).toBeTruthy();
    expect(publication).toMatchObject({ status: 'paused' });
  });

  it('uses distinct idempotency keys across repeated lifecycle commands', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    await activateWikiPublication('42', 'v7');
    await activateWikiPublication('42', 'v7');

    const keys = harness.activateCalls.map((call) => call.params.idempotencyKey);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe('wiki source file listing', () => {
  it('requests only pageSize on the first page', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const page = await listWikiSourceFiles('42', null, 20);

    expect(harness.listCalls).toHaveLength(1);
    expect(harness.listCalls[0]).toMatchObject({ spaceId: '42', params: { pageSize: 20 } });
    expect(harness.listCalls[0].params.cursor).toBeUndefined();
    expect(page.items).toHaveLength(1);
    expect(page.nextCursor).toBe('cursor-next');
    expect(page.hasMore).toBe(true);
  });

  it('forwards the opaque cursor for subsequent pages', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    await listWikiSourceFiles('42', 'cursor-abc', 20);

    expect(harness.listCalls[0].params).toEqual({ cursor: 'cursor-abc', pageSize: 20 });
  });
});

describe('wiki source file commands', () => {
  it('publishes with visibility and both expected versions', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const result = await publishWikiSourceFile('42', 'file-1', 'public', 'v8', 'v3');

    expect(harness.publishCalls).toHaveLength(1);
    expect(harness.publishCalls[0].body).toEqual({
      visibility: 'public',
      expectedPublicationVersion: 'v8',
      expectedPageVersion: 'v3',
    });
    expect(String(harness.publishCalls[0].params.idempotencyKey)).toBeTruthy();
    expect(result.publication).toMatchObject({ version: 'v8' });
    expect(result.sourceFile).toMatchObject({ uuid: 'file-1', publicationState: 'published' });
  });

  it('rejects private visibility on publish at the type level and sends unlisted otherwise', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    await publishWikiSourceFile('42', 'file-1', 'unlisted', 'v8', 'v3');

    expect(harness.publishCalls[0].body).toMatchObject({ visibility: 'unlisted' });
  });

  it('unpublishes with both expected versions', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const result = await unpublishWikiSourceFile('42', 'file-1', 'v8', 'v3');

    expect(harness.unpublishCalls).toHaveLength(1);
    expect(harness.unpublishCalls[0]).toMatchObject({
      spaceId: '42',
      uuid: 'file-1',
      body: { expectedPublicationVersion: 'v8', expectedPageVersion: 'v3' },
    });
    expect(result.sourceFile).toMatchObject({ publicationState: 'unpublished' });
  });

  it('changes visibility with both expected versions', async () => {
    const harness = wikiHarness();
    configureWikiClient(harness);

    const result = await changeWikiSourceFileVisibility('42', 'file-1', 'unlisted', 'v8', 'v3');

    expect(harness.visibilityCalls).toHaveLength(1);
    expect(harness.visibilityCalls[0].body).toEqual({
      visibility: 'unlisted',
      expectedPublicationVersion: 'v8',
      expectedPageVersion: 'v3',
    });
    expect(result.sourceFile).toMatchObject({ visibility: 'unlisted' });
  });
});

describe('wiki command gating sets mirror backend transitions', () => {
  it('allows activate only from ready and paused', () => {
    expect([...WIKI_ACTIVATE_ELIGIBLE_STATUSES].sort()).toEqual(['paused', 'ready']);
  });

  it('allows pause only from active and degraded', () => {
    expect([...WIKI_PAUSE_ELIGIBLE_STATUSES].sort()).toEqual(['active', 'degraded']);
  });

  it('allows page publish only while publication is ready, active, or paused', () => {
    expect([...WIKI_PUBLISH_ELIGIBLE_STATUSES].sort()).toEqual(['active', 'paused', 'ready']);
  });

  it('requires a ready source, a canonical route, and a non-error index for row publish', () => {
    expect(
      isWikiPublishRowEligible({
        sourceState: 'ready',
        canonicalRoute: '/guide',
        indexState: 'ready',
      } as never),
    ).toBe(true);
    expect(
      isWikiPublishRowEligible({
        sourceState: 'processing',
        canonicalRoute: '/guide',
        indexState: 'ready',
      } as never),
    ).toBe(false);
    expect(
      isWikiPublishRowEligible({
        sourceState: 'ready',
        canonicalRoute: null,
        indexState: 'ready',
      } as never),
    ).toBe(false);
    expect(
      isWikiPublishRowEligible({
        sourceState: 'ready',
        canonicalRoute: '/guide',
        indexState: 'error',
      } as never),
    ).toBe(false);
  });
});
