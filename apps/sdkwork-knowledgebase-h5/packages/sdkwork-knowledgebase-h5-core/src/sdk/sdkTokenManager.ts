import type { AuthTokenManager, AuthTokens } from "@sdkwork/sdk-common";

import type { KnowledgebaseH5Session, KnowledgebaseH5SessionStore } from "../session/session";

/**
 * Bind the single global TokenManager to the appbase IAM session store.
 *
 * Authenticated app-api SDK clients share one global token manager per
 * authenticated session (`APP_SDK_INTEGRATION_SPEC.md`; alignment spec
 * section 8). Logout, refresh failure, tenant switch, and account switch must
 * clear the token store and any sensitive session state.
 */
export function createKnowledgebaseSessionTokenManager(
  sessionStore: KnowledgebaseH5SessionStore,
): AuthTokenManager {
  return {
    clearTokens(): void {
      sessionStore.clearSession();
    },
    getTokens(): AuthTokens {
      const session: KnowledgebaseH5Session = sessionStore.getSnapshot();
      return {
        accessToken: session.accessToken,
        authToken: session.authToken,
        refreshToken: session.refreshToken,
      };
    },
    setTokens(tokens: AuthTokens): void {
      sessionStore.updateSession({
        accessToken: tokens.accessToken,
        authToken: tokens.authToken,
        refreshToken: tokens.refreshToken,
      });
    },
  };
}
