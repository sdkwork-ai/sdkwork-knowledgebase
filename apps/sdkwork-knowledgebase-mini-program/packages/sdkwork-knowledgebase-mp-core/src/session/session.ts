export interface KnowledgebaseMpSessionTokens {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
}

export interface KnowledgebaseMpSession extends KnowledgebaseMpSessionTokens {
  expiresAt?: number;
  sessionId?: string;
}

export interface KnowledgebaseMpSessionStore {
  clearSession(): void;
  getSnapshot(): KnowledgebaseMpSession;
  subscribe(listener: (session: KnowledgebaseMpSession) => void): () => void;
  updateSession(patch: KnowledgebaseMpSessionTokens): void;
}

export const KNOWLEDGEBASE_MP_SESSION_STORAGE_KEY = "sdkwork-knowledgebase-mp:session:v1";

interface StorageLike {
  getItem(key: string): string | null;
  removeItem(key: string): void;
  setItem(key: string, value: string): void;
}

/**
 * Mini program storage access.
 *
 * The platform storage API is reached through the host adapter boundary in
 * production; this guarded lookup keeps the session store testable off-platform
 * and never falls back to raw network calls.
 */
function getStorage(): StorageLike | null {
  const globalObject = globalThis as { wx?: { getStorageSync?: unknown } };
  if (!globalObject.wx?.getStorageSync) {
    return null;
  }
  const wx = globalObject.wx as unknown as {
    getStorageSync(key: string): unknown;
    removeStorageSync(key: string): void;
    setStorageSync(key: string, value: unknown): void;
  };
  return {
    getItem: (key) => {
      const value = wx.getStorageSync(key);
      return typeof value === "string" && value ? value : null;
    },
    removeItem: (key) => wx.removeStorageSync(key),
    setItem: (key, value) => wx.setStorageSync(key, value),
  };
}

export function readKnowledgebaseMpSessionTokens(): KnowledgebaseMpSession | null {
  const raw = getStorage()?.getItem(KNOWLEDGEBASE_MP_SESSION_STORAGE_KEY) ?? null;
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as KnowledgebaseMpSession;
    return typeof parsed === "object" && parsed !== null ? parsed : null;
  } catch {
    return null;
  }
}

export function isKnowledgebaseMpSessionAuthenticated(): boolean {
  const session = readKnowledgebaseMpSessionTokens();
  return Boolean(session?.accessToken || session?.authToken);
}

export function resolveKnowledgebaseMpAccessToken(session?: KnowledgebaseMpSession | null): string | undefined {
  return (session ?? readKnowledgebaseMpSessionTokens())?.accessToken || undefined;
}

export function resolveKnowledgebaseMpAuthToken(session?: KnowledgebaseMpSession | null): string | undefined {
  return (session ?? readKnowledgebaseMpSessionTokens())?.authToken || undefined;
}

export function createKnowledgebaseMpSessionStore(): KnowledgebaseMpSessionStore {
  let snapshot: KnowledgebaseMpSession = readKnowledgebaseMpSessionTokens() ?? {};
  const listeners = new Set<(session: KnowledgebaseMpSession) => void>();
  return {
    clearSession() {
      snapshot = {};
      getStorage()?.removeItem(KNOWLEDGEBASE_MP_SESSION_STORAGE_KEY);
      for (const listener of listeners) {
        listener(snapshot);
      }
    },
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    updateSession(patch) {
      snapshot = { ...snapshot, ...patch };
      getStorage()?.setItem(KNOWLEDGEBASE_MP_SESSION_STORAGE_KEY, JSON.stringify(snapshot));
      for (const listener of listeners) {
        listener(snapshot);
      }
    },
  };
}
