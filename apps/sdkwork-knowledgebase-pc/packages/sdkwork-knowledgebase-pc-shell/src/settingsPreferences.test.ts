import { afterEach, describe, expect, it, vi } from 'vitest';
import { bindKnowledgebaseSessionStore } from 'sdkwork-knowledgebase-pc-core';

import {
  clearLocalPreferenceData,
  matchesSettingsToken,
  sanitizeSettingsTabId,
  sanitizeStartupModule,
} from './settingsPreferences';

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

afterEach(() => {
  bindKnowledgebaseSessionStore({
    getSnapshot: () => ({}),
    refreshSession: () => ({}),
    setSession() {},
    clearSession() {},
    subscribe: () => () => {},
  });
  vi.unstubAllGlobals();
});

describe('settingsPreferences', () => {
  it('matchesSettingsToken matches partial query and keyword', () => {
    expect(matchesSettingsToken('theme', 'dark theme')).toBe(true);
    expect(matchesSettingsToken('xyz', 'theme')).toBe(false);
  });

  it('sanitizeSettingsTabId falls back for invalid values', () => {
    expect(sanitizeSettingsTabId('appearance')).toBe('appearance');
    expect(sanitizeSettingsTabId('invalid', 'general')).toBe('general');
  });

  it('sanitizeStartupModule accepts kb and market only', () => {
    expect(sanitizeStartupModule('market')).toBe('market');
    expect(sanitizeStartupModule('kb')).toBe('kb');
    expect(sanitizeStartupModule('other', 'kb')).toBe('kb');
  });

  it('clearLocalPreferenceData removes the scoped search history it actually wrote', () => {
    const localStorage = createFakeStorage();
    vi.stubGlobal('window', { localStorage, sessionStorage: createFakeStorage() });
    bindKnowledgebaseSessionStore({
      getSnapshot: () => ({ context: { tenantId: 'tenant-1', userId: 'user-a' } }),
      refreshSession: () => ({ context: { tenantId: 'tenant-1', userId: 'user-a' } }),
      setSession() {},
      clearSession() {},
      subscribe: () => () => {},
    });
    localStorage.setItem('app-search-sessions.tenant-1.user-a', '[{"id":"session-1"}]');
    localStorage.setItem('app-search-sessions', '[{"id":"legacy"}]');
    localStorage.setItem('app-settings-tab', '"general"');

    clearLocalPreferenceData();

    expect(localStorage.getItem('app-search-sessions.tenant-1.user-a')).toBeNull();
    expect(localStorage.getItem('app-search-sessions')).toBeNull();
    expect(localStorage.getItem('app-settings-tab')).toBeNull();
  });
});
