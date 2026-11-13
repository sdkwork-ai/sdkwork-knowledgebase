import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  bindKnowledgebaseSessionStore,
  readRegisteredSpaces,
  removeRegisteredSpace,
  updateRegisteredSpace,
  upsertRegisteredSpace,
  type RegisteredKnowledgebaseSpace,
} from './knowledgebaseSpaceRegistry';
import type { SessionSnapshot, SessionStore } from '../session/sessionStore';

const TENANT_ID = 'tenant-1';
const REGISTRY_KEY = `sdkwork.knowledgebase.spaces.v1.${TENANT_ID}`;

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {},
    clearSession() {},
    subscribe: () => () => {},
  };
}

function bindActor(userId: string): void {
  bindKnowledgebaseSessionStore(
    createStaticSessionStore({ context: { tenantId: TENANT_ID, userId } }),
  );
}

const recorded: { messages: unknown[] } = { messages: [] };

class RecordingBroadcastChannel {
  constructor() {}

  postMessage(message: unknown): void {
    recorded.messages.push(message);
  }

  close(): void {}
}

function createFakeLocalStorage(): {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  store: Map<string, string>;
} {
  const store = new Map<string, string>();
  return {
    store,
    getItem: (key) => store.get(key) ?? null,
    setItem: (key, value) => {
      store.set(key, value);
    },
    removeItem: (key) => {
      store.delete(key);
    },
  };
}

function fakeStorage(): Map<string, string> {
  const storage = (globalThis as {
    window?: { localStorage?: { store?: Map<string, string> } };
  }).window?.localStorage;
  if (!storage?.store) {
    throw new Error('fake localStorage was not installed');
  }
  return storage.store;
}

function seedSpaces(spaces: RegisteredKnowledgebaseSpace[]): void {
  fakeStorage().set(REGISTRY_KEY, JSON.stringify(spaces));
}

function readStoredSpaces(): RegisteredKnowledgebaseSpace[] {
  const raw = fakeStorage().get(REGISTRY_KEY);
  return raw ? (JSON.parse(raw) as RegisteredKnowledgebaseSpace[]) : [];
}

describe('knowledgebaseSpaceRegistry cross-tab updates', () => {
  beforeEach(() => {
    recorded.messages = [];
    vi.stubGlobal('BroadcastChannel', RecordingBroadcastChannel);
    vi.stubGlobal('window', { localStorage: createFakeLocalStorage() });
  });

  afterEach(() => {
    bindKnowledgebaseSessionStore(createStaticSessionStore({}));
    vi.unstubAllGlobals();
  });

  it('upserts against the stored snapshot and replaces the matching space', () => {
    seedSpaces([
      { spaceId: '11', kbType: 'team', createdAt: '2026-01-01T00:00:00.000Z' },
      { spaceId: '22', kbType: 'personal', createdAt: '2026-01-02T00:00:00.000Z' },
    ]);

    const next = upsertRegisteredSpace(TENANT_ID, {
      spaceId: '22',
      kbType: 'personal',
      icon: '📗',
      createdAt: '2026-02-01T00:00:00.000Z',
    });

    expect(next.map((space) => space.spaceId)).toEqual(['11', '22']);
    expect(readStoredSpaces()).toHaveLength(2);
    expect(readStoredSpaces()[1]?.icon).toBe('📗');
    expect(recorded.messages).toEqual([REGISTRY_KEY]);
  });

  it('removes only the targeted space', () => {
    seedSpaces([
      { spaceId: '11', kbType: 'team', createdAt: '2026-01-01T00:00:00.000Z' },
      { spaceId: '22', kbType: 'personal', createdAt: '2026-01-02T00:00:00.000Z' },
    ]);

    const next = removeRegisteredSpace(TENANT_ID, '11');

    expect(next.map((space) => space.spaceId)).toEqual(['22']);
    expect(readRegisteredSpaces(TENANT_ID).map((space) => space.spaceId)).toEqual(['22']);
  });

  it('patches only the targeted space without dropping siblings', () => {
    seedSpaces([
      { spaceId: '11', kbType: 'team', createdAt: '2026-01-01T00:00:00.000Z' },
      { spaceId: '22', kbType: 'personal', createdAt: '2026-01-02T00:00:00.000Z' },
    ]);

    const next = updateRegisteredSpace(TENANT_ID, '22', { icon: '📘', kbType: 'public' });

    expect(next.find((space) => space.spaceId === '22')?.kbType).toBe('public');
    expect(next.find((space) => space.spaceId === '11')?.kbType).toBe('team');
    expect(readStoredSpaces().find((space) => space.spaceId === '22')?.icon).toBe('📘');
  });

  it('keeps each account of one tenant on its own registry', () => {
    bindActor('user-a');
    upsertRegisteredSpace(TENANT_ID, {
      spaceId: '11',
      kbType: 'personal',
      createdAt: '2026-01-01T00:00:00.000Z',
    });

    expect(readRegisteredSpaces(TENANT_ID).map((space) => space.spaceId)).toEqual(['11']);

    bindActor('user-b');
    expect(readRegisteredSpaces(TENANT_ID)).toEqual([]);
    upsertRegisteredSpace(TENANT_ID, {
      spaceId: '22',
      kbType: 'personal',
      createdAt: '2026-01-02T00:00:00.000Z',
    });

    bindActor('user-a');
    expect(readRegisteredSpaces(TENANT_ID).map((space) => space.spaceId)).toEqual(['11']);
  });

  it('adopts the tenant-only registry once so a pre-upgrade account keeps its spaces', () => {
    seedSpaces([
      { spaceId: '11', kbType: 'personal', createdAt: '2026-01-01T00:00:00.000Z' },
    ]);
    bindActor('user-a');

    expect(readRegisteredSpaces(TENANT_ID).map((space) => space.spaceId)).toEqual(['11']);
    expect(
      JSON.parse(fakeStorage().get(`${REGISTRY_KEY}.user-a`) ?? 'null'),
    ).toHaveLength(1);

    // An account that already has its own registry never reads the adopted copy again.
    removeRegisteredSpace(TENANT_ID, '11');
    expect(readRegisteredSpaces(TENANT_ID)).toEqual([]);
    expect(JSON.parse(fakeStorage().get(`${REGISTRY_KEY}.user-a`) ?? 'null')).toEqual([]);
  });
});
