import { useEffect } from 'react';

import type { KnowledgebasePcRuntime } from '../runtime/KnowledgebaseRuntimeProvider.js';
import { clearLocalDocumentContentForTenant } from '../api/knowledgebaseDocumentContentCache.js';
import {
  KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS,
  clearAccountScopedStorageKeys,
  type KnowledgebaseIdentity,
} from './accountScopedStorage.js';
import { DEFAULT_SESSION_STORAGE_KEY } from './sessionStore.js';
import type { SessionStore } from './sessionStore.js';

type KnowledgebaseIamRuntimeLike = {
  service?: {
    iam?: {
      users?: {
        current?: {
          retrieve?: () => Promise<unknown>;
        };
      };
    };
    auth?: {
      sessions?: {
        current?: {
          retrieve?: () => Promise<unknown>;
          delete?: () => Promise<unknown>;
        };
      };
    };
  };
  contextStore?: {
    clear?: () => Promise<void> | void;
  };
};

/**
 * Preferences that describe the app session rather than the account.
 *
 * They are removed on sign-out because the next sign-in picks its own landing
 * slot; none of them carries account data.
 */
const RUNTIME_STATE_STORAGE_KEYS = [
  DEFAULT_SESSION_STORAGE_KEY,
  'app-active-tab',
  'app-search-view-active',
] as const;

/** Identity carried by the snapshot that is about to be discarded. */
function readSigningOutIdentity(session: SessionStore): KnowledgebaseIdentity {
  const context = session.getSnapshot().context;
  const tenantId = context?.tenantId?.trim();
  const userId = (context?.userId ?? context?.actorId)?.trim();
  return {
    ...(tenantId ? { tenantId } : {}),
    ...(userId ? { userId } : {}),
  };
}

/**
 * Remove every local trace of the signed-out account.
 *
 * The identity is read before `clearSession()` because it clears the snapshot the
 * account-scoped keys are addressed with. Document bodies and the open-tab,
 * search, selection, WeChat and writing-agent rows all belong to that account;
 * leaving them behind is what let the next account on a shared profile render the
 * previous account's titles and recipients.
 *
 * @param session - store of the account signing out.
 */
export function signOutKnowledgebaseAccount(session: SessionStore): void {
  const identity = readSigningOutIdentity(session);
  if (identity.tenantId) {
    // `clearSession` already covers the bound snapshot's tenant; this covers a
    // caller whose store context was cleared before the cleanup ran.
    clearLocalDocumentContentForTenant(identity.tenantId);
  }
  session.clearSession();
  clearPersistedKnowledgebaseRuntimeState(identity);
}

function clearPersistedKnowledgebaseRuntimeState(identity: KnowledgebaseIdentity): void {
  if (typeof window === 'undefined') {
    return;
  }

  const storages: Array<Storage | undefined> = [window.localStorage, window.sessionStorage];
  for (const storage of storages) {
    if (!storage) {
      continue;
    }
    for (const key of RUNTIME_STATE_STORAGE_KEYS) {
      try {
        storage.removeItem(key);
      } catch {
        // Ignore storage cleanup errors to keep logout resilient.
      }
    }
  }
  clearAccountScopedStorageKeys(
    Object.values(KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS),
    identity,
  );
}

export function useHydrateKnowledgebaseAccount(runtime: KnowledgebasePcRuntime): void {
  useEffect(() => {
    const snapshot = runtime.session.getSnapshot();
    if (!snapshot.accessToken && !snapshot.authToken) {
      return;
    }

    const iamRuntime = runtime.auth?.iamRuntime as KnowledgebaseIamRuntimeLike | undefined;
    if (!iamRuntime) {
      return;
    }

    let cancelled = false;

    void (async () => {
      try {
        if (!runtime.session.getSnapshot().user?.displayName) {
          await iamRuntime.service?.iam?.users?.current?.retrieve?.();
        }
      } catch {
        // Best-effort profile hydration; auth gate handles incomplete sessions.
      }

      if (cancelled || runtime.session.getSnapshot().context?.tenantId) {
        return;
      }

      try {
        await iamRuntime.service?.auth?.sessions?.current?.retrieve?.();
      } catch {
        // Best-effort tenant hydration after login.
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [runtime]);
}

/**
 * Sign out: end the server-side session when the runtime can, and always remove
 * the local account state.
 *
 * The local cleanup runs in `finally` because it is the only step that depends on
 * nothing remote. Skipping it when the server accepted the delete left the
 * account's tokens in device storage and its tab/search/WeChat rows readable by
 * whoever signed in next.
 *
 * @param runtime - runtime owning the session store and the IAM bridge.
 */
export async function signOutKnowledgebaseSession(
  runtime: KnowledgebasePcRuntime,
): Promise<void> {
  const iamRuntime = runtime.auth?.iamRuntime as KnowledgebaseIamRuntimeLike | undefined;

  try {
    await iamRuntime?.contextStore?.clear?.();
  } catch {
    // Continue with local session cleanup.
  }

  try {
    const deleteCurrentSession = iamRuntime?.service?.auth?.sessions?.current?.delete;
    if (typeof deleteCurrentSession === 'function') {
      await deleteCurrentSession.call(iamRuntime?.service?.auth?.sessions?.current);
    }
  } catch {
    // The server session may already be gone; local cleanup still runs.
  } finally {
    signOutKnowledgebaseAccount(runtime.session);
  }
}
