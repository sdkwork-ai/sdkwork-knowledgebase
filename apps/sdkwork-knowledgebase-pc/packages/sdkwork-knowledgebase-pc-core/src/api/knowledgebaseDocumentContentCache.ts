import { getKnowledgebaseActorId } from './knowledgebaseSpaceRegistry';

const CONTENT_CACHE_KEY_PREFIX = 'sdkwork.knowledgebase.content.v2';
const MAX_CACHED_CONTENT_BYTES = 512 * 1024;

interface CachedDocumentContent {
  content: string;
  contentVersion: string;
  cachedAt: string;
}

/**
 * Document bodies are cached per account as well as per tenant.
 *
 * A shared browser profile or Electron window can switch accounts without
 * reloading the page, and the cache is read before an authoritative fallback in
 * some paths; keying it by tenant alone let the next account read a body the
 * previous account had cached.
 */
function contentStorageKey(tenantId: string, documentId: string): string {
  const actorId = getKnowledgebaseActorId();
  return actorId === undefined
    ? `${CONTENT_CACHE_KEY_PREFIX}.${tenantId}.${documentId}`
    : `${CONTENT_CACHE_KEY_PREFIX}.${tenantId}.${actorId}.${documentId}`;
}

function getSessionStorage(): Storage | null {
  if (typeof window === 'undefined') {
    return null;
  }
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

function parseCachedDocumentContent(raw: string): CachedDocumentContent | undefined {
  try {
    const parsed = JSON.parse(raw) as Partial<CachedDocumentContent>;
    if (
      typeof parsed.content === 'string'
      && typeof parsed.contentVersion === 'string'
      && parsed.contentVersion.length > 0
    ) {
      return {
        content: parsed.content,
        contentVersion: parsed.contentVersion,
        cachedAt: typeof parsed.cachedAt === 'string' ? parsed.cachedAt : new Date().toISOString(),
      };
    }
  } catch {
    // Fall through to legacy plain-text cache entries.
  }

  if (raw.length > 0) {
    return {
      content: raw,
      contentVersion: 'legacy',
      cachedAt: new Date().toISOString(),
    };
  }

  return undefined;
}

export function readLocalDocumentContent(
  tenantId: string,
  documentId: string,
  expectedVersion?: string,
): string | undefined {
  const storage = getSessionStorage();
  if (!storage) {
    return undefined;
  }

  try {
    const raw = storage.getItem(contentStorageKey(tenantId, documentId));
    if (!raw) {
      return undefined;
    }
    const cached = parseCachedDocumentContent(raw);
    if (!cached) {
      return undefined;
    }
    if (expectedVersion && cached.contentVersion !== expectedVersion) {
      return undefined;
    }
    return cached.content;
  } catch {
    return undefined;
  }
}

export function writeLocalDocumentContent(
  tenantId: string,
  documentId: string,
  content: string,
  contentVersion?: string,
): void {
  const storage = getSessionStorage();
  if (!storage) {
    return;
  }

  if (content.length > MAX_CACHED_CONTENT_BYTES) {
    return;
  }

  const payload: CachedDocumentContent = {
    content,
    contentVersion: contentVersion ?? new Date().toISOString(),
    cachedAt: new Date().toISOString(),
  };
  try {
    storage.setItem(contentStorageKey(tenantId, documentId), JSON.stringify(payload));
  } catch {
    // Storage quota exhausted: evict the oldest cached entries for the tenant and retry
    // once. A failed cache write must never surface as a document save/load error.
    evictOldestTenantEntries(storage, tenantId, 8);
    try {
      storage.setItem(contentStorageKey(tenantId, documentId), JSON.stringify(payload));
    } catch {
      // Best-effort cache only; the caller's persistence path must not fail on quota.
    }
  }
}

function evictOldestTenantEntries(storage: Storage, tenantId: string, maxEntries: number): void {
  const prefix = `${CONTENT_CACHE_KEY_PREFIX}.${tenantId}.`;
  const entries: { key: string; cachedAt: number }[] = [];
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (!key?.startsWith(prefix)) {
      continue;
    }
    const raw = storage.getItem(key);
    let cachedAt = 0;
    if (raw) {
      try {
        const parsed = JSON.parse(raw) as Partial<CachedDocumentContent>;
        cachedAt = Date.parse(parsed.cachedAt ?? '') || 0;
      } catch {
        cachedAt = 0;
      }
    }
    entries.push({ key, cachedAt });
  }
  entries.sort((left, right) => left.cachedAt - right.cachedAt);
  for (const entry of entries.slice(0, maxEntries)) {
    storage.removeItem(entry.key);
  }
}

export function removeLocalDocumentContent(tenantId: string, documentId: string): void {
  const storage = getSessionStorage();
  if (!storage) {
    return;
  }

  storage.removeItem(contentStorageKey(tenantId, documentId));
}

export function clearLocalDocumentContentForTenant(tenantId: string): void {
  const storage = getSessionStorage();
  if (!storage) {
    return;
  }

  const prefix = `${CONTENT_CACHE_KEY_PREFIX}.${tenantId}.`;
  const keysToRemove: string[] = [];
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (key?.startsWith(prefix)) {
      keysToRemove.push(key);
    }
  }
  keysToRemove.forEach((key) => storage.removeItem(key));
}
