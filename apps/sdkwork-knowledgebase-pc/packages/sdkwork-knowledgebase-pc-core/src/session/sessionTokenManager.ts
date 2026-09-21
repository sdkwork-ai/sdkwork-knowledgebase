import { readBootstrapAccessTokenFromProcessEnv } from '@sdkwork/iam-credential-entry';
import { isBlank } from '@sdkwork/utils';

import type { SessionSnapshot, SessionStore } from './sessionStore';
export interface KnowledgebaseSessionAuthTokens {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
}

export interface KnowledgebaseSessionTokenManager {
  clearAccessToken(): void;
  clearAuthToken(): void;
  clearTokens(): void;
  getAccessToken(): string | undefined;
  getAuthToken(): string | undefined;
  getRefreshToken(): string | undefined;
  getTokens(): KnowledgebaseSessionAuthTokens;
  hasAccessToken(): boolean;
  hasAuthToken(): boolean;
  hasToken(): boolean;
  isExpired(): boolean;
  isValid(): boolean;
  setAccessToken(token: string): void;
  setAuthToken(token: string): void;
  setRefreshToken(token: string): void;
  setTokens(tokens: KnowledgebaseSessionAuthTokens): void;
  willExpireIn(seconds: number): boolean;
}

export function createKnowledgebaseSessionTokenManager(
  session: SessionStore,
): KnowledgebaseSessionTokenManager {
  const getAccessTokenExpiryEpochSeconds = (): number | undefined => {
    const accessToken = session.getSnapshot().accessToken;
    if (!accessToken) {
      return undefined;
    }
    return decodeJwtExpiryEpochSeconds(accessToken);
  };

  return {
    clearAccessToken() {
      deleteSessionFields(session, ['accessToken']);
    },
    clearAuthToken() {
      deleteSessionFields(session, ['authToken']);
    },
    clearTokens() {
      session.clearSession();
    },
    getAccessToken() {
      // Fall back to the private bootstrap Access-Token artifact when no
      // interactive session exists (APP_SDK_INTEGRATION_SPEC section 4).
      return normalizeDualTokenValue(session.getSnapshot().accessToken)
        ?? normalizeDualTokenValue(readBootstrapAccessTokenFromProcessEnv());
    },
    getAuthToken() {
      return normalizeDualTokenValue(session.getSnapshot().authToken);
    },
    getRefreshToken() {
      return session.getSnapshot().refreshToken;
    },
    getTokens() {
      const snapshot = session.getSnapshot();
      return {
        accessToken: snapshot.accessToken,
        authToken: snapshot.authToken,
        refreshToken: snapshot.refreshToken,
      };
    },
    hasAccessToken() {
      return Boolean(session.getSnapshot().accessToken);
    },
    hasAuthToken() {
      return Boolean(session.getSnapshot().authToken);
    },
    hasToken() {
      const snapshot = session.getSnapshot();
      return Boolean(snapshot.authToken || snapshot.accessToken);
    },
    isExpired() {
      const expiryEpochSeconds = getAccessTokenExpiryEpochSeconds();
      if (expiryEpochSeconds === undefined) {
        return false;
      }
      return Date.now() >= expiryEpochSeconds * 1000;
    },
    isValid() {
      const snapshot = session.getSnapshot();
      return Boolean(snapshot.authToken && snapshot.accessToken);
    },
    setAccessToken(token) {
      mergeSession(session, { accessToken: normalizeDualTokenValue(token) });
    },
    setAuthToken(token) {
      mergeSession(session, { authToken: normalizeDualTokenValue(token) });
    },
    setRefreshToken(token) {
      mergeSession(session, { refreshToken: token });
    },
    setTokens(tokens) {
      replaceSession(session, {
        ...session.getSnapshot(),
        accessToken: normalizeDualTokenValue(tokens.accessToken),
        authToken: normalizeDualTokenValue(tokens.authToken),
        refreshToken: tokens.refreshToken,
      });
    },
    willExpireIn(_seconds) {
      const expiryEpochSeconds = getAccessTokenExpiryEpochSeconds();
      if (expiryEpochSeconds === undefined) {
        return false;
      }
      return Date.now() + _seconds * 1000 >= expiryEpochSeconds * 1000;
    },
  };
}

function decodeJwtExpiryEpochSeconds(token: string): number | undefined {
  const segments = token.split('.');
  if (segments.length < 2) {
    return undefined;
  }
  const payload = decodeBase64Url(segments[1]);
  if (!payload) {
    return undefined;
  }
  try {
    const parsed = JSON.parse(payload) as Record<string, unknown>;
    const exp = parsed.exp;
    if (typeof exp === 'number' && Number.isFinite(exp)) {
      return exp;
    }
    if (typeof exp === 'string' && !isBlank(exp) && Number.isFinite(Number(exp))) {
      return Number(exp);
    }
  } catch {
    return undefined;
  }
  return undefined;
}

function decodeBase64Url(value: string): string | undefined {
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/');
  const remainder = normalized.length % 4;
  const padded = normalized + (remainder === 0 ? '' : '='.repeat(4 - remainder));
  if (typeof atob === 'function') {
    try {
      return atob(padded);
    } catch {
      return undefined;
    }
  }
  return undefined;
}

function mergeSession(session: SessionStore, patch: Partial<SessionSnapshot>): void {
  replaceSession(session, {
    ...session.getSnapshot(),
    ...compactSessionPatch(patch),
  });
}

function replaceSession(session: SessionStore, nextSession: SessionSnapshot): void {
  const compact = compactSessionPatch(nextSession) as SessionSnapshot;
  if (!compact.authToken && !compact.accessToken && !compact.refreshToken) {
    session.clearSession();
    return;
  }

  session.setSession(compact);
}

function deleteSessionFields(
  session: SessionStore,
  keys: Array<keyof SessionSnapshot>,
): void {
  replaceSession(session, omitSessionKeys(session.getSnapshot(), keys));
}

function omitSessionKeys(
  snapshot: SessionSnapshot,
  keys: Array<keyof SessionSnapshot>,
): SessionSnapshot {
  const next = { ...snapshot };
  for (const key of keys) {
    delete next[key];
  }
  return next;
}

function compactSessionPatch<T extends object>(value: T): Partial<T> {
  return Object.fromEntries(
    Object.entries(value).filter(([, entry]) => entry !== undefined),
  ) as Partial<T>;
}

function normalizeDualTokenValue(token: string | undefined): string | undefined {
  if (isBlank(token)) {
    return undefined;
  }

  const trimmed = (token ?? '').trim();
  const withoutBearer = trimmed.replace(/^Bearer\s+/i, '').trim();
  return withoutBearer || undefined;
}
