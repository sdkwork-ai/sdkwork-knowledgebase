import { getKnowledgebaseActorId, getKnowledgebaseTenantId } from '../api/knowledgebaseSpaceRegistry';

/**
 * Account-scoped browser storage keys.
 *
 * A browser profile outlives a sign-out and one tab outlives an account switch
 * inside the app, so a key that carries one account's derived data replays it to
 * the next account: open tabs leak document titles, the selected knowledge base
 * points at a space the reader cannot open, and WeChat drafts address the
 * previous account's recipients. Those keys are therefore namespaced as
 * `<base>.<tenantId>.<userId>`.
 *
 * Without a bound identity the helpers fall back to the bare base key: the
 * pre-scoping layout stays readable where no tenant/user is ever materialized
 * (and during the first paint before the session bridge runs), and logout always
 * removes the bare key together with the scoped one, so a legacy row cannot
 * outlive the account that wrote it.
 */
export interface KnowledgebaseIdentity {
  tenantId?: string;
  userId?: string;
}

/**
 * Every persisted base key whose value belongs to one account.
 *
 * The registry lives here rather than next to each module so that logout can
 * remove all of them from one place without importing the packages that own
 * them (`accountSession` cannot import the UI packages).
 */
export const KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS = {
  /** Open document tabs per knowledge base (`TabCacheService`). */
  tabCache: 'app-tabs-cache-v2',
  /** AI search sessions and their answers. */
  searchSessions: 'app-search-sessions',
  /** Last selected knowledge base. */
  activeKnowledgeBase: 'app-active-kb',
  /** One-shot search → knowledge-base navigation intent. */
  navIntent: 'app-kb-nav-intent',
  /** WeChat preview recipients. */
  wechatPreviewHistory: 'wechat_preview_history',
  /** WeChat publishing author history. */
  wechatAuthorHistory: 'wechat_author_history',
  /** Writing agent chosen for the notes assistant. */
  writingAgent: 'sdkwork.kb.writing-agent.v1',
} as const;

export type KnowledgebaseAccountScopedStorageKey =
  (typeof KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS)[keyof typeof KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS];

/** Identity of the currently bound session, when it carries one. */
export function currentKnowledgebaseIdentity(): KnowledgebaseIdentity {
  const tenantId = getKnowledgebaseTenantId()?.trim();
  const userId = getKnowledgebaseActorId()?.trim();
  return {
    ...(tenantId ? { tenantId } : {}),
    ...(userId ? { userId } : {}),
  };
}

/**
 * Build the storage key for one account.
 *
 * @param baseKey - unscoped key from {@link KNOWLEDGEBASE_ACCOUNT_SCOPED_STORAGE_KEYS}.
 * @param identity - account to address; defaults to the bound session.
 * @returns `<base>.<tenantId>.<userId>`, or the bare base key when the identity
 *   is incomplete (legacy layout).
 */
export function accountScopedStorageKey(
  baseKey: string,
  identity: KnowledgebaseIdentity = currentKnowledgebaseIdentity(),
): string {
  const tenantId = identity.tenantId?.trim();
  const userId = identity.userId?.trim();
  if (!tenantId || !userId) {
    return baseKey;
  }
  return `${baseKey}.${tenantId}.${userId}`;
}

function storageAreas(): Storage[] {
  if (typeof window === 'undefined') {
    return [];
  }
  const areas: Storage[] = [];
  for (const area of ['localStorage', 'sessionStorage'] as const) {
    try {
      const storage = window[area];
      if (storage) {
        areas.push(storage);
      }
    } catch {
      // Hardened browsers throw on mere storage access; cleanup is best-effort.
    }
  }
  return areas;
}

/**
 * Remove one account's value plus the legacy unscoped row.
 *
 * Both storage areas are touched because the same base key is written to
 * `localStorage` by most modules and to `sessionStorage` by the navigation
 * intent; a stale row in either area is equally unreadable to the next account.
 *
 * @param baseKey - unscoped key to remove.
 * @param identity - account whose scoped row is removed; defaults to the bound
 *   session, which is the account signing out.
 */
export function clearAccountScopedStorageKey(
  baseKey: string,
  identity: KnowledgebaseIdentity = currentKnowledgebaseIdentity(),
): void {
  const scopedKey = accountScopedStorageKey(baseKey, identity);
  for (const storage of storageAreas()) {
    for (const key of scopedKey === baseKey ? [baseKey] : [scopedKey, baseKey]) {
      try {
        storage.removeItem(key);
      } catch {
        // Best-effort cleanup: a failed removal must not break sign-out.
      }
    }
  }
}

/**
 * Remove several base keys for one account.
 *
 * @param baseKeys - unscoped keys to remove.
 * @param identity - account whose rows are removed; defaults to the bound session.
 */
export function clearAccountScopedStorageKeys(
  baseKeys: readonly string[],
  identity: KnowledgebaseIdentity = currentKnowledgebaseIdentity(),
): void {
  for (const baseKey of baseKeys) {
    clearAccountScopedStorageKey(baseKey, identity);
  }
}
