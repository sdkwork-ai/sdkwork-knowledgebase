import { useCallback, useEffect, useRef, useState } from 'react';
import {
  NotesAutosaveEngine,
  type NotesAutosaveEvent,
  type NotesAutosavePort,
  type NotesAutosaveSnapshot,
  type NotesSaveStatus,
} from './notesAutosaveEngine';

export type { NotesSaveStatus } from './notesAutosaveEngine';

export interface UseNotesAutosaveOptions {
  activeNoteId: string | null;
  port: NotesAutosavePortWithBaseline;
  /** Receives engine lifecycle events (conflict pause, final save failure). */
  onEvent?: (event: NotesAutosaveEvent) => void;
}

export interface NotesAutosavePortWithBaseline extends NotesAutosavePort {
  /** Loads the server version id observed when a note is opened, arming
   * optimistic-concurrency checks for subsequent saves. */
  loadVersionBaseline?: (noteId: string) => Promise<string | null>;
}

export interface UseNotesAutosaveResult {
  status: NotesSaveStatus;
  lastSavedAt: number | null;
  hasPendingEdits: boolean;
  editTitle: (noteId: string, title: string) => void;
  editContent: (noteId: string, content: string) => void;
  /** Manual save (Ctrl/Cmd+S, retry button): flushes the active note now. */
  flushActive: () => Promise<void>;
  /** Conflict recovery: overwrite the server copy with local content. */
  overwriteActive: () => Promise<void>;
  /** Drops buffered edits for a note (deletion path). */
  forget: (noteId: string) => void;
  /**
   * Latest unconfirmed edits for a note; content loading must overlay these
   * over the server copy so buffered/failed-save text is never hidden.
   */
  getPendingEdits: (
    noteId: string,
  ) => { title?: string; content?: string } | null;
}

const IDLE_SNAPSHOT: NotesAutosaveSnapshot = {
  noteId: '',
  status: 'idle',
  lastSavedAt: null,
  hasPendingEdits: false,
};

/**
 * Binds the notes autosave engine to React for the active note, and wires the
 * exit-path flushes that guarantee buffered edits survive: switching notes,
 * hiding the tab (visibilitychange), closing/reloading (beforeunload with an
 * unsaved-changes prompt only when edits are still unconfirmed), and the
 * explicit Ctrl/Cmd+S shortcut.
 *
 * The engine is created once per hook lifetime and forwards to the latest
 * port through refs, so a re-rendered caller never resets the save queue.
 */
export function useNotesAutosave(options: UseNotesAutosaveOptions): UseNotesAutosaveResult {
  const { activeNoteId } = options;

  const portRef = useRef(options.port);
  portRef.current = options.port;
  const onEventRef = useRef(options.onEvent);
  onEventRef.current = options.onEvent;

  const [engine] = useState(
    () =>
      new NotesAutosaveEngine({
        port: {
          saveTitle: (noteId, title) => portRef.current.saveTitle(noteId, title),
          saveContent: (noteId, content, baseVersionId) =>
            portRef.current.saveContent(noteId, content, baseVersionId),
          isConflictError: (error) => portRef.current.isConflictError(error),
        },
        onEvent: (event) => onEventRef.current?.(event),
      }),
  );

  const [snapshot, setSnapshot] = useState<NotesAutosaveSnapshot>(IDLE_SNAPSHOT);
  const unsavedWorkRef = useRef(false);

  useEffect(() => {
    return engine.subscribe((next) => {
      // Tracked across ALL notes: closing the tab with a background note in
      // error/conflict (buffer retained) must still warn, not just the active
      // note's snapshot.
      unsavedWorkRef.current = engine.hasAnyPendingChanges();
      if (next.noteId === activeNoteId) {
        setSnapshot(next);
      }
    });
  }, [engine, activeNoteId]);

  // Mirror the active note's current state whenever the selection changes, so
  // the status pill never shows the previous note's save state. The
  // beforeunload marker spans ALL notes (background notes paused in
  // error/conflict keep their buffers), so it is recomputed from the engine.
  useEffect(() => {
    const current = activeNoteId ? engine.getSnapshot(activeNoteId) : null;
    setSnapshot(current ?? { ...IDLE_SNAPSHOT, noteId: activeNoteId ?? '' });
    unsavedWorkRef.current = engine.hasAnyPendingChanges();
  }, [engine, activeNoteId]);

  // Open-time baseline: record the server version the note was opened with so
  // subsequent saves can detect concurrent edits from other windows.
  useEffect(() => {
    if (!activeNoteId) {
      return;
    }
    const loadBaseline = portRef.current.loadVersionBaseline;
    if (!loadBaseline) {
      return;
    }
    let cancelled = false;
    loadBaseline(activeNoteId)
      .then((versionId) => {
        if (!cancelled) {
          engine.setVersionBaseline(activeNoteId, versionId ?? null);
        }
      })
      .catch(() => {
        // No baseline means the next save simply skips the conflict check.
        if (!cancelled) {
          engine.setVersionBaseline(activeNoteId, null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [engine, activeNoteId]);

  const flushActiveRef = useRef<() => Promise<void>>(async () => undefined);
  const flushActive = useCallback(async () => {
    if (activeNoteId) {
      await engine.flush(activeNoteId);
    }
  }, [engine, activeNoteId]);
  flushActiveRef.current = flushActive;

  // Switching notes flushes the previous note before its editor unmounts.
  const previousNoteIdRef = useRef<string | null>(null);
  useEffect(() => {
    const previousNoteId = previousNoteIdRef.current;
    previousNoteIdRef.current = activeNoteId;
    if (previousNoteId && previousNoteId !== activeNoteId) {
      void engine.flush(previousNoteId).catch(() => undefined);
    }
  }, [engine, activeNoteId]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        // Hide covers mobile backgrounding: flush every note with buffered
        // work (conflict-paused notes are skipped inside the engine).
        void engine.flushAll().catch(() => undefined);
      }
    };
    const handleBeforeUnload = (event: BeforeUnloadEvent) => {
      void flushActiveRef.current().catch(() => undefined);
      // Prompt only when edits are still unconfirmed; a healthy autosave never
      // blocks closing the tab.
      if (unsavedWorkRef.current) {
        event.preventDefault();
        event.returnValue = '';
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
        event.preventDefault();
        void flushActiveRef.current().catch(() => undefined);
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('beforeunload', handleBeforeUnload);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('beforeunload', handleBeforeUnload);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // Unmount flush: the workspace is leaving, save everything buffered —
  // including one forced save for conflict-paused notes, whose buffered edits
  // would otherwise be silently lost (the pill's overwrite choice is gone).
  // Dispose only after the flush settles so an in-flight save can still run
  // its trailing cycle for mid-flight edits.
  useEffect(
    () => () => {
      void engine
        .flushAll({ forceConflictSave: true })
        .catch(() => undefined)
        .finally(() => engine.dispose());
    },
    [engine],
  );

  const editTitle = useCallback(
    (noteId: string, title: string) => {
      engine.edit(noteId, 'title', title);
    },
    [engine],
  );

  const editContent = useCallback(
    (noteId: string, content: string) => {
      engine.edit(noteId, 'content', content);
    },
    [engine],
  );

  const overwriteActive = useCallback(async () => {
    if (activeNoteId) {
      await engine.overwrite(activeNoteId);
    }
  }, [engine, activeNoteId]);

  const forget = useCallback(
    (noteId: string) => {
      engine.forget(noteId);
    },
    [engine],
  );

  const getPendingEdits = useCallback(
    (noteId: string) => engine.getPendingEdits(noteId),
    [engine],
  );

  return {
    status: snapshot.status,
    lastSavedAt: snapshot.lastSavedAt,
    hasPendingEdits: snapshot.hasPendingEdits,
    editTitle,
    editContent,
    flushActive,
    overwriteActive,
    forget,
    getPendingEdits,
  };
}
