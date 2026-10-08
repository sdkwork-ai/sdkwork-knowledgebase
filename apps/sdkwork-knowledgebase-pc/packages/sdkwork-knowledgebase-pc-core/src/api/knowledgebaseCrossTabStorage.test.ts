import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const STORAGE_KEY = 'sdkwork.knowledgebase.test.v1';

type CrossTabStorageModule = typeof import('./knowledgebaseCrossTabStorage');

const recorded: {
  messages: unknown[];
  constructed: string[];
  events: Array<{ type: string; key: string | null }>;
} = {
  messages: [],
  constructed: [],
  events: [],
};

class RecordingBroadcastChannel {
  constructor(channelName: string) {
    recorded.constructed.push(channelName);
  }

  postMessage(message: unknown): void {
    recorded.messages.push(message);
  }

  close(): void {}
}

class RecordingStorageEvent {
  type: string;
  key: string | null;

  constructor(type: string, init?: { key?: string | null }) {
    this.type = type;
    this.key = init?.key ?? null;
  }
}

const windowWithListeners = {
  dispatchEvent: (event: { type: string; key: string | null }) => {
    recorded.events.push({ type: event.type, key: event.key });
    return true;
  },
};

async function loadModule(): Promise<CrossTabStorageModule> {
  return import('./knowledgebaseCrossTabStorage');
}

describe('knowledgebaseCrossTabStorage', () => {
  beforeEach(() => {
    recorded.messages = [];
    recorded.constructed = [];
    recorded.events = [];
    vi.resetModules();
    vi.stubGlobal('BroadcastChannel', RecordingBroadcastChannel);
    vi.stubGlobal('StorageEvent', RecordingStorageEvent);
    vi.stubGlobal('window', windowWithListeners);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('re-reads immediately before mutating and writes the derived value', async () => {
    const { withCrossTabStorageUpdate } = await loadModule();
    let readCount = 0;
    let written: string | undefined;

    const result = withCrossTabStorageUpdate<string[]>(
      STORAGE_KEY,
      () => {
        readCount += 1;
        return ['existing'];
      },
      (current) => [...current, 'added'],
      (next) => {
        written = JSON.stringify(next);
      },
    );

    expect(readCount).toBe(1);
    expect(result).toEqual(['existing', 'added']);
    expect(written).toBe(JSON.stringify(['existing', 'added']));
    expect(recorded.messages).toEqual([STORAGE_KEY]);
  });

  it('publishes the affected key over the shared knowledgebase broadcast channel', async () => {
    const { publishCrossTabStorageChange } = await loadModule();

    publishCrossTabStorageChange(STORAGE_KEY);

    expect(recorded.constructed).toEqual(['sdkwork-knowledgebase-storage']);
    expect(recorded.messages).toEqual([STORAGE_KEY]);
    expect(recorded.events).toEqual([]);
  });

  it('reuses a single broadcast channel across publishes', async () => {
    const { publishCrossTabStorageChange } = await loadModule();

    publishCrossTabStorageChange(`${STORAGE_KEY}.a`);
    publishCrossTabStorageChange(`${STORAGE_KEY}.b`);

    expect(recorded.constructed).toEqual(['sdkwork-knowledgebase-storage']);
    expect(recorded.messages).toEqual([`${STORAGE_KEY}.a`, `${STORAGE_KEY}.b`]);
  });

  it('falls back to a storage event when BroadcastChannel is unavailable', async () => {
    const { publishCrossTabStorageChange } = await loadModule();
    vi.stubGlobal('BroadcastChannel', undefined);

    publishCrossTabStorageChange(STORAGE_KEY);

    expect(recorded.messages).toEqual([]);
    expect(recorded.events).toEqual([{ type: 'storage', key: STORAGE_KEY }]);
  });

  it('does not mutate, write, or publish without a window (server/SSR)', async () => {
    const { withCrossTabStorageUpdate } = await loadModule();
    vi.stubGlobal('window', undefined);

    const result = withCrossTabStorageUpdate<string[]>(
      STORAGE_KEY,
      () => ['read-only'],
      () => {
        throw new Error('mutate must not run without a window');
      },
      () => {
        throw new Error('write must not run without a window');
      },
    );

    expect(result).toEqual(['read-only']);
    expect(recorded.messages).toEqual([]);
  });
});
