import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  getKnowledgebaseBoundIdentity,
  installKnowledgebaseIdentityWatch,
  onKnowledgebaseIdentityChange,
  type KnowledgebaseIdentityChange,
} from './identityChange';
import { bindKnowledgebaseSessionStore } from '../api/knowledgebaseSpaceRegistry';
import type { SessionSnapshot, SessionStore } from './sessionStore';

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

afterEach(() => {
  bindKnowledgebaseSessionStore(createMutableSessionStore({}));
  vi.unstubAllGlobals();
});

describe('onKnowledgebaseIdentityChange', () => {
  it('reports the previous and next identity when the account changes', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    installKnowledgebaseIdentityWatch();

    const changes: KnowledgebaseIdentityChange[] = [];
    onKnowledgebaseIdentityChange((change) => changes.push(change));

    store.update(signedIn('tenant-1', 'user-b'));

    expect(changes).toEqual([
      {
        previous: { tenantId: 'tenant-1', userId: 'user-a' },
        next: { tenantId: 'tenant-1', userId: 'user-b' },
      },
    ]);
  });

  it('ignores re-emissions of the same identity', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    installKnowledgebaseIdentityWatch();

    const listener = vi.fn();
    onKnowledgebaseIdentityChange(listener);

    store.update({ context: { tenantId: 'tenant-1', userId: 'user-a' }, accessToken: 'refreshed' });

    expect(listener).not.toHaveBeenCalled();
  });

  it('reports sign-out with an empty next identity', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    installKnowledgebaseIdentityWatch();

    const changes: KnowledgebaseIdentityChange[] = [];
    onKnowledgebaseIdentityChange((change) => changes.push(change));

    store.clearSession();

    expect(changes).toEqual([
      { previous: { tenantId: 'tenant-1', userId: 'user-a' }, next: {} },
    ]);
  });

  it('moves the watch to a rebound store instead of stacking subscriptions', () => {
    const first = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(first);
    installKnowledgebaseIdentityWatch();

    const listener = vi.fn();
    onKnowledgebaseIdentityChange(listener);

    const second = createMutableSessionStore(signedIn('tenant-2', 'user-c'));
    bindKnowledgebaseSessionStore(second);
    installKnowledgebaseIdentityWatch();
    expect(getKnowledgebaseBoundIdentity()).toEqual({ tenantId: 'tenant-2', userId: 'user-c' });

    first.update(signedIn('tenant-1', 'user-b'));
    expect(listener).not.toHaveBeenCalled();

    second.update(signedIn('tenant-2', 'user-d'));
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener.mock.calls[0]?.[0]).toMatchObject({
      previous: { tenantId: 'tenant-2', userId: 'user-c' },
      next: { tenantId: 'tenant-2', userId: 'user-d' },
    });
  });

  it('stops notifying after unsubscribe', () => {
    const store = createMutableSessionStore(signedIn('tenant-1', 'user-a'));
    bindKnowledgebaseSessionStore(store);
    installKnowledgebaseIdentityWatch();

    const listener = vi.fn();
    const unsubscribe = onKnowledgebaseIdentityChange(listener);
    unsubscribe();

    store.update(signedIn('tenant-1', 'user-b'));

    expect(listener).not.toHaveBeenCalled();
  });
});
