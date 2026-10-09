import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  AlertTriangle,
  Check,
  CloudOff,
  FileText,
  Loader2,
  Plus,
  RotateCcw,
  Search,
  Trash2,
} from 'lucide-react';
import type { ErrorTranslateFn } from 'sdkwork-knowledgebase-pc-core';

import { DocumentService } from './services/document';
import { getDocumentContent } from './services/knowledgebaseDocumentApiBridge';
import { listSpaceNotes, type KnowledgeNoteSummary } from './services/knowledgeNotesListService';
import { isDocumentConflictError } from './services/documentConflict';
import type { NotesAutosaveEvent } from './hooks/notesAutosaveEngine';
import {
  useNotesAutosave,
  type NotesAutosavePortWithBaseline,
  type NotesSaveStatus,
} from './hooks/useNotesAutosave';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';
import { toast } from './components/ui/toast-manager';
import { formatNoteSavedTime } from './utils/noteSavedTimeFormatter';
import { TiptapEditor } from './TiptapEditor';

interface NoteEditorState {
  id: string;
  title: string;
  content: string;
}

type NoteContentLoadState = 'empty' | 'loading' | 'ready' | 'error';

/**
 * Full notes workspace: notes list on the left, the shared TiptapEditor on the
 * right (Notion / Tencent IMA style). Notes are the space's knowledge documents
 * listed through `documents.list`, so notes created here and notes created from
 * the knowledge base view ("新的笔记") are the same documents and stay in sync.
 *
 * Writing → auto-save runs through `useNotesAutosave`: edits coalesce behind a
 * debounce, saves serialize per note, transient failures retry with backoff,
 * and the header pill reports 保存中 / 已保存 / 保存失败 / 冲突 at all times.
 */
export function NotesWorkspace() {
  const { t, i18n } = useTranslation('kb');
  const [kbId, setKbId] = useState<string>('');
  const [kbResolved, setKbResolved] = useState(false);
  const [notes, setNotes] = useState<KnowledgeNoteSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchText, setSearchText] = useState('');
  const [activeId, setActiveId] = useState<string | null>(null);
  const [editor, setEditor] = useState<NoteEditorState | null>(null);
  const [contentLoadState, setContentLoadState] = useState<NoteContentLoadState>('empty');
  const [contentLoadRetryToken, setContentLoadRetryToken] = useState(0);
  const [focusTitleNoteId, setFocusTitleNoteId] = useState<string | null>(null);

  // Resolve the working knowledge base: the stored active KB first, then the
  // first personal KB, so the workspace is self-sufficient on first launch.
  useEffect(() => {
    let cancelled = false;
    let resolvedKbId: string | null = null;
    (async () => {
      try {
        const stored = window.localStorage.getItem('app-active-kb');
        if (stored) {
          try {
            const parsed = JSON.parse(stored) as { id?: string };
            if (parsed.id && parsed.id.trim()) {
              resolvedKbId = parsed.id.trim();
              if (!cancelled) setKbId(resolvedKbId);
              return;
            }
          } catch {
            // Fall through to KB discovery.
          }
        }
        const grouped = await DocumentService.getKnowledgeBases();
        const first = grouped.personal[0] ?? grouped.team[0] ?? grouped.public[0];
        if (first && !cancelled) {
          resolvedKbId = first.id;
          setKbId(first.id);
        }
      } catch (error) {
        toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
      } finally {
        if (!cancelled) {
          setKbResolved(true);
          // KB resolution failed (no working KB): end the list loading state
          // so the workspace shows its empty state instead of a forever
          // spinner. The `resolvedKbId` local beats the stale `kbId` closure.
          if (!resolvedKbId) {
            setLoading(false);
          }
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [t]);

  const refreshNotes = useCallback(async (selectId?: string | null) => {
    if (!kbId) return;
    setLoading(true);
    try {
      const listed = await listSpaceNotes(kbId);
      setNotes(listed);
      setActiveId((current) => {
        const target = selectId ?? current;
        if (target && listed.some((note) => note.id === target)) return target;
        return listed[0]?.id ?? null;
      });
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
    } finally {
      setLoading(false);
    }
  }, [kbId, t]);

  useEffect(() => {
    if (kbResolved && kbId) {
      void refreshNotes();
    }
  }, [kbResolved, kbId, refreshNotes]);

  // Latest notes snapshot for reads that must not re-trigger the editor reload
  // effect: list refetches would otherwise clobber in-progress edits mid-typing.
  const notesRef = useRef<KnowledgeNoteSummary[]>([]);
  notesRef.current = notes;

  // Autosave port over the document service facade. Stable identity: the save
  // queue behind it must never be rebuilt by a re-render.
  const autosavePort = useMemo<NotesAutosavePortWithBaseline>(
    () => ({
      saveTitle: async (noteId, title) => {
        // No `kbId`: the bridge treats a kbId that differs from the document's
        // resolved space as a cross-space move, which would swallow the rename.
        // Manual notes always rename in place via `documents.update`.
        await DocumentService.updateDocument(noteId, { title });
        setNotes((prev) =>
          prev.map((note) => (note.id === noteId ? { ...note, title } : note)),
        );
      },
      saveContent: async (noteId, content, baseVersionId) => {
        const result = await DocumentService.saveDocumentContent(noteId, content, {
          baseVersionId,
        });
        return result.currentVersionId;
      },
      isConflictError: isDocumentConflictError,
      loadVersionBaseline: async (noteId) => {
        try {
          const context = await DocumentService.getDocumentSaveContext(noteId);
          return context.currentVersionId;
        } catch {
          return null;
        }
      },
    }),
    [],
  );

  // Conflicts need a decision from the user, so they also surface as a toast;
  // ordinary save failures stay on the status pill (with its retry action).
  const handleAutosaveEvent = useCallback(
    (event: NotesAutosaveEvent) => {
      if (event.type === 'conflict') {
        toast.error(t('notesConflictToast'));
      }
    },
    [t],
  );

  const autosave = useNotesAutosave({
    activeNoteId: activeId,
    port: autosavePort,
    onEvent: handleAutosaveEvent,
  });

  // Translator held in a ref so error toasts inside effects never depend on
  // the `t` identity (a locale switch must not refetch note content).
  const translatorRef = useRef(t);
  translatorRef.current = t;

  // Latest editor identity for change guards: async continuations (AI insert,
  // image upload) may resolve after the user switched notes and must never
  // write another note's content into the autosave queue.
  const editorIdRef = useRef<string | null>(null);
  editorIdRef.current = editor?.id ?? null;

  // Stable engine accessor: pending edits must be readable while loading a
  // note without the loader depending on the per-render autosave object.
  const { getPendingEdits } = autosave;

  // Load the selected note into the shared editor. Depends only on `activeId`
  // (plus the explicit retry token): the title is read from the latest notes
  // snapshot via ref, so list refetches never revert in-progress edits.
  useEffect(() => {
    let cancelled = false;
    if (!activeId) {
      setEditor(null);
      setContentLoadState('empty');
      return;
    }
    setContentLoadState('loading');
    const title = notesRef.current.find((note) => note.id === activeId)?.title ?? '';
    (async () => {
      try {
        const content = await getDocumentContent(activeId);
        if (cancelled) return;
        // Buffered edits (a failed or still-queued autosave) win over the
        // server copy: re-opening a dirty note must show what the user typed.
        const pending = getPendingEdits(activeId);
        setEditor({
          id: activeId,
          title: pending?.title ?? title,
          content: pending?.content ?? content,
        });
        setContentLoadState('ready');
      } catch (error) {
        if (cancelled) return;
        setContentLoadState('error');
        toastKnowledgebaseError(error, translatorRef.current as unknown as ErrorTranslateFn);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [activeId, contentLoadRetryToken, getPendingEdits]);

  // The "focus the title" affordance applies to the first mount of a freshly
  // created note only; afterwards it must not steal the caret back on every
  // reselection.
  useEffect(() => {
    if (contentLoadState === 'ready' && focusTitleNoteId !== null) {
      setFocusTitleNoteId(null);
    }
  }, [contentLoadState, focusTitleNoteId]);

  const handleTitleChange = useCallback(
    (value: string) => {
      if (activeId && editorIdRef.current === activeId) {
        setEditor((prev) => (prev && prev.id === activeId ? { ...prev, title: value } : prev));
        autosave.editTitle(activeId, value);
      }
    },
    [activeId, autosave],
  );

  const handleContentChange = useCallback(
    (content: string) => {
      if (activeId && editorIdRef.current === activeId) {
        setEditor((prev) => (prev && prev.id === activeId ? { ...prev, content } : prev));
        autosave.editContent(activeId, content);
      }
    },
    [activeId, autosave],
  );

  const handleCreateNote = async () => {
    if (!kbId) return;
    try {
      const created = await DocumentService.createDocument({
        title: t('notesUntitled'),
        type: 'richtext',
        content: '',
        kbId,
      });
      setFocusTitleNoteId(created.id);
      await refreshNotes(created.id);
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
    }
  };

  const notesDeleteInFlightRef = useRef<Set<string>>(new Set());

  const handleDeleteNote = async (id: string) => {
    if (notesDeleteInFlightRef.current.has(id)) return;
    if (!window.confirm(t('notesDeleteConfirm'))) return;
    notesDeleteInFlightRef.current.add(id);
    try {
      await DocumentService.deleteDocument(id);
      // The document is gone: drop any buffered autosave edits for it.
      autosave.forget(id);
      if (activeId === id) setActiveId(null);
      await refreshNotes();
      toast.success(t('notesDeleted'));
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
    } finally {
      notesDeleteInFlightRef.current.delete(id);
    }
  };

  const filteredNotes = useMemo(() => {
    const keyword = searchText.trim().toLowerCase();
    if (!keyword) return notes;
    return notes.filter((note) => note.title.toLowerCase().includes(keyword));
  }, [notes, searchText]);

  return (
    <div
      className="flex-1 flex flex-col overflow-hidden bg-[var(--color-kb-bg-app)]"
      data-testid="knowledgebase-pc-notes-workspace"
    >
      {/* Header */}
      <div className="h-[52px] shrink-0 flex items-center justify-between px-5 border-b border-[var(--color-kb-panel-border)]/60 bg-[var(--color-kb-editor)]">
        <div className="flex items-center gap-2">
          <FileText size={18} className="text-[var(--color-kb-accent)]" />
          <h1 className="text-[15px] font-semibold text-[var(--color-kb-text-heading)]">
            {t('notesWorkspaceTitle')}
          </h1>
        </div>
        <div className="flex items-center gap-3">
          <NotesSaveStatusPill
            status={autosave.status}
            lastSavedAt={autosave.lastSavedAt}
            locale={i18n.language}
            onRetry={() => void autosave.flushActive()}
            onOverwrite={() => void autosave.overwriteActive()}
          />
          <button
            type="button"
            onClick={handleCreateNote}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--color-kb-accent)] text-white text-[13px] font-medium hover:opacity-90 transition-opacity"
            data-testid="knowledgebase-pc-notes-new"
          >
            <Plus size={15} />
            {t('notesNew')}
          </button>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden">
        {/* Left: notes list */}
        <div className="w-[280px] min-w-[280px] flex flex-col border-r border-[var(--color-kb-panel-border)]/60 bg-[var(--color-kb-panel)]">
          <div className="p-3">
            <div className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)]/50">
              <Search size={14} className="text-[var(--color-kb-text-muted)]" />
              <input
                type="text"
                value={searchText}
                onChange={(event) => setSearchText(event.target.value)}
                placeholder={t('notesSearchPlaceholder')}
                className="flex-1 bg-transparent text-[13px] text-[var(--color-kb-text)] outline-none placeholder:text-[var(--color-kb-text-muted)]"
                data-testid="knowledgebase-pc-notes-search"
              />
            </div>
          </div>
          <div className="flex-1 overflow-y-auto px-2 pb-2">
            {loading ? (
              <div className="px-3 py-6 text-[13px] text-[var(--color-kb-text-muted)]">{t('notesLoading')}</div>
            ) : filteredNotes.length === 0 ? (
              <div className="px-3 py-6 text-[13px] text-[var(--color-kb-text-muted)]">
                {notes.length === 0 ? t('notesEmpty') : t('notesSearchNoMatch')}
              </div>
            ) : (
              filteredNotes.map((note) => {
                const isActive = note.id === activeId;
                return (
                  <div
                    key={note.id}
                    role="button"
                    tabIndex={0}
                    aria-current={isActive ? 'true' : undefined}
                    onClick={() => setActiveId(note.id)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        setActiveId(note.id);
                      }
                    }}
                    className={`group flex items-center justify-between px-3 py-2.5 rounded-lg cursor-pointer mb-0.5 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-kb-accent)] ${isActive ? 'bg-[var(--color-kb-panel-active)]' : 'hover:bg-[var(--color-kb-panel-hover)]'}`}
                    data-testid="knowledgebase-pc-notes-item"
                  >
                    <span
                      className={`flex-1 truncate text-[13px] ${isActive ? 'text-[var(--color-kb-accent)] font-medium' : 'text-[var(--color-kb-text)]'}`}
                    >
                      {note.title || t('notesUntitled')}
                    </span>
                    <button
                      type="button"
                      aria-label={t('notesDelete')}
                      onClick={(event) => {
                        event.stopPropagation();
                        void handleDeleteNote(note.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 p-1 rounded text-[var(--color-kb-text-muted)] hover:text-rose-400 focus-visible:text-rose-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-kb-accent)] transition-all"
                      title={t('notesDelete')}
                      data-testid="knowledgebase-pc-notes-delete"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Right: the shared Tiptap editor (same editor as the knowledge base) */}
        <div className="flex-1 flex flex-col overflow-hidden bg-[var(--color-kb-editor)]">
          {contentLoadState === 'loading' ? (
            <div className="flex-1 flex flex-col items-center justify-center gap-3 text-[var(--color-kb-text-muted)]">
              <Loader2 size={22} className="animate-spin" />
              <p className="text-[13px]">{t('notesContentLoading')}</p>
            </div>
          ) : contentLoadState === 'error' ? (
            <div className="flex-1 flex flex-col items-center justify-center gap-3">
              <CloudOff size={30} className="text-[var(--color-kb-text-muted)]" />
              <p className="text-[14px] text-[var(--color-kb-text-heading)]">
                {t('notesContentLoadFailed')}
              </p>
              <button
                type="button"
                onClick={() => setContentLoadRetryToken((token) => token + 1)}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-kb-panel-border)] text-[13px] text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] transition-colors"
                data-testid="knowledgebase-pc-notes-content-retry"
              >
                <RotateCcw size={13} />
                {t('notesContentLoadRetry')}
              </button>
            </div>
          ) : editor ? (
            <TiptapEditor
              key={editor.id}
              initialContent={editor.content}
              mode="richtext"
              onChange={handleContentChange}
              docTitle={editor.title}
              onTitleChange={handleTitleChange}
              kbId={kbId || null}
              autoFocusTitle={editor.id === focusTitleNoteId}
            />
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-center">
              <FileText size={44} className="text-[var(--color-kb-text-muted)]/50 mb-4" />
              <p className="text-[15px] text-[var(--color-kb-text-heading)] font-medium mb-1.5">
                {t('notesEditorEmptyTitle')}
              </p>
              <p className="text-[13px] text-[var(--color-kb-text-muted)]">
                {notes.length > 0 ? t('notesEditorEmptyHintSelect') : t('notesEditorEmptyHintCreate')}
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Autosave status pill (Notion-style): quiet "已保存 <time>" when healthy, a
 * spinner while saving, and explicit recovery actions when a save failed or a
 * concurrent edit was detected. Announced politely to screen readers.
 */
function NotesSaveStatusPill(props: {
  status: NotesSaveStatus;
  lastSavedAt: number | null;
  locale: string;
  onRetry: () => void;
  onOverwrite: () => void;
}) {
  const { t } = useTranslation('kb');
  const { status, lastSavedAt, locale, onRetry, onOverwrite } = props;

  if (status === 'idle') {
    return null;
  }

  const baseClass =
    'flex items-center gap-1.5 text-[12px] leading-none text-[var(--color-kb-text-muted)]';

  if (status === 'saving') {
    return (
      <span className={baseClass} role="status" aria-live="polite" data-testid="knowledgebase-pc-notes-save-status">
        <Loader2 size={12} className="animate-spin" />
        {t('notesSaving')}
      </span>
    );
  }

  if (status === 'saved') {
    const timeLabel =
      lastSavedAt !== null
        ? formatNoteSavedTime(lastSavedAt, locale || 'zh-CN')
        : null;
    return (
      <span className={baseClass} role="status" aria-live="polite" data-testid="knowledgebase-pc-notes-save-status">
        <Check size={12} className="text-emerald-500" />
        {timeLabel ? t('notesStatusSavedAt', { time: timeLabel }) : t('notesStatusSaved')}
      </span>
    );
  }

  if (status === 'conflict') {
    return (
      <span
        className="flex items-center gap-1.5 text-[12px] leading-none text-amber-600"
        role="status"
        aria-live="polite"
        data-testid="knowledgebase-pc-notes-save-status"
      >
        <AlertTriangle size={12} />
        {t('notesStatusConflict')}
        <button
          type="button"
          onClick={onOverwrite}
          className="ml-0.5 px-2 py-1 rounded-md border border-amber-500/50 text-amber-600 text-[11px] font-medium hover:bg-amber-500/10 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-kb-accent)]"
          data-testid="knowledgebase-pc-notes-overwrite"
        >
          {t('notesStatusOverwrite')}
        </button>
      </span>
    );
  }

  if (status === 'error') {
    return (
      <span
        className="flex items-center gap-1.5 text-[12px] leading-none text-rose-500"
        role="status"
        aria-live="polite"
        data-testid="knowledgebase-pc-notes-save-status"
      >
        <CloudOff size={12} />
        {t('notesStatusSaveFailed')}
        <button
          type="button"
          onClick={onRetry}
          className="ml-0.5 px-2 py-1 rounded-md border border-rose-400/50 text-rose-500 text-[11px] font-medium hover:bg-rose-500/10 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-kb-accent)]"
          data-testid="knowledgebase-pc-notes-retry"
        >
          {t('notesStatusRetry')}
        </button>
      </span>
    );
  }

  // dirty: local edits are waiting for the debounce window.
  return (
    <span className={baseClass} role="status" aria-live="polite" data-testid="knowledgebase-pc-notes-save-status">
      <span className="inline-block w-[6px] h-[6px] rounded-full bg-[var(--color-kb-text-muted)]/60" />
      {t('notesStatusUnsaved')}
    </span>
  );
}
