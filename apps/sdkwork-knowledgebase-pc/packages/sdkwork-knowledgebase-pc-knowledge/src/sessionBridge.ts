import type { SessionStore } from 'sdkwork-knowledgebase-pc-core';

import { getKnowledgebasePcSdkPorts } from './sdkPorts';

export function syncHostSessionIntoKnowledgebaseStore(session: SessionStore): void {
  const hostSession = getKnowledgebasePcSdkPorts().readHostSession();
  if (hostSession) {
    session.setSession(hostSession);
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
