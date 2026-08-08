import { DocumentMeta } from './document';

interface TabCacheEntry {
  activeId: string | null;
  docs: DocumentMeta[];
}

export interface KnowledgebaseTabCache {
  closeAll(kbId: string): void;
  closeOthers(kbId: string, docId: string): { remainingDocs: DocumentMeta[] };
  closeToRight(kbId: string, docId: string): {
    nextActiveId: string | null;
    remainingDocs: DocumentMeta[];
  };
  closeDoc(kbId: string, docId: string): {
    nextActiveId: string | null;
    remainingDocs: DocumentMeta[];
  };
  dispose(): void;
  getActiveDocId(kbId: string): string | null;
  getOpenDocs(kbId: string): DocumentMeta[];
  initKb(kbId: string): void;
  openDoc(kbId: string, doc: DocumentMeta): void;
}

export class TabCacheService {
  private static STORAGE_KEY = 'app-tabs-cache-v2';
  /** Bounded open-tab window per Knowledge Base so the localStorage payload cannot grow
   *  without limit; the oldest tab is evicted when the window overflows. */
  private static MAX_TABS_PER_KB = 50;
  private static MAX_CACHED_KBS = 20;

  /**
   * Defines a unified service and methods to facilitate future desktop application compatibility.
   * Caches the list of opened tabs and the currently active tab for each Knowledge Base.
   */

  private static loadCache(): Record<string, TabCacheEntry> {
    try {
      const data = localStorage.getItem(this.STORAGE_KEY);
      return data ? JSON.parse(data) : {};
    } catch {
      return {};
    }
  }

  private static saveCache(data: Record<string, TabCacheEntry>) {
    // Bound the total persisted window across Knowledge Bases.
    const keys = Object.keys(data);
    if (keys.length > this.MAX_CACHED_KBS) {
      for (const key of keys.slice(0, keys.length - this.MAX_CACHED_KBS)) {
        delete data[key];
      }
    }
    localStorage.setItem(this.STORAGE_KEY, JSON.stringify(data));
  }

  private static trimKbWindow(cache: TabCacheEntry): void {
    if (cache.docs.length <= this.MAX_TABS_PER_KB) {
      return;
    }
    const activeId = cache.activeId;
    const overflow = cache.docs.length - this.MAX_TABS_PER_KB;
    const evicted = new Set(cache.docs.slice(0, overflow).map((doc) => doc.id));
    cache.docs = cache.docs.slice(overflow);
    if (activeId !== null && evicted.has(activeId)) {
      cache.activeId = cache.docs[cache.docs.length - 1]?.id ?? null;
    }
  }

  public static dispose(): void {
    // Persistent workspaces intentionally retain their normal tab restoration state.
  }

  // Handle a new kb structure gracefully.
  public static initKb(kbId: string): void {
    const cache = this.loadCache();
    if (!cache[kbId]) {
      cache[kbId] = { docs: [], activeId: null };
      this.saveCache(cache);
    }
  }

  public static getOpenDocs(kbId: string): DocumentMeta[] {
    const cache = this.loadCache();
    return cache[kbId]?.docs || [];
  }

  public static getActiveDocId(kbId: string): string | null {
    const cache = this.loadCache();
    return cache[kbId]?.activeId || null;
  }

  public static saveOpenDocs(kbId: string, docs: DocumentMeta[]): void {
    const cache = this.loadCache();
    if (!cache[kbId]) cache[kbId] = { docs: [], activeId: null };
    cache[kbId].docs = docs;
    this.trimKbWindow(cache[kbId]);
    this.saveCache(cache);
  }

  public static saveActiveDocId(kbId: string, activeId: string | null): void {
    const cache = this.loadCache();
    if (!cache[kbId]) cache[kbId] = { docs: [], activeId: null };
    cache[kbId].activeId = activeId;
    this.saveCache(cache);
  }

  public static openDoc(kbId: string, doc: DocumentMeta): void {
    const cache = this.loadCache();
    if (!cache[kbId]) cache[kbId] = { docs: [], activeId: null };
    
    // Add to open tabs if not a folder and not already present
    if (doc.type !== 'folder') {
      if (!cache[kbId].docs.some(d => d.id === doc.id)) {
        cache[kbId].docs.push(doc);
      }
    }
    this.trimKbWindow(cache[kbId]);

    cache[kbId].activeId = doc.id;
    this.saveCache(cache);
  }

  public static closeDoc(kbId: string, docId: string): { remainingDocs: DocumentMeta[], nextActiveId: string | null } {
    const cache = this.loadCache();
    if (!cache[kbId]) return { remainingDocs: [], nextActiveId: null };

    const index = cache[kbId].docs.findIndex(d => d.id === docId);
    if (index === -1) return { remainingDocs: cache[kbId].docs, nextActiveId: cache[kbId].activeId };

    const remainingDocs = cache[kbId].docs.filter(d => d.id !== docId);
    let nextActiveId = cache[kbId].activeId;

    if (cache[kbId].activeId === docId) {
      if (remainingDocs.length > 0) {
        const newIndex = Math.min(index, remainingDocs.length - 1);
        nextActiveId = remainingDocs[newIndex].id;
      } else {
        nextActiveId = null;
      }
    }

    cache[kbId].docs = remainingDocs;
    cache[kbId].activeId = nextActiveId;
    this.saveCache(cache);

    return { remainingDocs, nextActiveId };
  }

  public static closeOthers(kbId: string, docId: string): { remainingDocs: DocumentMeta[] } {
    const cache = this.loadCache();
    if (!cache[kbId]) return { remainingDocs: [] };

    const docToKeep = cache[kbId].docs.find(d => d.id === docId);
    const remainingDocs = docToKeep ? [docToKeep] : [];
    
    cache[kbId].docs = remainingDocs;
    if (docToKeep) {
      cache[kbId].activeId = docId;
    } else {
      cache[kbId].activeId = null;
    }
    
    this.saveCache(cache);
    return { remainingDocs };
  }

  public static closeAll(kbId: string): void {
    const cache = this.loadCache();
    if (!cache[kbId]) return;
    cache[kbId].docs = [];
    cache[kbId].activeId = null;
    this.saveCache(cache);
  }

  public static closeToRight(kbId: string, docId: string): { remainingDocs: DocumentMeta[], nextActiveId: string | null } {
    const cache = this.loadCache();
    if (!cache[kbId]) return { remainingDocs: [], nextActiveId: null };

    const index = cache[kbId].docs.findIndex(d => d.id === docId);
    if (index === -1) return { remainingDocs: cache[kbId].docs, nextActiveId: cache[kbId].activeId };

    const remainingDocs = cache[kbId].docs.slice(0, index + 1);
    let nextActiveId = cache[kbId].activeId;

    // Check if the currently active doc was removed
    if (nextActiveId && !remainingDocs.some(d => d.id === nextActiveId)) {
        nextActiveId = docId;
    }

    cache[kbId].docs = remainingDocs;
    cache[kbId].activeId = nextActiveId;
    this.saveCache(cache);

    return { remainingDocs, nextActiveId };
  }
}

/** Keeps fixed group-workspace tab metadata in process memory only. */
export class EphemeralTabCacheService implements KnowledgebaseTabCache {
  private readonly cache = new Map<string, TabCacheEntry>();

  private getOrCreateEntry(kbId: string): TabCacheEntry {
    let entry = this.cache.get(kbId);
    if (!entry) {
      entry = { docs: [], activeId: null };
      this.cache.set(kbId, entry);
    }
    return entry;
  }

  public initKb(kbId: string): void {
    this.getOrCreateEntry(kbId);
  }

  public getOpenDocs(kbId: string): DocumentMeta[] {
    return this.cache.get(kbId)?.docs ?? [];
  }

  public getActiveDocId(kbId: string): string | null {
    return this.cache.get(kbId)?.activeId ?? null;
  }

  public openDoc(kbId: string, doc: DocumentMeta): void {
    const entry = this.getOrCreateEntry(kbId);
    if (doc.type !== 'folder' && !entry.docs.some((item) => item.id === doc.id)) {
      entry.docs.push(doc);
    }
    entry.activeId = doc.id;
  }

  public closeDoc(kbId: string, docId: string): {
    remainingDocs: DocumentMeta[];
    nextActiveId: string | null;
  } {
    const entry = this.cache.get(kbId);
    if (!entry) {
      return { remainingDocs: [], nextActiveId: null };
    }

    const index = entry.docs.findIndex((doc) => doc.id === docId);
    if (index === -1) {
      return { remainingDocs: entry.docs, nextActiveId: entry.activeId };
    }

    const remainingDocs = entry.docs.filter((doc) => doc.id !== docId);
    const nextActiveId = entry.activeId === docId
      ? remainingDocs[Math.min(index, remainingDocs.length - 1)]?.id ?? null
      : entry.activeId;
    entry.docs = remainingDocs;
    entry.activeId = nextActiveId;
    return { remainingDocs, nextActiveId };
  }

  public closeOthers(kbId: string, docId: string): { remainingDocs: DocumentMeta[] } {
    const entry = this.cache.get(kbId);
    if (!entry) {
      return { remainingDocs: [] };
    }

    const docToKeep = entry.docs.find((doc) => doc.id === docId);
    entry.docs = docToKeep ? [docToKeep] : [];
    entry.activeId = docToKeep ? docId : null;
    return { remainingDocs: entry.docs };
  }

  public closeAll(kbId: string): void {
    const entry = this.cache.get(kbId);
    if (!entry) {
      return;
    }
    entry.docs = [];
    entry.activeId = null;
  }

  public closeToRight(kbId: string, docId: string): {
    remainingDocs: DocumentMeta[];
    nextActiveId: string | null;
  } {
    const entry = this.cache.get(kbId);
    if (!entry) {
      return { remainingDocs: [], nextActiveId: null };
    }

    const index = entry.docs.findIndex((doc) => doc.id === docId);
    if (index === -1) {
      return { remainingDocs: entry.docs, nextActiveId: entry.activeId };
    }

    const remainingDocs = entry.docs.slice(0, index + 1);
    const nextActiveId = remainingDocs.some((doc) => doc.id === entry.activeId)
      ? entry.activeId
      : docId;
    entry.docs = remainingDocs;
    entry.activeId = nextActiveId;
    return { remainingDocs, nextActiveId };
  }

  public dispose(): void {
    this.cache.clear();
  }
}
