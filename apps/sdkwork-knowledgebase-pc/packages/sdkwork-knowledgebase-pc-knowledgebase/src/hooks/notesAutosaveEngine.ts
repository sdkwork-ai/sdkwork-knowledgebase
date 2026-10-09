/**
 * Framework-agnostic autosave engine for the notes workspace.
 *
 * Industry-standard behavior (Notion / Bear style): every keystroke marks the
 * note dirty; the latest edit per field is coalesced and saved after a short
 * debounce; saves for the same note run strictly serially (latest value wins,
 * never two concurrent requests for one note); transient failures retry with
 * bounded backoff while the dirty buffer is preserved; a server-side conflict
 * pauses auto-save until the user explicitly overwrites; and every exit path
 * (note switch, tab hide, unmount) has an explicit flush.
 */

export type NotesSaveStatus = 'idle' | 'dirty' | 'saving' | 'saved' | 'error' | 'conflict';

export interface NotesAutosaveSnapshot {
  noteId: string;
  status: NotesSaveStatus;
  /** Server epoch ms of the last confirmed save; null before the first confirmed save. */
  lastSavedAt: number | null;
  /** True while local edits exist that the server has not confirmed. */
  hasPendingEdits: boolean;
}

export interface NotesAutosaveSavedEvent {
  type: 'saved';
  noteId: string;
  savedTitle: boolean;
  savedContent: boolean;
}

export interface NotesAutosaveConflictEvent {
  type: 'conflict';
  noteId: string;
  error: unknown;
}

export interface NotesAutosaveErrorEvent {
  type: 'error';
  noteId: string;
  error: unknown;
}

export type NotesAutosaveEvent =
  | NotesAutosaveSavedEvent
  | NotesAutosaveConflictEvent
  | NotesAutosaveErrorEvent;

/**
 * Persistence port the engine drives. Implemented over the document service
 * facade so the engine itself never touches the SDK client.
 */
export interface NotesAutosavePort {
  saveTitle(noteId: string, title: string): Promise<void>;
  /**
   * Persists note content. `baseVersionId` enables optimistic concurrency: the
   * implementation must fail with a conflict-typed error when the server
   * version no longer matches. Returns the server version id after the save
   * (null when unknown — the next save then skips the concurrency check).
   */
  saveContent(
    noteId: string,
    content: string,
    baseVersionId: string | null,
  ): Promise<string | null>;
  isConflictError(error: unknown): boolean;
}

export interface NotesAutosaveEngineOptions {
  port: NotesAutosavePort;
  /** Quiet period after the last keystroke before a save fires. */
  debounceMs?: number;
  /** Backoff delays between automatic retries; exhausted means status `error`. */
  retryDelaysMs?: number[];
  now?: () => number;
  onEvent?: (event: NotesAutosaveEvent) => void;
}

interface PendingEdits {
  title?: string;
  content?: string;
}

interface NoteState {
  pending: PendingEdits;
  /** The batch currently being saved; kept visible to getPendingEdits so a
   * note reopened mid-save still seeds from the local (authoritative) text. */
  inFlightBatch: PendingEdits | null;
  debounceTimer: ReturnType<typeof setTimeout> | null;
  retryTimer: ReturnType<typeof setTimeout> | null;
  inFlight: Promise<void> | null;
  attempts: number;
  baseVersionId: string | null;
  /** Next content save must skip the concurrency check (user-authorized overwrite). */
  forceNextSave: boolean;
  status: NotesSaveStatus;
  lastSavedAt: number | null;
}

const DEFAULT_DEBOUNCE_MS = 1200;
const DEFAULT_RETRY_DELAYS_MS = [1000, 3000];

export class NotesAutosaveEngine {
  private readonly port: NotesAutosavePort;
  private readonly debounceMs: number;
  private readonly retryDelaysMs: number[];
  private readonly now: () => number;
  private readonly onEvent: ((event: NotesAutosaveEvent) => void) | null;
  private readonly states = new Map<string, NoteState>();
  private readonly baselines = new Map<string, string | null>();
  private readonly listeners = new Set<(snapshot: NotesAutosaveSnapshot) => void>();

  constructor(options: NotesAutosaveEngineOptions) {
    this.port = options.port;
    this.debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;
    this.retryDelaysMs = options.retryDelaysMs ?? DEFAULT_RETRY_DELAYS_MS;
    this.now = options.now ?? Date.now;
    this.onEvent = options.onEvent ?? null;
  }

  subscribe(listener: (snapshot: NotesAutosaveSnapshot) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Records the server version observed when a note was opened. */
  setVersionBaseline(noteId: string, currentVersionId: string | null): void {
    this.baselines.set(noteId, currentVersionId);
    const state = this.states.get(noteId);
    if (state && state.status === 'idle') {
      state.baseVersionId = currentVersionId;
    }
  }

  edit(noteId: string, field: 'title' | 'content', value: string): void {
    const state = this.ensureState(noteId);
    // A fresh user edit re-arms the retry budget and clears a conflict pause.
    state.attempts = 0;
    state.forceNextSave = false;
    state.pending = { ...state.pending, [field]: value };
    if (state.status !== 'saving') {
      state.status = 'dirty';
    }
    this.emit(noteId);
    this.armDebounce(noteId);
  }

  /** Saves pending edits as soon as any in-flight save for the note settles. */
  async flush(noteId: string): Promise<void> {
    const state = this.states.get(noteId);
    if (!state) {
      return;
    }
    this.clearTimers(state);
    if (state.status === 'conflict') {
      // A conflict pause is broken only by overwrite() or a fresh edit(): a
      // mechanical flush (note switch, tab hide, Ctrl+S) must not re-fire the
      // doomed save and re-surface the same conflict.
      return;
    }
    // A manual flush is an explicit retry request: give it a fresh budget.
    state.attempts = 0;
    await this.saveNow(noteId, state);
  }

  async flushAll(options?: { forceConflictSave?: boolean }): Promise<void> {
    const noteIds = Array.from(this.states.keys());
    await Promise.all(
      noteIds.map(async (noteId) => {
        const state = this.states.get(noteId);
        if (options?.forceConflictSave && state?.status === 'conflict' && this.hasPendingEdits(state)) {
          // Teardown path: the workspace is going away, so a conflict-paused
          // buffer would be silently lost. One forced save (concurrency check
          // disabled) preserves the local content server-side.
          await this.overwrite(noteId);
          return;
        }
        await this.flush(noteId);
        if (!options?.forceConflictSave) {
          return;
        }
        // The awaited cycle may have TURNED a saving note into a conflict
        // pause (batch restored into pending); without this re-check the
        // forced save silently never happens and dispose drops the buffer.
        const after = this.states.get(noteId);
        if (after?.status === 'conflict' && this.hasPendingEdits(after)) {
          await this.overwrite(noteId);
        }
      }),
    );
  }

  /** User-authorized overwrite after a conflict: saves with the check disabled. */
  async overwrite(noteId: string): Promise<void> {
    const state = this.states.get(noteId);
    if (!state) {
      return;
    }
    this.clearTimers(state);
    state.attempts = 0;
    state.forceNextSave = true;
    state.status = 'dirty';
    this.emit(noteId);
    await this.saveNow(noteId, state);
  }

  /** Drops all buffered work for a note (used when the note is deleted). */
  forget(noteId: string): void {
    const state = this.states.get(noteId);
    if (!state) {
      return;
    }
    this.clearTimers(state);
    // An in-flight save resolves against a stale state object and no-ops.
    this.states.delete(noteId);
    this.baselines.delete(noteId);
    this.emitSnapshot({
      noteId,
      status: 'idle',
      lastSavedAt: null,
      hasPendingEdits: false,
    });
  }

  getSnapshot(noteId: string): NotesAutosaveSnapshot | null {
    const state = this.states.get(noteId);
    if (!state) {
      return null;
    }
    return this.snapshotOf(noteId, state);
  }

  /**
   * Latest unconfirmed edits for a note (title/content), or null when nothing
   * is buffered. Editors must seed from this when re-opening a note so
   * buffered or failed-save text is never replaced by the stale server copy.
   * While a save is in flight its batch still counts: the editor must not
   * re-seed from the (about-to-be-overwritten) server copy mid-save.
   */
  getPendingEdits(noteId: string): PendingEdits | null {
    const state = this.states.get(noteId);
    if (!state) {
      return null;
    }
    const pending = this.hasPendingEdits(state)
      ? state.pending
      : state.inFlightBatch && (state.inFlightBatch.title !== undefined || state.inFlightBatch.content !== undefined)
        ? state.inFlightBatch
        : null;
    if (!pending) {
      return null;
    }
    const overlay: PendingEdits = {};
    if (pending.title !== undefined) {
      overlay.title = pending.title;
    }
    if (pending.content !== undefined) {
      overlay.content = pending.content;
    }
    return overlay;
  }

  /** True while any note in the workspace still has unconfirmed edits. */
  hasAnyPendingChanges(): boolean {
    for (const state of this.states.values()) {
      if (this.hasPendingEdits(state) || state.inFlight) {
        return true;
      }
    }
    return false;
  }

  hasPendingChanges(noteId: string): boolean {
    const state = this.states.get(noteId);
    if (!state) {
      return false;
    }
    return this.hasPendingEdits(state);
  }

  /** Cancels every timer and drops buffered state. In-flight saves settle
   * against detached states. Listeners are deliberately NOT cleared: the
   * React unmount cleanup defers dispose behind flushAll, and a StrictMode
   * remount may have re-subscribed in the meantime — clearing would kill the
   * live subscription of the surviving hook instance. */
  dispose(): void {
    for (const state of this.states.values()) {
      this.clearTimers(state);
    }
    this.states.clear();
    this.baselines.clear();
  }

  private ensureState(noteId: string): NoteState {
    const existing = this.states.get(noteId);
    if (existing) {
      return existing;
    }
    const created: NoteState = {
      pending: {},
      inFlightBatch: null,
      debounceTimer: null,
      retryTimer: null,
      inFlight: null,
      attempts: 0,
      baseVersionId: this.baselines.get(noteId) ?? null,
      forceNextSave: false,
      status: 'idle',
      lastSavedAt: null,
    };
    this.states.set(noteId, created);
    return created;
  }

  private armDebounce(noteId: string): void {
    const state = this.states.get(noteId);
    if (!state || state.debounceTimer) {
      return;
    }
    state.debounceTimer = setTimeout(() => {
      state.debounceTimer = null;
      void this.saveNow(noteId, state);
    }, this.debounceMs);
  }

  private armRetry(noteId: string, state: NoteState): void {
    const delay = this.retryDelaysMs[Math.min(state.attempts, this.retryDelaysMs.length) - 1];
    if (delay === undefined) {
      return;
    }
    state.retryTimer = setTimeout(() => {
      state.retryTimer = null;
      void this.saveNow(noteId, state);
    }, delay);
  }

  /**
   * Runs one serialized save cycle for the note: takes the pending batch,
   * applies title then content, and classifies the outcome (saved / retry with
   * backoff / conflict pause / final error). Edits made while the batch is in
   * flight are preserved and trigger a trailing debounced save.
   */
  private async saveNow(noteId: string, state: NoteState): Promise<void> {
    if (this.states.get(noteId) !== state) {
      return;
    }
    if (state.inFlight) {
      await state.inFlight;
      if (this.states.get(noteId) !== state) {
        return;
      }
      // The awaited cycle may have ended in a conflict pause (batch restored
      // into pending). Re-running it now would repeat the doomed save against
      // the stale baseline and emit a second conflict event; the pause is
      // broken only by overwrite() or a fresh edit().
      if (state.status === 'conflict') {
        return;
      }
      // The finished cycle re-arms the debounce for edits made mid-flight;
      // a flush() call needs those saved now instead.
      if (this.hasPendingEdits(state)) {
        await this.runCycle(noteId, state);
      }
      return;
    }
    if (!this.hasPendingEdits(state)) {
      return;
    }
    await this.runCycle(noteId, state);
  }

  private async runCycle(noteId: string, state: NoteState): Promise<void> {
    const batch = state.pending;
    state.pending = {};
    state.inFlightBatch = batch;
    state.status = 'saving';
    this.emit(noteId);

    const cycle = (async () => {
      let savedTitle = false;
      let savedContent = false;
      try {
        if (batch.title !== undefined) {
          await this.port.saveTitle(noteId, batch.title);
          savedTitle = true;
        }
        if (batch.content !== undefined) {
          const baseVersionId = state.forceNextSave ? null : state.baseVersionId;
          const versionId = await this.port.saveContent(noteId, batch.content, baseVersionId);
          state.forceNextSave = false;
          if (versionId) {
            state.baseVersionId = versionId;
          }
          savedContent = true;
        }
      } catch (error) {
        return { ok: false as const, error, savedTitle, savedContent };
      }
      return { ok: true as const, error: null, savedTitle, savedContent };
    })();

    state.inFlight = cycle.then(() => undefined);
    const outcome = await cycle;
    state.inFlight = null;
    // The cycle settled: on success the server copy now matches the batch, on
    // failure the batch was restored into pending — either way the shadow has
    // served its purpose for mid-flight reopen seeding.
    state.inFlightBatch = null;

    // A forget() (or dispose) during the cycle invalidates this state object.
    if (this.states.get(noteId) !== state) {
      return;
    }

    if (outcome.ok) {
      state.attempts = 0;
      state.status = 'saved';
      state.lastSavedAt = this.now();
      this.emit(noteId);
      this.onEvent?.({
        type: 'saved',
        noteId,
        savedTitle: outcome.savedTitle,
        savedContent: outcome.savedContent,
      });
      if (this.hasPendingEdits(state)) {
        this.armDebounce(noteId);
      }
      return;
    }

    // Edits from the failed batch stay queued: local input is never dropped.
    state.pending = { ...batch, ...state.pending };

    if (this.port.isConflictError(outcome.error)) {
      // Another editor saved newer content. Auto-save pauses until the user
      // overwrites or edits again; the conflict event fires once per pause.
      // Kill the debounce timer a mid-flight edit() armed: without this it
      // would fire saveNow against the stale baseline and emit a duplicate
      // conflict event. The buffered edits stay queued for overwrite() or a
      // fresh edit().
      this.clearTimers(state);
      state.status = 'conflict';
      this.emit(noteId);
      this.onEvent?.({ type: 'conflict', noteId, error: outcome.error });
      return;
    }

    if (state.attempts < this.retryDelaysMs.length) {
      state.attempts += 1;
      state.status = 'dirty';
      this.emit(noteId);
      this.armRetry(noteId, state);
      return;
    }

    state.status = 'error';
    this.emit(noteId);
    this.onEvent?.({ type: 'error', noteId, error: outcome.error });
  }

  private hasPendingEdits(state: NoteState): boolean {
    return state.pending.title !== undefined || state.pending.content !== undefined;
  }

  private clearTimers(state: NoteState): void {
    if (state.debounceTimer) {
      clearTimeout(state.debounceTimer);
      state.debounceTimer = null;
    }
    if (state.retryTimer) {
      clearTimeout(state.retryTimer);
      state.retryTimer = null;
    }
  }

  private snapshotOf(noteId: string, state: NoteState): NotesAutosaveSnapshot {
    return {
      noteId,
      status: state.status,
      lastSavedAt: state.lastSavedAt,
      hasPendingEdits: this.hasPendingEdits(state),
    };
  }

  private emit(noteId: string): void {
    const state = this.states.get(noteId);
    if (!state) {
      return;
    }
    this.emitSnapshot(this.snapshotOf(noteId, state));
  }

  private emitSnapshot(snapshot: NotesAutosaveSnapshot): void {
    for (const listener of this.listeners) {
      listener(snapshot);
    }
  }
}
