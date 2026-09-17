import type { AuthTokenManager } from "@sdkwork/sdk-common";

export interface KnowledgebaseH5SessionTokens {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
}

export interface KnowledgebaseH5SessionUser {
  avatar?: string;
  displayName?: string;
  email?: string;
  id?: string;
  nickname?: string;
  username?: string;
}

export interface KnowledgebaseH5Session extends KnowledgebaseH5SessionTokens {
  expiresAt?: number;
  sessionId?: string;
  user?: KnowledgebaseH5SessionUser;
}

export interface KnowledgebaseH5RequestContext {
  appId: string;
  tenantId: string;
  userId: string;
}

export interface KnowledgebaseH5SessionStore {
  clearSession(): void;
  getSnapshot(): KnowledgebaseH5Session;
  subscribe(listener: (session: KnowledgebaseH5Session) => void): () => void;
  updateSession(patch: KnowledgebaseH5SessionTokens): void;
}

export const KNOWLEDGEBASE_H5_SESSION_KEY = "sdkwork-knowledgebase-h5:session:v1";
export const KNOWLEDGEBASE_H5_SESSION_CHANGED_EVENT = "sdkwork-knowledgebase-h5:auth-session-changed";

interface StorageLike {
  getItem(key: string): string | null;
  removeItem(key: string): void;
  setItem(key: string, value: string): void;
}

function getStorage(): StorageLike | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

/** Read the persisted session tokens, tolerating malformed stored payloads. */
export function readKnowledgebaseSessionTokens(): KnowledgebaseH5Session | null {
  const raw = getStorage()?.getItem(KNOWLEDGEBASE_H5_SESSION_KEY) ?? null;
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as KnowledgebaseH5Session;
    return typeof parsed === "object" && parsed !== null ? parsed : null;
  } catch {
    return null;
  }
}

export function isKnowledgebaseH5SessionAuthenticated(): boolean {
  const session = readKnowledgebaseSessionTokens();
  return Boolean(session?.accessToken || session?.authToken);
}

export function resolveKnowledgebaseAccessToken(session?: KnowledgebaseH5Session | null): string | undefined {
  return (session ?? readKnowledgebaseSessionTokens())?.accessToken || undefined;
}

export function resolveKnowledgebaseAuthToken(session?: KnowledgebaseH5Session | null): string | undefined {
  return (session ?? readKnowledgebaseSessionTokens())?.authToken || undefined;
}

/**
 * Build request-context interceptors from the live session so a token rotated
 * after client construction still propagates.
 */
export function createKnowledgebaseRequestContextInterceptors(
  readSession: () => KnowledgebaseH5Session | null,
) {
  return {
    onRequest(config: Record<string, unknown>): Record<string, unknown> {
      const session = readSession();
      const accessToken = resolveKnowledgebaseAccessToken(session);
      const authToken = resolveKnowledgebaseAuthToken(session);
      const headers: Record<string, string> = {
        ...((config.headers as Record<string, string> | undefined) ?? {}),
      };
      if (accessToken) {
        headers["Access-Token"] = accessToken;
      }
      if (authToken) {
        headers.Authorization = `Bearer ${authToken}`;
      }
      return { ...config, headers };
    },
  };
}

/** Create the in-memory session store used by bootstrap and the token manager. */
export function createKnowledgebaseH5SessionStore(): KnowledgebaseH5SessionStore {
  let snapshot: KnowledgebaseH5Session = readKnowledgebaseSessionTokens() ?? {};
  const listeners = new Set<(session: KnowledgebaseH5Session) => void>();

  const emit = () => {
    for (const listener of listeners) {
      listener(snapshot);
    }
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent(KNOWLEDGEBASE_H5_SESSION_CHANGED_EVENT, {
        detail: { session: snapshot },
      }));
    }
  };

  return {
    clearSession() {
      snapshot = {};
      const storage = getStorage();
      storage?.removeItem(KNOWLEDGEBASE_H5_SESSION_KEY);
      emit();
    },
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    updateSession(patch) {
      snapshot = { ...snapshot, ...patch };
      getStorage()?.setItem(KNOWLEDGEBASE_H5_SESSION_KEY, JSON.stringify(snapshot));
      emit();
    },
  };
}

export type { AuthTokenManager };
