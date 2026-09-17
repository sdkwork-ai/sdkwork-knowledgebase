import {
  createKnowledgebaseH5SessionStore,
  type KnowledgebaseH5Session,
} from "@sdkwork/knowledgebase-h5-core/session";

let sessionStore: ReturnType<typeof createKnowledgebaseH5SessionStore> | null = null;

/**
 * Create (or reuse) the appbase IAM session store for this browser runtime.
 *
 * Appbase IAM owns login, registration, current session, refresh, logout,
 * OAuth, QR auth, password reset, runtime metadata, current-user
 * self-service, and token propagation
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 8).
 */
export function createKnowledgebaseH5IamRuntime() {
  sessionStore ??= createKnowledgebaseH5SessionStore();
  return sessionStore;
}

export function getKnowledgebaseH5Session(): KnowledgebaseH5Session {
  return createKnowledgebaseH5IamRuntime().getSnapshot();
}
