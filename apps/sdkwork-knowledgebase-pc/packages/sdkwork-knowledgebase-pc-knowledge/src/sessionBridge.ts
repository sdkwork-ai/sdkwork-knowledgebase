import type { SessionStore } from 'sdkwork-knowledgebase-pc-core';
import { withKnowledgebaseTokenDerivedContext } from 'sdkwork-knowledgebase-pc-core';

import { getKnowledgebasePcSdkPorts } from './sdkPorts';

/**
 * Sync the host session into the knowledgebase session store.
 * The host forwards credentials only (never tenant ids); the identity context
 * is derived here from the tokens' SDKWork JWT claims so every tenant-scoped
 * guard issues real requests without an explicit context handoff.
 */
export function syncHostSessionIntoKnowledgebaseStore(session: SessionStore): void {
  const hostSession = getKnowledgebasePcSdkPorts().readHostSession();
  if (hostSession) {
    session.setSession(withKnowledgebaseTokenDerivedContext(hostSession));
    return;
  }
  session.clearSession();
}

export function bindHostSessionToKnowledgebaseStore(session: SessionStore): () => void {
  syncHostSessionIntoKnowledgebaseStore(session);
  const subscribe = getKnowledgebasePcSdkPorts().subscribeHostSession;
  if (!subscribe) {
    return () => undefined;
  }
  return subscribe(() => {
    syncHostSessionIntoKnowledgebaseStore(session);
  });
}
