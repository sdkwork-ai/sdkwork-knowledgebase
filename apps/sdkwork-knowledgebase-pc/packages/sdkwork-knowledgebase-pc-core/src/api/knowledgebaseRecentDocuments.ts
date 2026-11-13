import { getKnowledgebaseActorId } from './knowledgebaseSpaceRegistry';
import { withCrossTabStorageUpdate } from './knowledgebaseCrossTabStorage';

const RECENT_DOCS_KEY_PREFIX = 'sdkwork.knowledgebase.recent.v1';
const MAX_RECENT_DOCUMENTS = 32;

export interface RecentDocumentEntry {
  id: string;
  title: string;
  type: 'richtext' | 'code' | 'markdown' | 'file' | 'image' | 'audio' | 'video' | 'folder' | 'pdf' | 'music';
  kbId?: string;
  updatedAt: string;
  author?: string;
}

/**
 * Recent documents are account-scoped for the same reason the space registry is:
 * a shared browser profile would otherwise offer the previous account's
 * documents, and opening one answers a permission error for content the reader
 * never owned.
 */
function recentStorageKey(tenantId: string): string {
  const actorId = getKnowledgebaseActorId();
  return actorId === undefined
    ? `${RECENT_DOCS_KEY_PREFIX}.${tenantId}`
    : `${RECENT_DOCS_KEY_PREFIX}.${tenantId}.${actorId}`;
}

export function readRecentDocuments(tenantId: string): RecentDocumentEntry[] {
  if (typeof window === 'undefined') {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(recentStorageKey(tenantId));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw) as RecentDocumentEntry[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function touchRecentDocument(
  tenantId: string,
  entry: RecentDocumentEntry,
): RecentDocumentEntry[] {
  if (typeof window === 'undefined') {
    return [];
  }

  return withCrossTabStorageUpdate(
    recentStorageKey(tenantId),
    () => readRecentDocuments(tenantId),
    (current) => {
      const next = current.filter((item) => item.id !== entry.id);
      next.unshift({
        ...entry,
        updatedAt: entry.updatedAt || new Date().toISOString(),
      });
      return next.slice(0, MAX_RECENT_DOCUMENTS);
    },
    (next) => window.localStorage.setItem(recentStorageKey(tenantId), JSON.stringify(next)),
  );
}

export function removeRecentDocument(tenantId: string, documentId: string): void {
  if (typeof window === 'undefined') {
    return;
  }

  withCrossTabStorageUpdate(
    recentStorageKey(tenantId),
    () => readRecentDocuments(tenantId),
    (current) => current.filter((item) => item.id !== documentId),
    (next) => window.localStorage.setItem(recentStorageKey(tenantId), JSON.stringify(next)),
  );
}
