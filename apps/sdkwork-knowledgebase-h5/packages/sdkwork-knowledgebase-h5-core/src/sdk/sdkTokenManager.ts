import { readBootstrapAccessTokenFromProcessEnv } from "@sdkwork/iam-credential-entry";
import type { AuthTokenManager, AuthTokens } from "@sdkwork/sdk-common";

import type { KnowledgebaseH5Session, KnowledgebaseH5SessionStore } from "../session/session";

/**
 * Bind the single global TokenManager to the appbase IAM session store.
 *
 * Authenticated app-api SDK clients share one global token manager per
 * authenticated session (`APP_SDK_INTEGRATION_SPEC.md`; alignment spec
 * section 8). Logout, refresh failure, tenant switch, and account switch must
 * clear the token store and any sensitive session state.
 *
 * `getAccessToken()` falls back to the private bootstrap Access-Token artifact
 * (`APP_SDK_INTEGRATION_SPEC.md` section 4): the generated SDK transports read
 * `Access-Token` exclusively from `getAccessToken()` and fail before dispatch
 * when it is empty, so without this fallback every protected surface is
 * unusable before the first login.
 */
export function createKnowledgebaseSessionTokenManager(
  sessionStore: KnowledgebaseH5SessionStore,
): AuthTokenManager {
  return {
    clearTokens(): void {
      sessionStore.clearSession();
    },
    getAccessToken(): string | undefined {
      return sessionStore.getSnapshot().accessToken
        ?? readBootstrapAccessTokenFromProcessEnv();
    },
    getAuthToken(): string | undefined {
      return sessionStore.getSnapshot().authToken;
    },
    getRefreshToken(): string | undefined {
      return sessionStore.getSnapshot().refreshToken;
    },
    getTokens(): AuthTokens {
      const session: KnowledgebaseH5Session = sessionStore.getSnapshot();
      return {
        accessToken: session.accessToken ?? readBootstrapAccessTokenFromProcessEnv(),
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
