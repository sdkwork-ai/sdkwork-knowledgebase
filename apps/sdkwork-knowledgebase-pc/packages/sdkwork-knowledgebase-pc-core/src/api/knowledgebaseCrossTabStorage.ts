/**
 * Minimal coordination for localStorage read-modify-write cycles that can race
 * across tabs or across async flows. `withCrossTabStorageUpdate` takes its
 * snapshot immediately before the mutation is applied and the write lands
 * (narrowing the lost-update window instead of letting callers hold a stale
 * copy), then publishes the affected key so other tabs reload it.
 *
 * Deliberately not a state-sync framework: stores stay read-through (every
 * access re-reads localStorage), and the only extra surface is the publish.
 */

const STORAGE_BROADCAST_CHANNEL_NAME = 'sdkwork-knowledgebase-storage';

let storageBroadcastChannel: BroadcastChannel | null | undefined;

function acquireStorageBroadcastChannel(): BroadcastChannel | null {
  if (typeof window === 'undefined') {
    return null;
  }
  if (storageBroadcastChannel !== undefined) {
    return storageBroadcastChannel;
  }
  try {
    storageBroadcastChannel =
      typeof BroadcastChannel === 'function'
        ? new BroadcastChannel(STORAGE_BROADCAST_CHANNEL_NAME)
        : null;
  } catch {
    storageBroadcastChannel = null;
  }
  return storageBroadcastChannel;
}

/**
 * Publishes a storage-key change so other tabs reload the affected key.
 * Uses `BroadcastChannel('sdkwork-knowledgebase-storage')` and falls back to a
 * synthetic storage event where BroadcastChannel is unavailable. No-op on
 * server/SSR runtimes without `window`.
 */
export function publishCrossTabStorageChange(storageKey: string): void {
  if (typeof window === 'undefined') {
    return;
  }
  const channel = acquireStorageBroadcastChannel();
  if (channel) {
    channel.postMessage(storageKey);
    return;
  }
  try {
    window.dispatchEvent(new StorageEvent('storage', { key: storageKey }));
  } catch {
    // StorageEvent construction unavailable: the native write event remains.
  }
}

/**
 * Runs one read→mutate→write cycle: `readParse` re-reads the affected key
 * immediately before `mutate` derives the next value and `writeSerialize`
 * persists it, then the key is published to other tabs. On server/SSR runtimes
 * (no `window`) only the read runs — no mutation, write, or publish.
 */
export function withCrossTabStorageUpdate<T>(
  storageKey: string,
  readParse: () => T,
  mutate: (current: T) => T,
  writeSerialize: (next: T) => void,
): T {
  if (typeof window === 'undefined') {
    return readParse();
  }
  const next = mutate(readParse());
  writeSerialize(next);
  publishCrossTabStorageChange(storageKey);
  return next;
}
