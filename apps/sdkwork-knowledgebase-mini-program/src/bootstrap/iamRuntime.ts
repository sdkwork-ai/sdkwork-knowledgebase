import {
  createKnowledgebaseMpSessionStore,
  type KnowledgebaseMpSession,
} from "@sdkwork/knowledgebase-mp-core/session";

let sessionStore: ReturnType<typeof createKnowledgebaseMpSessionStore> | null = null;

/**
 * Appbase IAM owns login, session, refresh, logout, and token propagation.
 * The mini program runtime reuses the shared session store from core.
 */
export function createKnowledgebaseMpIamRuntime() {
  sessionStore ??= createKnowledgebaseMpSessionStore();
  return sessionStore;
}

export function getKnowledgebaseMpSession(): KnowledgebaseMpSession {
  return createKnowledgebaseMpIamRuntime().getSnapshot();
}
