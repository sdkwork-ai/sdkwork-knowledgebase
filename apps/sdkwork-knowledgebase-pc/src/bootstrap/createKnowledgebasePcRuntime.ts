import {
  createHostAdapter,
  createKnowledgebaseAppSdkClient,
  createKnowledgebaseDriveAppSdkClient,
  createKnowledgebaseSessionTokenManager,
  createRuntimeConfig,
  createSessionStore,
  DEFAULT_SESSION_STORAGE_KEY,
  configureKnowledgebaseAppSdk,
  configureKnowledgebaseDriveAppSdk,
  bindKnowledgebaseSessionStore,
  setKnowledgebaseApiEnabled,
  isKnowledgebaseAppApiConfigured,
  invokeDesktopCommand,
  isTauriDesktopRuntime,
  type KnowledgebasePcRuntime,
  type SessionStorageLike,
} from 'sdkwork-knowledgebase-pc-core';
import {
  configureKnowledgebaseBackendSdk,
  createKnowledgebaseBackendSdkClient,
  isKnowledgebaseBackendApiConfigured,
  setKnowledgebaseBackendApiEnabled,
} from 'sdkwork-knowledgebase-pc-admin-core';

import { createKnowledgebaseIamRuntime } from './knowledgebaseIamRuntime';
import { primePcReactRuntimeSessionCache } from './sdkworkCorePcReactShim';

export function createKnowledgebasePcRuntime(): KnowledgebasePcRuntime {
  const config = createRuntimeConfig(import.meta.env);
  const resolvedStorage = resolveSessionStorage(config.auth.tokenStorage);
  const session = createSessionStore(resolvedStorage.storage);
  void resolvedStorage.hydrated?.then(() => {
    const current = session.getSnapshot();
    if (!current.authToken && !current.accessToken && !current.refreshToken) {
      session.refreshSession();
    }
  });
  const tokenManager = createKnowledgebaseSessionTokenManager(session);
  const appSdkClient = createKnowledgebaseAppSdkClient({
    config,
    tokenManager,
  });
  const backendSdkClient = createKnowledgebaseBackendSdkClient({
    config,
    tokenManager,
  });
  const driveSdkClient = createKnowledgebaseDriveAppSdkClient({
    config,
    tokenManager,
  });
  const iamRuntime = createKnowledgebaseIamRuntime({
    config,
    sdkClients: [appSdkClient, backendSdkClient, driveSdkClient],
    session,
    tokenManager,
  });

  primePcReactRuntimeSessionCache(session.getSnapshot());
  session.subscribe((snapshot) => {
    primePcReactRuntimeSessionCache(snapshot);
  });

  bindKnowledgebaseSessionStore(session);
  configureKnowledgebaseAppSdk(appSdkClient);
  configureKnowledgebaseBackendSdk(backendSdkClient);
  configureKnowledgebaseDriveAppSdk(driveSdkClient);
  setKnowledgebaseApiEnabled(
    config.auth.tokenManagerMode !== 'test'
    && isKnowledgebaseAppApiConfigured(config),
  );
  setKnowledgebaseBackendApiEnabled(
    config.auth.tokenManagerMode !== 'test'
    && isKnowledgebaseBackendApiConfigured(config),
  );

  return {
    config,
    auth: {
      iamRuntime,
    },
    sdk: {
      app: appSdkClient,
      drive: driveSdkClient,
    },
    session,
    host: createHostAdapter(),
  };
}

interface ResolvedSessionStorage {
  storage?: SessionStorageLike;
  hydrated?: Promise<void>;
}

// Hardened browsers / private modes can throw SecurityError on mere storage
// ACCESS; startup runs before any error boundary, so a throw here white-screens
// the whole app. Fall back to an in-memory no-op storage instead — auth simply
// behaves as "not logged in" for the tab.
function createMemoryFallbackStorage(): SessionStorageLike {
  const entries = new Map<string, string>();
  return {
    getItem: (key) => entries.get(key) ?? null,
    setItem: (key, value) => {
      entries.set(key, value);
    },
    removeItem: (key) => {
      entries.delete(key);
    },
  };
}

function safeWindowStorage(area: 'localStorage' | 'sessionStorage'): SessionStorageLike {
  try {
    return window[area];
  } catch (error) {
    console.error(`window.${area} is unavailable; falling back to in-memory storage`, error);
    return createMemoryFallbackStorage();
  }
}

function safeMigrateLegacyBrowserSession(): void {
  try {
    migrateLegacyBrowserSession();
  } catch (error) {
    console.error('legacy session migration failed; skipping', error);
  }
}

function resolveSessionStorage(
  tokenStorage: KnowledgebasePcRuntime['config']['auth']['tokenStorage'],
): ResolvedSessionStorage {
  if (typeof window === 'undefined') {
    return {};
  }
  if (tokenStorage === 'browser-local') {
    // Persistent browser login: keep the legacy sessionStorage migration path.
    safeMigrateLegacyBrowserSession();
    return { storage: safeWindowStorage('localStorage') };
  }
  if (tokenStorage === 'browser-session') {
    // Session-scoped login: tokens live in sessionStorage and expire with the
    // tab, shrinking the XSS-exposed credential window.
    return { storage: safeWindowStorage('sessionStorage') };
  }
  if (tokenStorage === 'os-secure-storage') {
    try {
      return createDesktopSecureSessionStorage() ?? {};
    } catch (error) {
      console.error('desktop secure storage init failed; auth starts signed out', error);
      return {};
    }
  }
  return {};
}

function migrateLegacyBrowserSession(): void {
  const legacySession = window.sessionStorage.getItem(DEFAULT_SESSION_STORAGE_KEY);
  if (legacySession && !window.localStorage.getItem(DEFAULT_SESSION_STORAGE_KEY)) {
    window.localStorage.setItem(DEFAULT_SESSION_STORAGE_KEY, legacySession);
  }
  if (legacySession) {
    window.sessionStorage.removeItem(DEFAULT_SESSION_STORAGE_KEY);
  }
}

function createDesktopSecureSessionStorage(): ResolvedSessionStorage | undefined {
  if (!isTauriDesktopRuntime()) {
    return undefined;
  }

  const memory = new Map<string, string>();
  let mutationVersion = 0;
  const hydrated = invokeDesktopCommand<string | null>('read_secure_session_value', {
    request: { key: DEFAULT_SESSION_STORAGE_KEY },
  })
    .then((value) => {
      if (value && mutationVersion === 0) {
        memory.set(DEFAULT_SESSION_STORAGE_KEY, value);
      }
    })
    .catch(() => undefined)
    .then(() => undefined);

  return {
    hydrated,
    storage: {
      getItem(key: string) {
        return memory.get(key) ?? null;
      },
      setItem(key: string, value: string) {
        mutationVersion += 1;
        memory.set(key, value);
        void invokeDesktopCommand('write_secure_session_value', { request: { key, value } }).catch(() => {
          memory.delete(key);
        });
      },
      removeItem(key: string) {
        mutationVersion += 1;
        memory.delete(key);
        void invokeDesktopCommand('remove_secure_session_value', { request: { key } }).catch(() => undefined);
      },
    },
  };
}
