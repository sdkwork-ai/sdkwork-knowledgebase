import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { DocumentMeta } from './document';
import { EphemeralTabCacheService, TabCacheService } from './tabService';

const groupDocument = {
  id: 'group-document-1',
  kbId: 'group-space-42',
  title: 'Group document',
  type: 'richtext',
} as DocumentMeta;

const originalLocalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');

const STORAGE_KEY = 'app-tabs-cache-v2';

const broadcastMessages: unknown[] = [];

class RecordingBroadcastChannel {
  constructor(public readonly name: string) {}

  postMessage(message: unknown): void {
    broadcastMessages.push(message);
  }

  close(): void {}
}

function createFakeLocalStorage(): {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
  store: Map<string, string>;
  setItemCalls: number;
} {
  const store = new Map<string, string>();
  const fake = {
    store,
    setItemCalls: 0,
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      fake.setItemCalls += 1;
      store.set(key, value);
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => {
      store.clear();
    },
  };
  return fake;
}

afterEach(() => {
  if (originalLocalStorage) {
    Object.defineProperty(globalThis, 'localStorage', originalLocalStorage);
  } else {
    Reflect.deleteProperty(globalThis, 'localStorage');
  }
  vi.unstubAllGlobals();
});

describe('EphemeralTabCacheService', () => {
  it('keeps fixed group tab metadata in memory and clears it on disposal', () => {
    const localStorage = {
      getItem: vi.fn(),
      setItem: vi.fn(),
    };
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: localStorage,
    });

    const cache = new EphemeralTabCacheService();
    cache.initKb('group-space-42');
    cache.openDoc('group-space-42', groupDocument);

    expect(cache.getOpenDocs('group-space-42')).toEqual([groupDocument]);
    expect(cache.getActiveDocId('group-space-42')).toBe('group-document-1');
    expect(localStorage.getItem).not.toHaveBeenCalled();
    expect(localStorage.setItem).not.toHaveBeenCalled();

    cache.dispose();

    expect(cache.getOpenDocs('group-space-42')).toEqual([]);
    expect(cache.getActiveDocId('group-space-42')).toBeNull();
  });
});

describe('TabCacheService cross-tab safe writes', () => {
  let fakeStorage: ReturnType<typeof createFakeLocalStorage>;

  beforeEach(() => {
    broadcastMessages.length = 0;
    fakeStorage = createFakeLocalStorage();
    vi.stubGlobal('localStorage', fakeStorage);
    vi.stubGlobal('BroadcastChannel', RecordingBroadcastChannel);
    vi.stubGlobal('window', globalThis);
  });

  it('re-reads storage before each write so entries written by another tab survive', () => {
    const otherTabDoc = { id: 'other-doc-1', kbId: 'kb-other', title: 'Other', type: 'richtext' } as DocumentMeta;
    fakeStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        'kb-other': { docs: [otherTabDoc], activeId: 'other-doc-1' },
      }),
    );

    TabCacheService.openDoc('kb-mine', groupDocument);

    const stored = JSON.parse(fakeStorage.getItem(STORAGE_KEY) ?? '{}') as Record<string, {
      docs: DocumentMeta[];
      activeId: string | null;
    }>;
    expect(stored['kb-other']?.docs).toEqual([otherTabDoc]);
    expect(stored['kb-mine']?.docs).toEqual([groupDocument]);
    expect(stored['kb-mine']?.activeId).toBe('group-document-1');
  });

  it('publishes the storage key so other tabs reload it after a write', () => {
    TabCacheService.openDoc('kb-mine', groupDocument);

    expect(broadcastMessages).toContain(STORAGE_KEY);
  });

  it('returns the untouched entry and skips the write for a no-op mutation', () => {
    fakeStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        'kb-mine': { docs: [groupDocument], activeId: 'group-document-1' },
      }),
    );
    const writesAfterSeed = fakeStorage.setItemCalls;

    const result = TabCacheService.closeDoc('kb-mine', 'missing-doc');

    expect(result).toEqual({
      remainingDocs: [groupDocument],
      nextActiveId: 'group-document-1',
    });
    // No write beyond the seed: the unchanged cache must not be re-persisted.
    expect(fakeStorage.setItemCalls).toBe(writesAfterSeed);
  });
});
