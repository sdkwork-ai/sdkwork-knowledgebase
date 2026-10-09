import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  NotesAutosaveEngine,
  type NotesAutosaveEvent,
  type NotesAutosavePort,
} from './notesAutosaveEngine';

const DEBOUNCE_MS = 1200;
const RETRY_DELAYS_MS = [1000, 3000];
const NOTE_ID = 'note-1';

interface ContentCall {
  noteId: string;
  content: string;
  baseVersionId: string | null;
}

interface TitleCall {
  noteId: string;
  title: string;
}

interface ContentGate {
  resolve: (versionId: string) => void;
  reject: (error: unknown) => void;
}

function conflictError(): Error {
  return Object.assign(new Error('document was updated elsewhere'), { code: 'conflict' });
}

interface PortMockOptions {
  /** Number of leading content saves that fail with a transient error. */
  failContentTimes?: number;
  /** The Nth content save fails with the canonical conflict error. */
  conflictOnContentAttempt?: number;
}

function createPortMock(options: PortMockOptions = {}) {
  const titleCalls: TitleCall[] = [];
  const contentCalls: ContentCall[] = [];
  const gates: ContentGate[] = [];
  let contentAttempts = 0;
  const failureControl = { contentFailuresRemaining: options.failContentTimes ?? 0 };
  const conflictOnContentAttempt = options.conflictOnContentAttempt ?? 0;

  const port: NotesAutosavePort = {
    saveTitle: async (noteId, title) => {
      titleCalls.push({ noteId, title });
    },
    saveContent: (noteId, content, baseVersionId) => {
      contentCalls.push({ noteId, content, baseVersionId });
      contentAttempts += 1;
      const attempt = contentAttempts;
      return new Promise<string>((resolve, reject) => {
        gates.push({
          resolve: (versionId) => resolve(versionId),
          reject: (error) => reject(error),
        });
        // Auto-settle deterministic outcomes (transient failure / conflict /
        // success) unless the test wants to hold the gate itself.
        if (attempt === conflictOnContentAttempt) {
          queueMicrotask(() => reject(conflictError()));
          return;
        }
        if (failureControl.contentFailuresRemaining > 0) {
          failureControl.contentFailuresRemaining -= 1;
          queueMicrotask(() => reject(new Error('network down')));
          return;
        }
        queueMicrotask(() => resolve(`v${attempt}`));
      });
    },
    isConflictError: (error) =>
      typeof error === 'object' && error !== null && (error as { code?: unknown }).code === 'conflict',
  };

  return {
    port,
    titleCalls,
    contentCalls,
    gates,
    /** Lets a test heal the network mid-scenario. */
    healNetwork(): void {
      failureControl.contentFailuresRemaining = 0;
    },
    get contentAttempts(): number {
      return contentAttempts;
    },
  };
}

function createEngine(
  port: NotesAutosavePort,
  extras: { now?: () => number; onEvent?: (event: NotesAutosaveEvent) => void } = {},
): NotesAutosaveEngine {
  return new NotesAutosaveEngine({
    port,
    debounceMs: DEBOUNCE_MS,
    retryDelaysMs: RETRY_DELAYS_MS,
    now: extras.now,
    onEvent: extras.onEvent,
  });
}

async function settle(): Promise<void> {
  await vi.advanceTimersByTimeAsync(0);
}

describe('notes autosave engine', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('coalesces rapid edits into one debounced save with the latest value', async () => {
    const mock = createPortMock();
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'first draft');
    await vi.advanceTimersByTimeAsync(400);
    engine.edit(NOTE_ID, 'content', 'first draft, continued');
    await vi.advanceTimersByTimeAsync(400);
    engine.edit(NOTE_ID, 'content', 'final draft');
    expect(mock.contentCalls).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();

    expect(mock.contentCalls).toHaveLength(1);
    expect(mock.contentCalls[0]?.content).toBe('final draft');
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('flush() saves immediately without waiting for the debounce window', async () => {
    const mock = createPortMock();
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'urgent note');
    await engine.flush(NOTE_ID);

    expect(mock.contentCalls).toHaveLength(1);
    expect(mock.contentCalls[0]?.content).toBe('urgent note');
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('saves title and content together in order within one cycle', async () => {
    const mock = createPortMock();
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'title', 'Meeting notes');
    engine.edit(NOTE_ID, 'content', '<p>agenda</p>');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();

    expect(mock.titleCalls).toEqual([{ noteId: NOTE_ID, title: 'Meeting notes' }]);
    expect(mock.contentCalls).toHaveLength(1);
    engine.dispose();
  });

  it('serializes saves and chains the version baseline across cycles', async () => {
    const mock = createPortMock();
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'version one');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    // First save finished: it seeded the baseline from its result.
    expect(mock.contentCalls[0]?.baseVersionId).toBeNull();

    engine.edit(NOTE_ID, 'content', 'version two');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();

    expect(mock.contentCalls[1]?.baseVersionId).toBe('v1');
    expect(mock.contentCalls[1]?.content).toBe('version two');
    engine.dispose();
  });

  it('holds an in-flight save and runs a trailing cycle for mid-flight edits', async () => {
    const heldGates: ContentGate[] = [];
    const calls: ContentCall[] = [];
    const engine = new NotesAutosaveEngine({
      port: {
        saveTitle: async () => undefined,
        saveContent: (noteId, content, baseVersionId) => {
          calls.push({ noteId, content, baseVersionId });
          return new Promise<string>((resolve, reject) => {
            heldGates.push({ resolve, reject });
          });
        },
        isConflictError: (error) =>
          typeof error === 'object' && error !== null && (error as { code?: unknown }).code === 'conflict',
      },
      debounceMs: DEBOUNCE_MS,
      retryDelaysMs: RETRY_DELAYS_MS,
    });

    engine.edit(NOTE_ID, 'content', 'in-flight edit');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    expect(heldGates).toHaveLength(1);

    // Edit while the first save is still in flight.
    engine.edit(NOTE_ID, 'content', 'edited during flight');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS * 2);
    await settle();
    // Still serialized: no second concurrent request yet.
    expect(heldGates).toHaveLength(1);

    heldGates[0]!.resolve('v1');
    await settle();
    expect(heldGates).toHaveLength(2);
    expect(calls[1]?.content).toBe('edited during flight');
    expect(calls[1]?.baseVersionId).toBe('v1');

    heldGates[1]!.resolve('v2');
    await settle();
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('retries transient failures with backoff while preserving the buffer', async () => {
    const mock = createPortMock({ failContentTimes: 1 });
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'survives a network blip');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('dirty');
    expect(engine.getSnapshot(NOTE_ID)?.hasPendingEdits).toBe(true);

    await vi.advanceTimersByTimeAsync(RETRY_DELAYS_MS[0]!);
    await settle();
    expect(mock.contentCalls).toHaveLength(2);
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('gives up after the retry budget, keeps the buffer, and recovers via a fresh edit', async () => {
    const mock = createPortMock({ failContentTimes: Number.POSITIVE_INFINITY });
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'offline note');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    await vi.advanceTimersByTimeAsync(RETRY_DELAYS_MS[0]!);
    await settle();
    await vi.advanceTimersByTimeAsync(RETRY_DELAYS_MS[1]!);
    await settle();

    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('error');
    expect(engine.getSnapshot(NOTE_ID)?.hasPendingEdits).toBe(true);
    expect(mock.contentCalls).toHaveLength(3);

    // The network heals; the next edit re-arms the save cycle from scratch.
    mock.healNetwork();
    engine.edit(NOTE_ID, 'content', 'offline note, now online');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    expect(mock.contentCalls).toHaveLength(4);
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('pauses on conflict, keeps the buffer, and overwrite forces the save', async () => {
    const mock = createPortMock({ conflictOnContentAttempt: 1 });
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'local wins eventually');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();

    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('conflict');
    expect(engine.getSnapshot(NOTE_ID)?.hasPendingEdits).toBe(true);

    // No auto-retry must happen while paused.
    await vi.advanceTimersByTimeAsync(60_000);
    await settle();
    expect(mock.contentAttempts).toBe(1);

    await engine.overwrite(NOTE_ID);
    expect(mock.contentCalls[1]?.baseVersionId).toBeNull();
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('a fresh edit after a conflict re-arms normal saving', async () => {
    const mock = createPortMock({ conflictOnContentAttempt: 1 });
    const engine = createEngine(mock.port);

    engine.edit(NOTE_ID, 'content', 'conflicted draft');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('conflict');

    engine.edit(NOTE_ID, 'content', 'conflicted draft, edited');
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('dirty');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    // Second attempt does not conflict and succeeds; the baseline stayed null
    // because the conflicted attempt never returned a version id.
    expect(mock.contentCalls).toHaveLength(2);
    expect(mock.contentCalls[1]?.baseVersionId).toBeNull();
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('forget() drops the buffered edits and in-flight results never resurrect state', async () => {
    const heldGates: ContentGate[] = [];
    const engine = new NotesAutosaveEngine({
      port: {
        saveTitle: async () => undefined,
        saveContent: () =>
          new Promise<string>((resolve, reject) => {
            heldGates.push({ resolve, reject });
          }),
        isConflictError: () => false,
      },
      debounceMs: DEBOUNCE_MS,
      retryDelaysMs: RETRY_DELAYS_MS,
    });

    engine.edit(NOTE_ID, 'content', 'doomed edit');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    expect(heldGates).toHaveLength(1);

    engine.forget(NOTE_ID);
    expect(engine.getSnapshot(NOTE_ID)).toBeNull();
    expect(engine.hasPendingChanges(NOTE_ID)).toBe(false);

    heldGates[0]!.resolve('v1');
    await settle();
    expect(engine.getSnapshot(NOTE_ID)).toBeNull();

    // A later edit starts from a clean state.
    engine.edit(NOTE_ID, 'content', 'new life');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();
    expect(heldGates).toHaveLength(2);
    heldGates[1]!.resolve('v2');
    await settle();
    expect(engine.getSnapshot(NOTE_ID)?.status).toBe('saved');
    engine.dispose();
  });

  it('uses the opened baseline for the first save', async () => {
    const mock = createPortMock();
    const engine = createEngine(mock.port);

    engine.setVersionBaseline(NOTE_ID, 'v-open');
    engine.edit(NOTE_ID, 'content', 'based on opened version');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();

    expect(mock.contentCalls[0]?.baseVersionId).toBe('v-open');
    engine.dispose();
  });

  it('emits saved and conflict lifecycle events', async () => {
    const mock = createPortMock({ conflictOnContentAttempt: 1 });
    const events: NotesAutosaveEvent[] = [];
    const engine = createEngine(mock.port, { onEvent: (event) => events.push(event) });

    engine.edit(NOTE_ID, 'title', 't');
    engine.edit(NOTE_ID, 'content', 'c');
    await vi.advanceTimersByTimeAsync(DEBOUNCE_MS);
    await settle();

    const conflictEvent = events.find((event) => event.type === 'conflict');
    expect(conflictEvent?.type).toBe('conflict');
    engine.dispose();
  });

  it('reports the save time through the injected clock', async () => {
    let currentTime = 1_000_000;
    const mock = createPortMock();
    const engine = createEngine(mock.port, { now: () => currentTime });

    engine.edit(NOTE_ID, 'content', 'timestamped');
    await engine.flush(NOTE_ID);
    expect(engine.getSnapshot(NOTE_ID)?.lastSavedAt).toBe(1_000_000);

    currentTime = 2_000_000;
    engine.edit(NOTE_ID, 'content', 'timestamped again');
    await engine.flush(NOTE_ID);
    expect(engine.getSnapshot(NOTE_ID)?.lastSavedAt).toBe(2_000_000);
    engine.dispose();
  });
});
