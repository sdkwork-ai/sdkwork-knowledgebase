import { withCrossTabStorageUpdate } from 'sdkwork-knowledgebase-pc-core';

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

  /**
   * Runs one read→mutate→write cycle against a fresh snapshot taken immediately
   * before the write, then publishes the storage key so other tabs reload it.
   * Mutations that leave the cache content unchanged skip the write so no-op
   * paths keep storage traffic quiet.
   */
  private static updateCache(
    mutate: (cache: Record<string, TabCacheEntry>) => Record<string, TabCacheEntry>,
  ): void {
    // Snapshot the JSON at read time: the mutate closures revise the cache in
    // place, so a reference held until the write would compare the mutated
    // object against itself and always look unchanged.
    let originalJson: string | null = null;
    withCrossTabStorageUpdate(
      this.STORAGE_KEY,
      () => {
        const cache = this.loadCache();
        originalJson = JSON.stringify(cache);
        return cache;
      },
      (cache) => mutate(cache),
      (next) => {
        if (originalJson !== null && JSON.stringify(next) === originalJson) {
          return;
        }
        this.saveCache(next);
      },
    );
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
    this.updateCache((cache) => {
      if (cache[kbId]) {
        return cache;
      }
      cache[kbId] = { docs: [], activeId: null };
      return cache;
    });
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
    this.updateCache((cache) => {
      const entry = cache[kbId] ?? { docs: [], activeId: null };
      entry.docs = docs;
      this.trimKbWindow(entry);
      cache[kbId] = entry;
      return cache;
    });
  }

  public static saveActiveDocId(kbId: string, activeId: string | null): void {
    this.updateCache((cache) => {
      const entry = cache[kbId] ?? { docs: [], activeId: null };
      entry.activeId = activeId;
      cache[kbId] = entry;
      return cache;
    });
  }

  public static openDoc(kbId: string, doc: DocumentMeta): void {
    this.updateCache((cache) => {
      const entry = cache[kbId] ?? { docs: [], activeId: null };

      // Add to open tabs if not a folder and not already present
      if (doc.type !== 'folder' && !entry.docs.some((d) => d.id === doc.id)) {
        entry.docs.push(doc);
      }
      this.trimKbWindow(entry);

      entry.activeId = doc.id;
      cache[kbId] = entry;
      return cache;
    });
  }

  public static closeDoc(kbId: string, docId: string): { remainingDocs: DocumentMeta[], nextActiveId: string | null } {
    let result: { remainingDocs: DocumentMeta[], nextActiveId: string | null } = {
      remainingDocs: [],
      nextActiveId: null,
    };

    this.updateCache((cache) => {
      const entry = cache[kbId];
      if (!entry) {
        return cache;
      }

      const index = entry.docs.findIndex((d) => d.id === docId);
      if (index === -1) {
        result = { remainingDocs: entry.docs, nextActiveId: entry.activeId };
        return cache;
      }

      const remainingDocs = entry.docs.filter((d) => d.id !== docId);
      let nextActiveId = entry.activeId;

      if (entry.activeId === docId) {
        if (remainingDocs.length > 0) {
          const newIndex = Math.min(index, remainingDocs.length - 1);
          nextActiveId = remainingDocs[newIndex].id;
        } else {
          nextActiveId = null;
        }
      }

      entry.docs = remainingDocs;
      entry.activeId = nextActiveId;
      result = { remainingDocs, nextActiveId };
      return cache;
    });

    return result;
  }

  public static closeOthers(kbId: string, docId: string): { remainingDocs: DocumentMeta[] } {
    let result: { remainingDocs: DocumentMeta[] } = { remainingDocs: [] };

    this.updateCache((cache) => {
      const entry = cache[kbId];
      if (!entry) {
        return cache;
      }

      const docToKeep = entry.docs.find((d) => d.id === docId);
      const remainingDocs = docToKeep ? [docToKeep] : [];

      entry.docs = remainingDocs;
      entry.activeId = docToKeep ? docId : null;
      result = { remainingDocs };
      return cache;
    });

    return result;
  }

  public static closeAll(kbId: string): void {
    this.updateCache((cache) => {
      const entry = cache[kbId];
      if (!entry) {
        return cache;
      }
      entry.docs = [];
      entry.activeId = null;
      return cache;
    });
  }

  public static closeToRight(kbId: string, docId: string): { remainingDocs: DocumentMeta[], nextActiveId: string | null } {
    let result: { remainingDocs: DocumentMeta[], nextActiveId: string | null } = {
      remainingDocs: [],
      nextActiveId: null,
    };

    this.updateCache((cache) => {
      const entry = cache[kbId];
      if (!entry) {
        return cache;
      }

      const index = entry.docs.findIndex((d) => d.id === docId);
      if (index === -1) {
        result = { remainingDocs: entry.docs, nextActiveId: entry.activeId };
        return cache;
      }

      const remainingDocs = entry.docs.slice(0, index + 1);
      let nextActiveId = entry.activeId;

      // Check if the currently active doc was removed
      if (nextActiveId && !remainingDocs.some((d) => d.id === nextActiveId)) {
        nextActiveId = docId;
      }

      entry.docs = remainingDocs;
      entry.activeId = nextActiveId;
      result = { remainingDocs, nextActiveId };
      return cache;
    });

    return result;
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
