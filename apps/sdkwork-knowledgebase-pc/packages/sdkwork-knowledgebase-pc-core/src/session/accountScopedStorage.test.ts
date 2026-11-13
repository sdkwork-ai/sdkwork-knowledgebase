import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  accountScopedStorageKey,
  clearAccountScopedStorageKey,
  clearAccountScopedStorageKeys,
  currentKnowledgebaseIdentity,
  KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS,
} from './accountScopedStorage';
import { bindKnowledgebaseSessionStore } from '../api/knowledgebaseSpaceRegistry';
import type { SessionSnapshot, SessionStore } from './sessionStore';

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {},
    clearSession() {},
    subscribe: () => () => {},
  };
}

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

let localStorageFake: FakeStorage;
let sessionStorageFake: FakeStorage;

function installWindow(): void {
  localStorageFake = createFakeStorage();
  sessionStorageFake = createFakeStorage();
  vi.stubGlobal('window', {
    localStorage: localStorageFake,
    sessionStorage: sessionStorageFake,
  });
}

afterEach(() => {
  bindKnowledgebaseSessionStore(createStaticSessionStore({}));
  vi.unstubAllGlobals();
});

describe('accountScopedStorageKey', () => {
  it('namespaces the base key with the bound tenant and account', () => {
    bindKnowledgebaseSessionStore(
      createStaticSessionStore({ context: { tenantId: 'tenant-1', userId: 'user-9' } }),
    );

    expect(accountScopedStorageKey('app-active-kb')).toBe('app-active-kb.tenant-1.user-9');
    expect(currentKnowledgebaseIdentity()).toEqual({ tenantId: 'tenant-1', userId: 'user-9' });
  });

  it('falls back to the bare base key while the identity is incomplete', () => {
    bindKnowledgebaseSessionStore(
      createStaticSessionStore({ context: { tenantId: 'tenant-1', userId: '' } }),
    );
    expect(accountScopedStorageKey('app-active-kb')).toBe('app-active-kb');

    bindKnowledgebaseSessionStore(createStaticSessionStore({}));
    expect(accountScopedStorageKey('app-active-kb')).toBe('app-active-kb');
  });

  it('uses the actor id when the snapshot only carries one', () => {
    bindKnowledgebaseSessionStore(
      createStaticSessionStore({
        context: { tenantId: 'tenant-1', actorId: 'user-7' },
      } as SessionSnapshot),
    );

    expect(currentKnowledgebaseIdentity()).toEqual({ tenantId: 'tenant-1', userId: 'user-7' });
    expect(accountScopedStorageKey('app-tabs-cache-v2')).toBe('app-tabs-cache-v2.tenant-1.user-7');
  });

  it('receives an explicit identity and ignores surrounding blanks', () => {
    expect(accountScopedStorageKey('app-x', { tenantId: ' t1 ', userId: 'u1' })).toBe('app-x.t1.u1');
    expect(accountScopedStorageKey('app-x', { tenantId: 't1', userId: '   ' })).toBe('app-x');
  });
});

describe('clearAccountScopedStorageKey', () => {
  it('removes the account row and the legacy row from both storage areas only', () => {
    installWindow();
    for (const storage of [localStorageFake, sessionStorageFake]) {
      storage.setItem('app-kb-nav-intent.tenant-1.user-9', 'mine');
      storage.setItem('app-kb-nav-intent.tenant-1.user-8', 'other-account');
      storage.setItem('app-kb-nav-intent', 'legacy');
    }

    clearAccountScopedStorageKey(
      KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS.navIntent,
      { tenantId: 'tenant-1', userId: 'user-9' },
    );

    for (const storage of [localStorageFake, sessionStorageFake]) {
      expect(storage.getItem('app-kb-nav-intent.tenant-1.user-9')).toBeNull();
      expect(storage.getItem('app-kb-nav-intent')).toBeNull();
      expect(storage.getItem('app-kb-nav-intent.tenant-1.user-8')).toBe('other-account');
    }
  });

  it('clears several base keys for one account', () => {
    installWindow();
    localStorageFake.setItem('app-active-kb.tenant-1.user-9', '{}');
    localStorageFake.setItem('wechat_preview_history.tenant-1.user-9', '[]');

    clearAccountScopedStorageKeys(
      [
        KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS.activeKnowledgeBase,
        KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS.wechatPreviewHistory,
      ],
      { tenantId: 'tenant-1', userId: 'user-9' },
    );

    expect(localStorageFake.store.size).toBe(0);
  });

  it('survives a missing window', () => {
    vi.stubGlobal('window', undefined);
    expect(() =>
      clearAccountScopedStorageKey('app-x', { tenantId: 't', userId: 'u' }),
    ).not.toThrow();
  });
});
