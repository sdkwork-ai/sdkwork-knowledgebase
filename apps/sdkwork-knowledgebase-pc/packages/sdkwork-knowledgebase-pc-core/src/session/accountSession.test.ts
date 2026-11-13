import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  signOutKnowledgebaseAccount,
  signOutKnowledgebaseSession,
} from './accountSession';
import type { SessionSnapshot, SessionStore } from './sessionStore';

interface FakeStorage extends Storage {
  store: Map<string, string>;
}

function createFakeStorage(): FakeStorage {
  const store = new Map<string, string>();
  return {
    store,
    get length() {
      return store.size;
    },
    clear: () => {
      store.clear();
    },
    getItem: (key: string) => store.get(key) ?? null,
    key: (index: number) => [...store.keys()][index] ?? null,
    removeItem: (key: string) => {
      store.delete(key);
    },
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  };
}

function createSessionStore(snapshot: SessionSnapshot): { store: SessionStore; cleared: () => boolean } {
  let cleared = false;
  const store: SessionStore = {
    getSnapshot: () => (cleared ? {} : snapshot),
    refreshSession: () => snapshot,
    setSession() {},
    clearSession() {
      cleared = true;
    },
    subscribe: () => () => {},
  };
  return { store, cleared: () => cleared };
}

let localStorageFake: FakeStorage;
let sessionStorageFake: FakeStorage;

const SIGNED_IN: SessionSnapshot = { context: { tenantId: 'tenant-1', userId: 'user-9' } };

const ACCOUNT_KEYS = [
  'app-tabs-cache-v2.tenant-1.user-9',
  'app-search-sessions.tenant-1.user-9',
  'app-active-kb.tenant-1.user-9',
  'wechat_preview_history.tenant-1.user-9',
  'wechat_author_history.tenant-1.user-9',
  'sdkwork.kb.writing-agent.v1.tenant-1.user-9',
];

beforeEach(() => {
  localStorageFake = createFakeStorage();
  sessionStorageFake = createFakeStorage();
  vi.stubGlobal('window', {
    localStorage: localStorageFake,
    sessionStorage: sessionStorageFake,
  });
  for (const key of ACCOUNT_KEYS) {
    localStorageFake.setItem(key, '{}');
  }
  localStorageFake.setItem('app-active-tab', '"kb"');
  localStorageFake.setItem('sdkwork-knowledgebase-pc-session', '{"accessToken":"a"}');
  localStorageFake.setItem('app-active-kb', '{"id":"42"}');
  sessionStorageFake.setItem('app-kb-nav-intent.tenant-1.user-9', '{}');
  sessionStorageFake.setItem('app-search-view-active', '1');
  sessionStorageFake.setItem(
    'sdkwork.knowledgebase.content.v2.tenant-1.user-9.77',
    '{"content":"body"}',
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('signOutKnowledgebaseAccount', () => {
  it('removes the account rows, the legacy rows and this tenant document bodies', () => {
    const { store, cleared } = createSessionStore(SIGNED_IN);

    signOutKnowledgebaseAccount(store);

    expect(cleared()).toBe(true);
    for (const key of ACCOUNT_KEYS) {
      expect(localStorageFake.getItem(key)).toBeNull();
    }
    expect(localStorageFake.getItem('app-active-kb')).toBeNull();
    expect(localStorageFake.getItem('sdkwork-knowledgebase-pc-session')).toBeNull();
    expect(localStorageFake.getItem('app-active-tab')).toBeNull();
    expect(sessionStorageFake.getItem('app-search-view-active')).toBeNull();
    expect(sessionStorageFake.getItem('app-kb-nav-intent.tenant-1.user-9')).toBeNull();
    expect(
      sessionStorageFake.getItem('sdkwork.knowledgebase.content.v2.tenant-1.user-9.77'),
    ).toBeNull();
  });

  it('leaves another account rows in place', () => {
    localStorageFake.setItem('app-tabs-cache-v2.tenant-1.user-8', '{"kb":{"docs":[]}}');

    signOutKnowledgebaseAccount(createSessionStore(SIGNED_IN).store);

    expect(localStorageFake.getItem('app-tabs-cache-v2.tenant-1.user-8')).not.toBeNull();
  });
});

function runtimeWithSessionDelete(deleteSession?: () => Promise<unknown>) {
  const { store, cleared } = createSessionStore(SIGNED_IN);
  return {
    runtime: {
      session: store,
      auth: {
        iamRuntime: {
          contextStore: { clear: () => Promise.resolve() },
          service: {
            auth: {
              sessions: {
                current: {
                  ...(deleteSession ? { delete: deleteSession } : {}),
                },
              },
            },
          },
        },
      },
    } as never,
    sessionCleared: () => cleared(),
  };
}

describe('signOutKnowledgebaseSession', () => {
  it('cleans up locally even when the server session delete succeeds', async () => {
    const { runtime, sessionCleared } = runtimeWithSessionDelete(async () => undefined);

    await signOutKnowledgebaseSession(runtime);

    expect(sessionCleared()).toBe(true);
    expect(localStorageFake.getItem('app-tabs-cache-v2.tenant-1.user-9')).toBeNull();
    expect(localStorageFake.getItem('sdkwork-knowledgebase-pc-session')).toBeNull();
  });

  it('cleans up locally when the server session delete fails', async () => {
    const { runtime, sessionCleared } = runtimeWithSessionDelete(async () => {
      throw new Error('gateway down');
    });

    await expect(signOutKnowledgebaseSession(runtime)).resolves.toBeUndefined();

    expect(sessionCleared()).toBe(true);
    expect(localStorageFake.getItem('app-active-kb.tenant-1.user-9')).toBeNull();
  });

  it('cleans up locally when the runtime has no IAM bridge', async () => {
    const { store, cleared } = createSessionStore(SIGNED_IN);

    await signOutKnowledgebaseSession({ session: store, auth: {} } as never);

    expect(cleared()).toBe(true);
    expect(localStorageFake.getItem('app-search-sessions.tenant-1.user-9')).toBeNull();
  });
});
