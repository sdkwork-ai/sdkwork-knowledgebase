import { useCallback, useEffect, useRef, type Dispatch, type SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';
import type { DocumentMeta } from '../services/document';
import { DocumentService } from '../services/document';
import { toastKnowledgebaseError } from '../components/ui/toastKnowledgebaseError';
import { isDocumentConflictError } from '../services/documentConflict';

const SAVE_DEBOUNCE_MS = 800;

interface PendingSave {
  content: string;
  baseVersionId: string | null;
}

interface UseKnowledgeBaseDocumentPersistenceOptions {
  activeDoc: DocumentMeta | null;
  docs: unknown[];
  loadingDocs: boolean;
  setOpenDocs: Dispatch<SetStateAction<DocumentMeta[]>>;
  setActiveDoc: Dispatch<SetStateAction<DocumentMeta | null>>;
  setDocContent: Dispatch<SetStateAction<string>>;
}

export function useKnowledgeBaseDocumentPersistence({
  activeDoc,
  docs,
  loadingDocs,
  setOpenDocs,
  setActiveDoc,
  setDocContent,
}: UseKnowledgeBaseDocumentPersistenceOptions) {
  const { t } = useTranslation(['kb', 'common', 'errors']);
  const pendingByDocRef = useRef<Map<string, PendingSave>>(new Map());
  const baseVersionIdRef = useRef<Map<string, string | null>>(new Map());
  const timersByDocRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  const saveInFlightRef = useRef<Map<string, Promise<void>>>(new Map());
  // Documents whose latest save hit a conflict: the queued save stays pending
  // (needing user action) and the conflict toast is surfaced only once per
  // conflict instead of on every silent retry.
  const conflictDocsRef = useRef<Set<string>>(new Set());
  const activeDocIdRef = useRef<string | null>(null);

  const flushDocumentSave = useCallback(async (docId: string) => {
    const timer = timersByDocRef.current.get(docId);
    if (timer) {
      clearTimeout(timer);
      timersByDocRef.current.delete(docId);
    }

    const pending = pendingByDocRef.current.get(docId);
    if (pending === undefined) {
      return;
    }
    pendingByDocRef.current.delete(docId);

    const existing = saveInFlightRef.current.get(docId);
    if (existing) {
      await existing.catch(() => undefined);
    }

    const savePromise = (async () => {
      try {
        const result = await DocumentService.saveDocumentContent(docId, pending.content, {
          baseVersionId: pending.baseVersionId,
        });
        conflictDocsRef.current.delete(docId);
        if (result.currentVersionId) {
          baseVersionIdRef.current.set(docId, result.currentVersionId);
        }
      } catch (error) {
        // A conflict means another editor saved newer content. Keep this save
        // pending so the user's input is not silently lost, but never auto-retry:
        // the user must decide between reviewing the remote version or forcing an
        // overwrite. The conflict is toasted once (per conflict) and the document
        // stays marked in `conflictDocsRef` until the user edits again.
        if (isDocumentConflict(error)) {
          // Keep the content queued (needing user action) but do not re-arm a
          // timer: the save only refires on an explicit flush or a new edit.
          pendingByDocRef.current.set(docId, pending);
          if (!conflictDocsRef.current.has(docId)) {
            conflictDocsRef.current.add(docId);
            toastKnowledgebaseError(error, t);
          } else {
            console.error('[useKnowledgeBaseDocumentPersistence] save conflict still unresolved for document', docId, error);
          }
          return;
        }
        toastKnowledgebaseError(error, t);
        console.error('[useKnowledgeBaseDocumentPersistence] document save failed', docId, error);
      }
    })();

    saveInFlightRef.current.set(docId, savePromise);
    try {
      await savePromise;
    } finally {
      if (saveInFlightRef.current.get(docId) === savePromise) {
        saveInFlightRef.current.delete(docId);
      }
    }
  }, [t]);

  const scheduleDocumentSave = useCallback((docId: string, content: string) => {
    // A new user edit re-arms the save attempt: clear the conflict mark so a
    // fresh conflict can surface its toast again.
    conflictDocsRef.current.delete(docId);
    pendingByDocRef.current.set(docId, {
      content,
      baseVersionId: baseVersionIdRef.current.get(docId) ?? null,
    });
    const existingTimer = timersByDocRef.current.get(docId);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }
    timersByDocRef.current.set(
      docId,
      setTimeout(() => {
        void flushDocumentSave(docId);
      }, SAVE_DEBOUNCE_MS),
    );
  }, [flushDocumentSave]);

  const flushAllPendingSaves = useCallback(async () => {
    const docIds = new Set<string>([
      ...pendingByDocRef.current.keys(),
      ...timersByDocRef.current.keys(),
    ]);
    await Promise.all(Array.from(docIds, (docId) => flushDocumentSave(docId)));
  }, [flushDocumentSave]);

  const handleContentChange = useCallback((newContent: string) => {
    if (!activeDoc) {
      return;
    }
    scheduleDocumentSave(activeDoc.id, newContent);
  }, [activeDoc, scheduleDocumentSave]);

  useEffect(() => {
    const previousDocId = activeDocIdRef.current;
    const nextDocId = activeDoc?.id ?? null;
    if (previousDocId && previousDocId !== nextDocId) {
      void flushDocumentSave(previousDocId);
    }
    activeDocIdRef.current = nextDocId;
  }, [activeDoc, flushDocumentSave]);

  useEffect(() => {
    const flushAllPendingNow = () => {
      for (const docId of pendingByDocRef.current.keys()) {
        const timer = timersByDocRef.current.get(docId);
        if (timer) {
          clearTimeout(timer);
          timersByDocRef.current.delete(docId);
        }
        const pending = pendingByDocRef.current.get(docId);
        if (pending !== undefined) {
          pendingByDocRef.current.delete(docId);
          void DocumentService.saveDocumentContent(docId, pending.content).catch(() => undefined);
        }
      }
    };
    const handleBeforeUnload = () => {
      flushAllPendingNow();
    };
    // visibilitychange→hidden fires while the page is still alive, so the
    // multi-request ingest save can actually complete — unlike a beforeunload
    // flush, which browsers routinely abort mid-chain.
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        flushAllPendingNow();
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      window.removeEventListener('beforeunload', handleBeforeUnload);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      void flushAllPendingSaves();
    };
  }, [flushAllPendingSaves]);

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
    const traverse = (items: any[]) => {
      items.forEach((item) => {
        flatIds.add(item.id);
        if (item.type === 'folder' && item.children) {
          traverse(item.children);
        }
      });
    };
    traverse(docs as any[]);

    setOpenDocs((prev) => {
      const filtered = prev.filter((doc) => flatIds.has(doc.id));
      if (filtered.length !== prev.length) {
        return filtered;
      }
      return prev;
    });

    if (activeDoc && !flatIds.has(activeDoc.id)) {
      void flushDocumentSave(activeDoc.id).finally(() => {
        setActiveDoc(null);
        setDocContent('');
      });
    }
  }, [
    activeDoc,
    docs,
    flushDocumentSave,
    loadingDocs,
    setActiveDoc,
    setDocContent,
    setOpenDocs,
  ]);

  return { handleContentChange };
}

function isDocumentConflict(error: unknown): boolean {
  return isDocumentConflictError(error);
}
