import {
  KnowledgebaseErrorCodes,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';
import { createRuntimeConfig } from 'sdkwork-knowledgebase-pc-core/config/runtimeConfig';
import { DEFAULT_SESSION_STORAGE_KEY } from 'sdkwork-knowledgebase-pc-core/session/sessionStore';
import { trim } from '@sdkwork/utils';

export interface PcReactRuntimeSession {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
}

const SESSION_STORAGE_KEY = DEFAULT_SESSION_STORAGE_KEY;
let runtimeSessionCache: PcReactRuntimeSession = {};

// Hardened browsers / private modes throw SecurityError on mere storage
// ACCESS (this shim runs on the login/session-restore path, outside any error
// boundary). Degrade to an in-memory no-op storage — auth behaves as "not
// logged in" for the tab instead of throwing.
function safeWindowStorage(area: 'localStorage' | 'sessionStorage'): Storage | undefined {
  try {
    return window[area];
  } catch (error) {
    console.error(`window.${area} is unavailable; session persistence is disabled`, error);
    return undefined;
  }
}

function readStorage(): Storage | undefined {
  if (typeof window === 'undefined') {
    return undefined;
  }

  const tokenStorage = createRuntimeConfig(import.meta.env).auth.tokenStorage;
  if (tokenStorage === 'browser-local') {
    try {
      migrateLegacyBrowserSession();
    } catch (error) {
      console.error('legacy session migration failed; skipping', error);
    }
    return safeWindowStorage('localStorage');
  }
  if (tokenStorage === 'browser-session') {
    return safeWindowStorage('sessionStorage');
  }
  return undefined;
}

function migrateLegacyBrowserSession(): void {
  const legacySession = window.sessionStorage.getItem(SESSION_STORAGE_KEY);
  if (legacySession && !window.localStorage.getItem(SESSION_STORAGE_KEY)) {
    window.localStorage.setItem(SESSION_STORAGE_KEY, legacySession);
  }
  if (legacySession) {
    window.sessionStorage.removeItem(SESSION_STORAGE_KEY);
  }
}

function normalizeToken(value: unknown): string | undefined {
  const normalized = typeof value === 'string' ? trim(value) : '';
  return normalized.replace(/^Bearer\s+/i, '') || undefined;
}

function readStoredSession(): PcReactRuntimeSession {
  const tokenStorage = createRuntimeConfig(import.meta.env).auth.tokenStorage;
  if (tokenStorage === 'os-secure-storage') {
    return runtimeSessionCache;
  }

  const storage = readStorage();
  if (!storage) {
    return {};
  }

  try {
    const raw = storage.getItem(SESSION_STORAGE_KEY);
    if (!raw) {
      return {};
    }
    const parsed = JSON.parse(raw) as PcReactRuntimeSession;
    return {
      accessToken: normalizeToken(parsed.accessToken),
      authToken: normalizeToken(parsed.authToken),
      refreshToken: normalizeToken(parsed.refreshToken),
    };
  } catch {
    return {};
  }
}

export function readPcReactRuntimeSession(): PcReactRuntimeSession {
  return readStoredSession();
}

export function primePcReactRuntimeSessionCache(session: PcReactRuntimeSession): void {
  runtimeSessionCache = {
    accessToken: normalizeToken(session.accessToken),
    authToken: normalizeToken(session.authToken),
    refreshToken: normalizeToken(session.refreshToken),
  };
}

export function persistPcReactRuntimeSession(
  tokens: PcReactRuntimeSession,
): PcReactRuntimeSession {
  const current = readStoredSession();
  const next = {
    accessToken: tokens.accessToken !== undefined
      ? normalizeToken(tokens.accessToken)
      : current.accessToken,
    authToken: tokens.authToken !== undefined
      ? normalizeToken(tokens.authToken)
      : current.authToken,
    refreshToken: tokens.refreshToken !== undefined
      ? normalizeToken(tokens.refreshToken)
      : current.refreshToken,
  };
  runtimeSessionCache = next;
  const storage = readStorage();

  if (storage) {
    if (!next.accessToken && !next.authToken && !next.refreshToken) {
      storage.removeItem(SESSION_STORAGE_KEY);
    } else {
      storage.setItem(SESSION_STORAGE_KEY, JSON.stringify(next));
    }
  }

  return next;
}

export function clearPcReactRuntimeSession(): void {
  runtimeSessionCache = {};
  readStorage()?.removeItem(SESSION_STORAGE_KEY);
}

export function resolveAppClientAccessToken(): string {
  return normalizeToken(readStoredSession().accessToken) ?? '';
}

export function getAppClientWithSession(): never {
  throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
}
