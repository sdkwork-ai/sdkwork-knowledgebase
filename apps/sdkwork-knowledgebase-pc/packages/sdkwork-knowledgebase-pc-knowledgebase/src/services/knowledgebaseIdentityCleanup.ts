import {
  onKnowledgebaseIdentityChange,
  type KnowledgebaseIdentity,
  type KnowledgebaseIdentityChange,
} from 'sdkwork-knowledgebase-pc-core';

import { invalidateKnowledgeBrowserNodeCache } from './knowledgeBrowserListService';
import { forgetKnowledgeSpaceKnowledgeMode } from './knowledgeSpaceKnowledgeMode';
import { forgetSpaceAgentProfileInFlight } from './knowledgeSpaceSettingsService';
import { clearKnowledgebaseTabCacheForIdentity } from './tabService';

let disposePreviousRegistration: (() => void) | null = null;

/**
 * Apply one account change to every cache this package owns.
 *
 * The caches fall into two groups. Browser nodes, knowledge modes and in-flight
 * agent-profile provisionings describe a *space* and are keyed by space id only,
 * so a change of reader invalidates all of them. Persisted tab rows carry
 * document titles and belong to the account that opened them, so the previous
 * account's rows are removed.
 */
function handleKnowledgebaseIdentityChange(change: KnowledgebaseIdentityChange): void {
  invalidateKnowledgeBrowserNodeCache();
  forgetKnowledgeSpaceKnowledgeMode();
  forgetSpaceAgentProfileInFlight();
  clearKnowledgebaseTabCacheForIdentity(change.previous);
}

/**
 * Subscribe the knowledge-base caches to account changes.
 *
 * Installed by the runtime assembly right after the session store is bound, so
 * the very first identity the host publishes is already observed. Calling it
 * again replaces the previous registration instead of stacking one per runtime.
 *
 * @returns disposer that unsubscribes the registration installed here.
 */
export function installKnowledgebaseDerivedCacheCleanup(): () => void {
  disposePreviousRegistration?.();
  const dispose = onKnowledgebaseIdentityChange(handleKnowledgebaseIdentityChange);
  disposePreviousRegistration = dispose;
  return () => {
    dispose();
    if (disposePreviousRegistration === dispose) {
      disposePreviousRegistration = null;
    }
  };
}

/**
 * Remove the derived rows that belong to one account without waiting for a change
 * event (used by sign-out before the session store is cleared).
 *
 * @param identity - account whose rows are removed.
 */
export function clearKnowledgebaseDerivedCachesForIdentity(identity?: KnowledgebaseIdentity): void {
  invalidateKnowledgeBrowserNodeCache();
  forgetKnowledgeSpaceKnowledgeMode();
  forgetSpaceAgentProfileInFlight();
  clearKnowledgebaseTabCacheForIdentity(identity);
}
