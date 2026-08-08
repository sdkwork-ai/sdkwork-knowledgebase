import type { KnowledgeBrowserNode, KnowledgeBrowserView } from 'sdkwork-knowledgebase-pc-core';
import {
  isKnowledgebaseAppError,
  parseKnowledgeSpaceId,
  requireKnowledgebaseAppSdkHttpClient,
} from 'sdkwork-knowledgebase-pc-core';
import { normalizeSdkWorkListPage } from './sdkWorkListPage';

const BROWSER_NODE_CACHE_TTL_MS = 30_000;
const BROWSER_CACHE_MAX_ENTRIES = 256;
const DEFAULT_BROWSER_PAGE_SIZE = 100;
/**
 * Upper bound for node-resolution scans: a single space is scanned at most
 * this many pages per resolution so a pathological space cannot trigger an
 * unbounded request storm (PAGINATION_SPEC §8 on-demand paging).
 */
export const MAX_BROWSER_RESOLVE_SCAN_PAGES = 25;
const browserParentCache = new Map<string, BrowserNodeCacheEntry>();

interface BrowserNodeCacheEntry {
  fetchedAt: number;
  view: KnowledgeBrowserView;
  parentId: string | null;
  nodes: KnowledgeBrowserNode[];
  nextCursor: string | null;
  hasMore: boolean;
}

const DEFAULT_BROWSER_VIEW: KnowledgeBrowserView = 'files';

function parentCacheKey(
  spaceId: string,
  view: KnowledgeBrowserView,
  parentId: string | null,
): string {
  return `${spaceId}:${view}:${parentId ?? '__root__'}`;
}

export function getLoadedKnowledgeBrowserNodes(
  spaceId: string,
  options?: { view?: KnowledgeBrowserView },
): KnowledgeBrowserNode[] {
  const prefix = `${spaceId}:`;
  const view = options?.view ?? DEFAULT_BROWSER_VIEW;
  const merged = new Map<string, KnowledgeBrowserNode>();
  for (const [key, entry] of browserParentCache.entries()) {
    if (!key.startsWith(prefix)) {
      continue;
    }
    if (entry.view !== view) {
      continue;
    }
    for (const node of entry.nodes) {
      merged.set(node.id, node);
    }
  }
  return Array.from(merged.values());
}

function requireSdkClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function purgeExpiredBrowserCacheEntries(now = Date.now()): void {
  for (const [key, entry] of browserParentCache.entries()) {
    if (now - entry.fetchedAt >= BROWSER_NODE_CACHE_TTL_MS) {
      browserParentCache.delete(key);
    }
  }
}

function trimBrowserCacheMaps(): void {
  while (browserParentCache.size > BROWSER_CACHE_MAX_ENTRIES) {
    const oldestKey = browserParentCache.keys().next().value;
    if (oldestKey === undefined) {
      break;
    }
    browserParentCache.delete(oldestKey);
  }
}

function rememberBrowserParentCacheEntry(
  cacheKey: string,
  view: KnowledgeBrowserView,
  parentId: string | null,
  nodes: KnowledgeBrowserNode[],
  nextCursor: string | null,
  hasMore: boolean,
  append = false,
): void {
  purgeExpiredBrowserCacheEntries();
  const previous = browserParentCache.get(cacheKey);
  const mergedNodes =
    append && previous && previous.view === view
      ? mergeUniqueBrowserNodes(previous.nodes, nodes)
      : nodes;
  browserParentCache.set(cacheKey, {
    fetchedAt: Date.now(),
    view,
    parentId,
    nodes: mergedNodes,
    nextCursor,
    hasMore,
  });
  trimBrowserCacheMaps();
}

function mergeUniqueBrowserNodes(
  existing: KnowledgeBrowserNode[],
  incoming: KnowledgeBrowserNode[],
): KnowledgeBrowserNode[] {
  const merged = new Map<string, KnowledgeBrowserNode>();
  for (const node of existing) {
    merged.set(node.id, node);
  }
  for (const node of incoming) {
    merged.set(node.id, node);
  }
  return Array.from(merged.values());
}

function invalidateBrowserCachesForSpaceId(spaceId: string): void {
  const prefix = `${spaceId}:`;
  for (const key of browserParentCache.keys()) {
    if (key.startsWith(prefix)) {
      browserParentCache.delete(key);
    }
  }
}

export function invalidateKnowledgeBrowserNodeCache(spaceId?: string): void {
  if (spaceId === undefined) {
    browserParentCache.clear();
    return;
  }
  invalidateBrowserCachesForSpaceId(spaceId);
}

export function invalidateKnowledgeBrowserNodeCacheForSpaceIds(
  ...spaceIds: Array<string | null | undefined>
): void {
  for (const spaceId of spaceIds) {
    if (spaceId !== null && spaceId !== undefined && spaceId !== '' && spaceId !== '0') {
      invalidateBrowserCachesForSpaceId(spaceId);
    }
  }
}

export function invalidateKnowledgeBrowserNodeCacheForKbIds(
  ...kbIds: Array<string | null | undefined>
): void {
  for (const kbId of kbIds) {
    if (kbId === null || kbId === undefined) {
      continue;
    }
    try {
      invalidateBrowserCachesForSpaceId(parseKnowledgeSpaceId(kbId));
    } catch (error) {
      if (!isKnowledgebaseAppError(error)) {
        throw error;
      }
    }
  }
}

export interface KnowledgeBrowserNodesPageResult {
  parentId: string | null;
  items: KnowledgeBrowserNode[];
  nextCursor: string | null;
  hasMore: boolean;
}

function resolveBrowserPageParentId(page: unknown): string | null {
  if (typeof page !== 'object' || page === null || Array.isArray(page)) {
    return null;
  }
  const parentId = (page as { parentId?: unknown }).parentId;
  return typeof parentId === 'string' && parentId.trim() ? parentId : null;
}

export async function listKnowledgeBrowserNodesPage(
  spaceId: string,
  parentId: string | null,
  options?: {
    cursor?: string | null;
    pageSize?: number;
    fresh?: boolean;
    view?: KnowledgeBrowserView;
  },
): Promise<KnowledgeBrowserNodesPageResult> {
  const view = options?.view ?? DEFAULT_BROWSER_VIEW;
  const pageSize = options?.pageSize ?? DEFAULT_BROWSER_PAGE_SIZE;
  const cursor = options?.cursor ?? null;
  const cacheKey = parentCacheKey(spaceId, view, parentId);

  if (!cursor && !options?.fresh) {
    const cached = browserParentCache.get(cacheKey);
    if (cached && Date.now() - cached.fetchedAt < BROWSER_NODE_CACHE_TTL_MS) {
      return {
        parentId: cached.parentId,
        items: cached.nodes,
        nextCursor: cached.nextCursor,
        hasMore: cached.hasMore,
      };
    }
    if (cached) {
      browserParentCache.delete(cacheKey);
    }
  }

  const client = requireSdkClient();
  const page = await client.knowledge.spaces.browser.list(spaceId, {
    view,
    parentId,
    cursor,
    pageSize,
  });
  const normalized = normalizeSdkWorkListPage<KnowledgeBrowserNode>(page);
  const pageParentId = resolveBrowserPageParentId(page);
  const nextCursor = normalized.nextCursor;
  const hasMore = normalized.hasMore;

  // First pages replace the cached window; continuation pages append so a
  // resolution scan fills the cache and later lookups hit it instead of
  // re-scanning the space.
  rememberBrowserParentCacheEntry(
    cacheKey,
    view,
    pageParentId,
    normalized.items,
    nextCursor,
    hasMore,
    cursor !== null,
  );

  return {
    parentId: pageParentId,
    items: normalized.items,
    nextCursor,
    hasMore,
  };
}

export async function listKnowledgeBrowserNodesForParent(
  spaceId: string,
  parentId: string | null,
  options?: { fresh?: boolean; pageSize?: number; view?: KnowledgeBrowserView },
): Promise<KnowledgeBrowserNode[]> {
  const page = await listKnowledgeBrowserNodesPage(spaceId, parentId, {
    fresh: options?.fresh,
    pageSize: options?.pageSize,
    view: options?.view,
  });
  return page.items;
}

export async function ensureKnowledgeBrowserFolderLoaded(
  spaceId: string,
  folderId: string | null,
  options?: { view?: KnowledgeBrowserView },
): Promise<KnowledgeBrowserNode[]> {
  return listKnowledgeBrowserNodesForParent(spaceId, folderId, {
    view: options?.view,
  });
}

/**
 * True when the cached window for a folder (or the root when `folderId` is null) still
 * has continuation pages. Interactive lists render a "load more" affordance from this
 * signal instead of aggregating the full collection (PAGINATION_SPEC §8).
 */
export function hasMoreKnowledgeBrowserNodes(
  spaceId: string,
  folderId: string | null,
  options?: { view?: KnowledgeBrowserView },
): boolean {
  const view = options?.view ?? DEFAULT_BROWSER_VIEW;
  const entry = browserParentCache.get(parentCacheKey(spaceId, view, folderId));
  return entry !== undefined && entry.hasMore;
}

/**
 * Appends the next cursor page of a folder (or the root) to the cached window and
 * returns whether more pages remain. The UI calls this on demand from a "load more"
 * action; it never prefetches unbounded page aggregates.
 */
export async function loadMoreKnowledgeBrowserNodes(
  spaceId: string,
  folderId: string | null,
  options?: { view?: KnowledgeBrowserView },
): Promise<boolean> {
  const view = options?.view ?? DEFAULT_BROWSER_VIEW;
  const entry = browserParentCache.get(parentCacheKey(spaceId, view, folderId));
  if (!entry || !entry.hasMore || !entry.nextCursor) {
    return false;
  }
  const page = await listKnowledgeBrowserNodesPage(spaceId, folderId, {
    cursor: entry.nextCursor,
    view,
  });
  return page.hasMore;
}

export async function listLoadedKnowledgeBrowserNodes(
  spaceId: string,
  options?: { includeRoot?: boolean; view?: KnowledgeBrowserView },
): Promise<KnowledgeBrowserNode[]> {
  const view = options?.view ?? DEFAULT_BROWSER_VIEW;
  if (options?.includeRoot !== false) {
    await listKnowledgeBrowserNodesForParent(spaceId, null, { view });
  }
  return getLoadedKnowledgeBrowserNodes(spaceId, { view });
}

export interface KnowledgeBrowserNodeScanOptions {
  view?: KnowledgeBrowserView;
  fresh?: boolean;
}

/**
 * Scans a space page by page until `predicate` matches or the bounded page
 * budget is exhausted. Every visited page is cached (append mode), so repeated
 * resolutions hit the cache instead of re-scanning. Returns the first match.
 */
export async function scanKnowledgeBrowserNodes(
  spaceId: string,
  predicate: (node: KnowledgeBrowserNode) => boolean,
  options?: KnowledgeBrowserNodeScanOptions,
): Promise<KnowledgeBrowserNode | null> {
  const view = options?.view ?? DEFAULT_BROWSER_VIEW;
  let cursor: string | null = null;
  let pages = 0;
  do {
    const page = await listKnowledgeBrowserNodesPage(spaceId, null, {
      cursor,
      fresh: options?.fresh === true && cursor === null,
      view,
    });
    const found = page.items.find(predicate);
    if (found) {
      return found;
    }
    cursor = page.hasMore ? page.nextCursor : null;
    pages += 1;
  } while (cursor && pages < MAX_BROWSER_RESOLVE_SCAN_PAGES);
  return null;
}

export function findKnowledgeBrowserNodeByDocumentId(
  nodes: KnowledgeBrowserNode[],
  documentId: string,
  kbId: string,
): KnowledgeBrowserNode | null {
  return nodes.find(
    (candidate) => resolveBrowserDocumentId(candidate, kbId) === documentId,
  ) ?? nodes.find((candidate) => candidate.id === documentId) ?? null;
}

export function resolveBrowserDocumentId(node: KnowledgeBrowserNode, kbId: string): string {
  if (node.conceptId) {
    return `okf:${kbId}:${node.conceptId}`;
  }
  if (node.documentId) {
    return String(node.documentId);
  }
  return node.id;
}
