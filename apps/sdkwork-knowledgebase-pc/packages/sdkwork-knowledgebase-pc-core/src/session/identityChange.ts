import {
  getKnowledgebaseActorId,
  getKnowledgebaseSessionStore,
  getKnowledgebaseTenantId,
} from '../api/knowledgebaseSpaceRegistry';
import type { KnowledgebaseIdentity } from './accountScopedStorage';
import type { SessionStore } from './sessionStore';

/** One observed change of the signed-in account. */
export interface KnowledgebaseIdentityChange {
  /** Account that owned every cache before the change; empty after sign-out. */
  previous: KnowledgebaseIdentity;
  /** Account that owns the caches after the change. */
  next: KnowledgebaseIdentity;
}

export type KnowledgebaseIdentityChangeListener = (change: KnowledgebaseIdentityChange) => void;

const listeners = new Set<KnowledgebaseIdentityChangeListener>();

let watchedStore: SessionStore | null = null;
let unwatch: (() => void) | null = null;
let currentIdentity: KnowledgebaseIdentity = {};

/**
 * Read the identity the bound session carries.
 *
 * Both fields are required for an account-scoped key, and the actor follows the
 * same `userId ?? actorId` rule as {@link getKnowledgebaseActorId}, so a partial
 * snapshot never produces a half-scoped key.
 */
function readBoundIdentity(): KnowledgebaseIdentity {
  const tenantId = getKnowledgebaseTenantId()?.trim();
  const userId = getKnowledgebaseActorId()?.trim();
  return {
    ...(tenantId ? { tenantId } : {}),
    ...(userId ? { userId } : {}),
  };
}

function sameIdentity(left: KnowledgebaseIdentity, right: KnowledgebaseIdentity): boolean {
  return left.tenantId === right.tenantId && left.userId === right.userId;
}

/**
 * Start watching the bound session store for account changes.
 *
 * The watch follows rebinding: a remounted runtime binds a fresh store, and the
 * subscription moves to it instead of stacking one per mount. Callers do not
 * need this function directly — {@link onKnowledgebaseIdentityChange} installs
 * it.
 */
export function installKnowledgebaseIdentityWatch(): void {
  const store = getKnowledgebaseSessionStore();
  if (store === null) {
    currentIdentity = {};
    unwatch?.();
    unwatch = null;
    watchedStore = null;
    return;
  }
  if (store === watchedStore && unwatch !== null) {
    return;
  }
  unwatch?.();
  watchedStore = store;
  currentIdentity = readBoundIdentity();
  unwatch = store.subscribe(() => {
    // A newer runtime may have rebound the store while this subscription was
    // still attached; that runtime's watch owns reporting from then on.
    if (getKnowledgebaseSessionStore() !== store) {
      return;
    }
    const next = readBoundIdentity();
    if (sameIdentity(next, currentIdentity)) {
      return;
    }
    const previous = currentIdentity;
    currentIdentity = next;
    for (const listener of [...listeners]) {
      listener({ previous, next });
    }
  });
}

/**
 * Subscribe to account changes (sign-out, sign-in, or one account replacing
 * another without a page reload).
 *
 * This is the hook every cache that is scoped by account identity uses to drop
 * its rows. It is deliberately independent of `clearSession()`: a host can swap
 * credentials without ever clearing the session store, and caches must still
 * stop serving the previous account.
 *
 * @param listener - called once per observed identity change.
 * @returns unsubscribe function.
 */
export function onKnowledgebaseIdentityChange(
  listener: KnowledgebaseIdentityChangeListener,
): () => void {
  listeners.add(listener);
  // Covers a listener registered before the runtime bound its store.
  installKnowledgebaseIdentityWatch();
  return () => {
    listeners.delete(listener);
  };
}

/** @returns the identity observed when the watch last started or changed. */
export function getKnowledgebaseBoundIdentity(): KnowledgebaseIdentity {
  return currentIdentity;
}
