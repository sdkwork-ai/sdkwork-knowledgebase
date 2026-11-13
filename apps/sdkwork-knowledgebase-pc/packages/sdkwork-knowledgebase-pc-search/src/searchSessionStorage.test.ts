import { afterEach, describe, expect, it } from 'vitest';
import {
  bindKnowledgebaseSessionStore,
  type SessionSnapshot,
  type SessionStore,
} from 'sdkwork-knowledgebase-pc-core';

import type { SearchSession } from './types';
import {
  currentSearchSessionsStorageKey,
  MAX_MESSAGES_PER_SESSION,
  MAX_SEARCH_SESSIONS,
  loadSearchSessions,
  persistSearchSessions,
  trimSearchSessions,
} from './searchSessionStorage';

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {},
    clearSession() {},
    subscribe: () => () => {},
  };
}

function bindAccount(userId: string): void {
  bindKnowledgebaseSessionStore(
    createStaticSessionStore({ context: { tenantId: 'tenant-1', userId } }),
  );
}

afterEach(() => {
  bindKnowledgebaseSessionStore(createStaticSessionStore({}));
  localStorage.clear();
});

function createSession(id: string, messageCount: number): SearchSession {
  return {
    id,
    title: id,
    createdAt: new Date().toISOString(),
    messages: Array.from({ length: messageCount }, (_, index) => ({
      id: `${id}-msg-${index}`,
      role: 'user' as const,
      content: `message-${index}`,
      timestamp: new Date().toISOString(),
    })),
    webSearchEnabled: true,
    deepThinkEnabled: false,
  };
}

describe('searchSessionStorage', () => {
  it('trims sessions and messages to configured limits', () => {
    const sessions = Array.from({ length: MAX_SEARCH_SESSIONS + 5 }, (_, index) =>
      createSession(`session-${index}`, MAX_MESSAGES_PER_SESSION + 10),
    );

    const trimmed = trimSearchSessions(sessions);
    expect(trimmed).toHaveLength(MAX_SEARCH_SESSIONS);
    expect(trimmed[0]?.messages).toHaveLength(MAX_MESSAGES_PER_SESSION);
    expect(trimmed[0]?.messages[0]?.content).toBe(`message-${MAX_MESSAGES_PER_SESSION + 10 - MAX_MESSAGES_PER_SESSION}`);
  });

  it('persists and reloads trimmed sessions from localStorage', () => {
    const storageKey = 'test-search-sessions';
    localStorage.setItem(storageKey, '[]');

    const sessions = [createSession('session-1', 2)];
    persistSearchSessions(storageKey, sessions);
    const loaded = loadSearchSessions(storageKey);

    expect(loaded).toHaveLength(1);
    expect(loaded?.[0]?.messages).toHaveLength(2);
  });

  it('scopes the signed-in account history and hides it from the next account', () => {
    bindAccount('user-a');
    const firstAccountKey = currentSearchSessionsStorageKey();
    expect(firstAccountKey).toBe('app-search-sessions.tenant-1.user-a');

    persistSearchSessions(firstAccountKey, [createSession('session-a', 1)]);

    expect(localStorage.getItem(firstAccountKey)).toContain('session-a');
    expect(localStorage.getItem('app-search-sessions')).toBeNull();

    bindAccount('user-b');
    expect(currentSearchSessionsStorageKey()).toBe('app-search-sessions.tenant-1.user-b');
    expect(loadSearchSessions(currentSearchSessionsStorageKey())).toBeNull();
  });

  it('falls back to the legacy base key while the identity is unknown', () => {
    expect(currentSearchSessionsStorageKey()).toBe('app-search-sessions');
  });
});
