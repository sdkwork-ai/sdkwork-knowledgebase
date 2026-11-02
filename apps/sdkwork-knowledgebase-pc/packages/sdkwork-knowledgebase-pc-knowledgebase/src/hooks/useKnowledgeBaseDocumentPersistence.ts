import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';
import type { DocumentMeta } from '../services/document';
import { DocumentService } from '../services/document';
import { isDocumentConflictError } from '../services/documentConflict';
import { toastKnowledgebaseError } from '../components/ui/toastKnowledgebaseError';
import {
  NotesAutosaveEngine,
  type NotesAutosaveEvent,
  type NotesAutosavePort,
  type NotesAutosaveSnapshot,
  type NotesSaveStatus,
} from './notesAutosaveEngine';

/** Save lifecycle of the active document, surfaced by the editor chrome. */
export interface KnowledgeDocumentSaveStatus {
  docId: string | null;
  state: NotesSaveStatus;
  /** Epoch ms of the last server-confirmed save; null before the first one. */
  savedAt: number | null;
}

/** Buffered (not yet server-confirmed) edits for one document. */
export interface KnowledgeDocumentPendingEdits {
  title?: string;
  content?: string;
}

interface UseKnowledgeBaseDocumentPersistenceOptions {
  activeDoc: DocumentMeta | null;
  docs: unknown[];
  loadingDocs: boolean;
  setOpenDocs: Dispatch<SetStateAction<DocumentMeta[]>>;
  setActiveDoc: Dispatch<SetStateAction<DocumentMeta | null>>;
  setDocContent: Dispatch<SetStateAction<string>>;
}

interface UseKnowledgeBaseDocumentPersistenceResult {
  /** Routes a title edit into the debounced save pipeline. */
  handleTitleChange: (docId: string, title: string) => void;
  /** Routes a body edit into the debounced save pipeline. */
  handleContentChange: (content: string) => void;
  /** Save status of the active document (idle when nothing is selected). */
  saveStatus: KnowledgeDocumentSaveStatus;
  /** True while the active document has edits the server has not confirmed. */
  hasPendingEdits: boolean;
  /** Flush the active document now (manual save / retry, also Ctrl/Cmd+S). */
  saveNow: () => Promise<void>;
  /** Conflict recovery: overwrite the server copy with the local content. */
  overwriteActive: () => Promise<void>;
  /** Latest unconfirmed edits for a document; content loading overlays these
   * over the server copy so buffered or failed-save text is never hidden. */
  getPendingEdits: (docId: string) => KnowledgeDocumentPendingEdits | null;
  /** Drops buffered edits for a document (deletion path). */
  forget: (docId: string) => void;
}

const IDLE_STATUS: KnowledgeDocumentSaveStatus = {
  docId: null,
  state: 'idle',
  savedAt: null,
};

/**
 * Debounced, serialized document persistence for the knowledge editor.
 *
 * Autosave runs through the shared `NotesAutosaveEngine` (the same engine the
 * notes workspace uses): edits coalesce behind a debounce, saves serialize per
 * document, transient failures retry with bounded backoff, a concurrent edit
 * pauses auto-save until the user overwrites, and switching documents, hiding
 * the tab, or leaving the workspace always flushes the buffer first.
 *
 * The server version observed when a document is opened arms optimistic
 * concurrency, so a stale editor is told about the newer copy instead of
 * silently overwriting it.
 */
export function useKnowledgeBaseDocumentPersistence({
  activeDoc,
  docs,
  loadingDocs,
  setOpenDocs,
  setActiveDoc,
  setDocContent,
}: UseKnowledgeBaseDocumentPersistenceOptions): UseKnowledgeBaseDocumentPersistenceResult {
  const { t } = useTranslation(['kb', 'common', 'errors']);
  const activeDocId = activeDoc?.id ?? null;

  const onEventRef = useRef<((event: NotesAutosaveEvent) => void) | null>(null);
  onEventRef.current = (event) => {
    // Conflicts need a decision from the user, so they also surface as a
    // toast; ordinary failures stay on the status pill with its retry action.
    if (event.type === 'conflict') {
      toastKnowledgebaseError(event.error, t);
    }
  };

  const [engine] = useState(
    () =>
      new NotesAutosaveEngine({
        port: KNOWLEDGE_DOCUMENT_AUTOSAVE_PORT,
        onEvent: (event) => onEventRef.current?.(event),
      }),
  );

  const [snapshot, setSnapshot] = useState<NotesAutosaveSnapshot | null>(null);
  const unsavedWorkRef = useRef(false);

  useEffect(() => {
    return engine.subscribe((next) => {
      // Tracked across ALL documents: leaving the workspace with a background
      // document still in error/conflict (buffer retained) must warn too.
      unsavedWorkRef.current = engine.hasAnyPendingChanges();
      if (next.noteId === activeDocId) {
        setSnapshot(next);
      }
    });
  }, [engine, activeDocId]);

  // Mirror the active document's current state whenever the selection changes,
  // so the status pill never shows the previous document's save state.
  useEffect(() => {
    const current = activeDocId ? engine.getSnapshot(activeDocId) : null;
    setSnapshot(current);
    unsavedWorkRef.current = engine.hasAnyPendingChanges();
  }, [engine, activeDocId]);

  // Open-time baseline: record the server version the document was opened with
  // so later saves detect concurrent edits from other windows.
  useEffect(() => {
    if (!activeDocId) {
      return undefined;
    }
    let cancelled = false;
    DocumentService.getDocumentSaveContext(activeDocId)
      .then((context) => {
        if (!cancelled) {
          engine.setVersionBaseline(activeDocId, context.currentVersionId);
        }
      })
      .catch(() => {
        // No baseline means the next save simply skips the conflict check.
        if (!cancelled) engine.setVersionBaseline(activeDocId, null);
      });
    return () => {
      cancelled = true;
    };
  }, [engine, activeDocId]);

  const flushActive = useCallback(async () => {
    if (activeDocId) {
      await engine.flush(activeDocId);
    }
  }, [engine, activeDocId]);

  const saveNow = useCallback(async () => {
    await flushActive();
  }, [flushActive]);

  const overwriteActive = useCallback(async () => {
    if (activeDocId) {
      await engine.overwrite(activeDocId);
    }
  }, [engine, activeDocId]);

  const flushActiveRef = useRef(flushActive);
  flushActiveRef.current = flushActive;

  // Switching documents flushes the previous one before its editor unmounts.
  const previousDocIdRef = useRef<string | null>(null);
  useEffect(() => {
    const previousDocId = previousDocIdRef.current;
    previousDocIdRef.current = activeDocId;
    if (previousDocId && previousDocId !== activeDocId) {
      void engine.flush(previousDocId).catch(() => undefined);
    }
  }, [engine, activeDocId]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        // Hide covers mobile backgrounding: flush every document with buffered
        // work (conflict-paused documents are skipped inside the engine).
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
    const handleOnline = () => {
      // Reconnected: drain every buffered edit instead of waiting for the next
      // keystroke to retry.
      void engine.flushAll().catch(() => undefined);
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('beforeunload', handleBeforeUnload);
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('online', handleOnline);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('beforeunload', handleBeforeUnload);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('online', handleOnline);
    };
  }, [engine]);

  // Unmount flush: the workspace is leaving, so save everything buffered —
  // including one forced save for conflict-paused documents, whose buffered
  // edits would otherwise be silently lost (the pill's overwrite choice is
  // gone). Dispose only after the flush settles so an in-flight save can still
  // run its trailing cycle for mid-flight edits.
  useEffect(
    () => () => {
      void engine
        .flushAll({ forceConflictSave: true })
        .catch(() => undefined)
        .finally(() => engine.dispose());
    },
    [engine],
  );

  const handleTitleChange = useCallback(
    (docId: string, title: string) => {
      engine.edit(docId, 'title', title);
    },
    [engine],
  );

  const handleContentChange = useCallback(
    (content: string) => {
      if (activeDocId) {
        engine.edit(activeDocId, 'content', content);
      }
    },
    [activeDocId, engine],
  );

  const getPendingEdits = useCallback(
    (docId: string) => engine.getPendingEdits(docId),
    [engine],
  );

  const forget = useCallback(
    (docId: string) => {
      engine.forget(docId);
    },
    [engine],
  );

  // Documents that vanish from the tree (deleted elsewhere) drop out of the
  // open-tab list, and the active one is flushed before the editor is cleared.
  useEffect(() => {
    if (!docs || docs.length === 0) {
      if (!loadingDocs) {
        setOpenDocs([]);
        setActiveDoc(null);
        setDocContent('');
      }
      return;
    }

    const flatIds = new Set<string>();
    const traverse = (items: unknown[]) => {
      for (const item of items) {
        const node = item as { id?: string; type?: string; children?: unknown[] };
        if (typeof node.id === 'string') flatIds.add(node.id);
        if (node.type === 'folder' && Array.isArray(node.children)) {
          traverse(node.children);
        }
      }
    };
    traverse(docs);

    setOpenDocs((prev) => {
      const filtered = prev.filter((doc) => flatIds.has(doc.id));
      return filtered.length === prev.length ? prev : filtered;
    });

    if (activeDoc && !flatIds.has(activeDoc.id)) {
      void engine.flush(activeDoc.id).finally(() => {
        setActiveDoc(null);
        setDocContent('');
      });
    }
  }, [
    activeDoc,
    docs,
    engine,
    loadingDocs,
    setActiveDoc,
    setDocContent,
    setOpenDocs,
  ]);

  const activeSnapshot = activeDocId ? snapshot : null;
  const saveStatus: KnowledgeDocumentSaveStatus = activeSnapshot
    ? {
      docId: activeSnapshot.noteId,
      state: activeSnapshot.status,
      savedAt: activeSnapshot.lastSavedAt,
    }
    : IDLE_STATUS;

  return {
    handleTitleChange,
    handleContentChange,
    saveStatus,
    hasPendingEdits: activeSnapshot?.hasPendingEdits ?? false,
    saveNow,
    overwriteActive,
    getPendingEdits,
    forget,
  };
}

/**
 * Persistence port over the document service facade. Stateless, so one shared
 * instance serves every editor mount without retaining per-document state.
 */
const KNOWLEDGE_DOCUMENT_AUTOSAVE_PORT: NotesAutosavePort = {
  async saveTitle(docId, title) {
    // No `kbId`: a kbId that differs from the document's resolved space is a
    // cross-space move, not a rename.
    await DocumentService.updateDocument(docId, { title });
  },
  async saveContent(docId, content, baseVersionId) {
    const result = await DocumentService.saveDocumentContent(docId, content, {
      baseVersionId,
    });
    return result.currentVersionId;
  },
  isConflictError: isDocumentConflictError,
};
