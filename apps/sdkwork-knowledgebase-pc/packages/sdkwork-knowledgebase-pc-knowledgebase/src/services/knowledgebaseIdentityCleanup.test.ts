import { afterEach, describe, expect, it, vi } from 'vitest';
import { bindKnowledgebaseSessionStore } from 'sdkwork-knowledgebase-pc-core';

import type { SessionSnapshot, SessionStore } from 'sdkwork-knowledgebase-pc-core';

const mocks = vi.hoisted(() => ({
  invalidateKnowledgeBrowserNodeCache: vi.fn(),
  forgetKnowledgeSpaceKnowledgeMode: vi.fn(),
  forgetSpaceAgentProfileInFlight: vi.fn(),
  clearKnowledgebaseTabCacheForIdentity: vi.fn(),
}));

vi.mock('./knowledgeBrowserListService', () => ({
  invalidateKnowledgeBrowserNodeCache: mocks.invalidateKnowledgeBrowserNodeCache,
}));

vi.mock('./knowledgeSpaceKnowledgeMode', () => ({
  forgetKnowledgeSpaceKnowledgeMode: mocks.forgetKnowledgeSpaceKnowledgeMode,
}));

vi.mock('./knowledgeSpaceSettingsService', () => ({
  forgetSpaceAgentProfileInFlight: mocks.forgetSpaceAgentProfileInFlight,
}));

vi.mock('./tabService', () => ({
  clearKnowledgebaseTabCacheForIdentity: mocks.clearKnowledgebaseTabCacheForIdentity,
}));

const {
  clearKnowledgebaseDerivedCachesForIdentity,
  installKnowledgebaseDerivedCacheCleanup,
} = await import('./knowledgebaseIdentityCleanup');

interface MutableSessionStore extends SessionStore {
  update(snapshot: SessionSnapshot): void;
}

function createMutableSessionStore(initial: SessionSnapshot): MutableSessionStore {
  let snapshot = initial;
  const listeners = new Set<(next: SessionSnapshot) => void>();
  const store: MutableSessionStore = {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession(next) {
      snapshot = next;
      for (const listener of listeners) listener(snapshot);
    },
    clearSession() {
      snapshot = {};
      for (const listener of listeners) listener(snapshot);
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    update(next) {
      store.setSession(next);
    },
  };
  return store;
}

function signedIn(tenantId: string, userId: string): SessionSnapshot {
  return { context: { tenantId, userId } };
}

function resetMocks(): void {
  mocks.invalidateKnowledgeBrowserNodeCache.mockReset();
  mocks.forgetKnowledgeSpaceKnowledgeMode.mockReset();
  mocks.forgetSpaceAgentProfileInFlight.mockReset();
  mocks.clearKnowledgebaseTabCacheForIdentity.mockReset();
}

afterEach(() => {
  bindKnowledgebaseSessionStore(createMutableSessionStore({}));
  resetMocks();
});

describe('knowledgebase derived cache cleanup', () => {
  it('invalidates every derived cache when the account changes', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    const dispose = installKnowledgebaseDerivedCacheCleanup();

    store.update(signedIn('tenant-1', 'user-b'));

    expect(mocks.invalidateKnowledgeBrowserNodeCache).toHaveBeenCalledTimes(1);
    expect(mocks.forgetKnowledgeSpaceKnowledgeMode).toHaveBeenCalledTimes(1);
    expect(mocks.forgetSpaceAgentProfileInFlight).toHaveBeenCalledTimes(1);
    expect(mocks.clearKnowledgebaseTabCacheForIdentity).toHaveBeenCalledWith({
      tenantId: 'tenant-1',
      userId: 'user-a',
    });
    dispose();
  });

  it('does not stack registrations across runtime rebuilds', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    installKnowledgebaseDerivedCacheCleanup();
    installKnowledgebaseDerivedCacheCleanup();

    store.update(signedIn('tenant-1', 'user-c'));

    expect(mocks.invalidateKnowledgeBrowserNodeCache).toHaveBeenCalledTimes(1);
    expect(mocks.clearKnowledgebaseTabCacheForIdentity).toHaveBeenCalledTimes(1);
  });

  it('stops listening after the disposer runs', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    const dispose = installKnowledgebaseDerivedCacheCleanup();
    dispose();

    store.update(signedIn('tenant-1', 'user-b'));

    expect(mocks.invalidateKnowledgeBrowserNodeCache).not.toHaveBeenCalled();
  });

  it('clears the caches for an explicit identity without waiting for an event', () => {
    clearKnowledgebaseDerivedCachesForIdentity({ tenantId: 'tenant-1', userId: 'user-z' });

    expect(mocks.invalidateKnowledgeBrowserNodeCache).toHaveBeenCalledTimes(1);
    expect(mocks.forgetKnowledgeSpaceKnowledgeMode).toHaveBeenCalledTimes(1);
    expect(mocks.forgetSpaceAgentProfileInFlight).toHaveBeenCalledTimes(1);
    expect(mocks.clearKnowledgebaseTabCacheForIdentity).toHaveBeenCalledWith({
      tenantId: 'tenant-1',
      userId: 'user-z',
    });
  });
});
