import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FileText, Plus, Search, Trash2 } from 'lucide-react';
import type { ErrorTranslateFn } from 'sdkwork-knowledgebase-pc-core';

import { DocumentService } from './services/document';
import { getDocumentContent } from './services/knowledgebaseDocumentApiBridge';
import { listSpaceNotes, type KnowledgeNoteSummary } from './services/knowledgeNotesListService';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';
import { toast } from './components/ui/toast-manager';
import { TiptapEditor } from './TiptapEditor';

const TITLE_SAVE_DEBOUNCE_MS = 800;
const CONTENT_SAVE_DEBOUNCE_MS = 1200;

function useDebouncedCallback<A extends unknown[]>(callback: (...args: A) => void, delayMs: number) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const callbackRef = useRef(callback);
  callbackRef.current = callback;
  useEffect(() => () => {
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);
  return useCallback((...args: A) => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => callbackRef.current(...args), delayMs);
  }, [delayMs]);
}

interface NoteEditorState {
  id: string;
  title: string;
  content: string;
}

/**
 * Full notes workspace: notes list on the left, the shared TiptapEditor on the
 * right (Notion / Tencent IMA style). Notes are the space's knowledge documents
 * listed through `documents.list`, so notes created here and notes created from
 * the knowledge base view ("新的笔记") are the same documents and stay in sync.
 */
export function NotesWorkspace() {
  const { t } = useTranslation('kb');
  const [kbId, setKbId] = useState<string>('');
  const [kbResolved, setKbResolved] = useState(false);
  const [notes, setNotes] = useState<KnowledgeNoteSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchText, setSearchText] = useState('');
  const [activeId, setActiveId] = useState<string | null>(null);
  const [editor, setEditor] = useState<NoteEditorState | null>(null);
  const [saving, setSaving] = useState(false);

  // Resolve the working knowledge base: the stored active KB first, then the
  // first personal KB, so the workspace is self-sufficient on first launch.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const stored = window.localStorage.getItem('app-active-kb');
        if (stored) {
          try {
            const parsed = JSON.parse(stored) as { id?: string };
            if (parsed.id && parsed.id.trim()) {
              if (!cancelled) setKbId(parsed.id.trim());
              return;
            }
          } catch {
            // Fall through to KB discovery.
          }
        }
        const grouped = await DocumentService.getKnowledgeBases();
        const first = grouped.personal[0] ?? grouped.team[0] ?? grouped.public[0];
        if (first && !cancelled) setKbId(first.id);
      } catch (error) {
        toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
      } finally {
        if (!cancelled) setKbResolved(true);
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

  // Load the selected note into the shared editor.
  useEffect(() => {
    let cancelled = false;
    if (!activeId) {
      setEditor(null);
      return;
    }
    (async () => {
      const title = notes.find((note) => note.id === activeId)?.title ?? '';
      try {
        const content = await getDocumentContent(activeId);
        if (cancelled) return;
        setEditor({ id: activeId, title, content });
      } catch (error) {
        if (!cancelled) {
          setEditor({ id: activeId, title, content: '' });
          toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [activeId, notes, t]);

  const persistTitle = useDebouncedCallback(async (id: string, title: string) => {
    try {
      setSaving(true);
      // No `kbId`: the bridge treats a kbId that differs from the document's
      // resolved space as a cross-space move, which would swallow the rename.
      // Manual notes always rename in place via `documents.update`.
      await DocumentService.updateDocument(id, { title });
      setNotes((prev) => prev.map((note) => (note.id === id ? { ...note, title } : note)));
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
    } finally {
      setSaving(false);
    }
  }, TITLE_SAVE_DEBOUNCE_MS);

  const persistContent = useDebouncedCallback(async (id: string, content: string) => {
    try {
      setSaving(true);
      await DocumentService.saveDocumentContent(id, content);
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
    } finally {
      setSaving(false);
    }
  }, CONTENT_SAVE_DEBOUNCE_MS);

  const handleTitleChange = useCallback((value: string) => {
    setEditor((prev) => (prev ? { ...prev, title: value } : prev));
    if (activeId) persistTitle(activeId, value);
  }, [activeId, persistTitle]);

  const handleContentChange = useCallback((content: string) => {
    setEditor((prev) => (prev ? { ...prev, content } : prev));
    if (activeId) persistContent(activeId, content);
  }, [activeId, persistContent]);

  const handleCreateNote = async () => {
    if (!kbId) return;
    try {
      const created = await DocumentService.createDocument({
        title: t('notesUntitled'),
        type: 'richtext',
        content: '',
        kbId,
      });
      await refreshNotes(created.id);
      toast.success(t('notesCreated'));
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
    }
  };

  const handleDeleteNote = async (id: string) => {
    try {
      await DocumentService.deleteDocument(id);
      if (activeId === id) setActiveId(null);
      await refreshNotes();
      toast.success(t('notesDeleted'));
    } catch (error) {
      toastKnowledgebaseError(error, t as unknown as ErrorTranslateFn);
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
          {saving ? (
            <span className="text-[11px] text-[var(--color-kb-text-muted)]">{t('notesSaving')}</span>
          ) : null}
        </div>
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
                    onClick={() => setActiveId(note.id)}
                    className={`group flex items-center justify-between px-3 py-2.5 rounded-lg cursor-pointer mb-0.5 transition-colors ${isActive ? 'bg-[var(--color-kb-panel-active)]' : 'hover:bg-[var(--color-kb-panel-hover)]'}`}
                    data-testid="knowledgebase-pc-notes-item"
                  >
                    <span
                      className={`flex-1 truncate text-[13px] ${isActive ? 'text-[var(--color-kb-accent)] font-medium' : 'text-[var(--color-kb-text)]'}`}
                    >
                      {note.title || t('notesUntitled')}
                    </span>
                    <button
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        void handleDeleteNote(note.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 rounded text-[var(--color-kb-text-muted)] hover:text-rose-400 transition-all"
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
          {editor ? (
            <TiptapEditor
              key={editor.id}
              initialContent={editor.content}
              mode="richtext"
              onChange={handleContentChange}
              docTitle={editor.title}
              onTitleChange={handleTitleChange}
              kbId={kbId || null}
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
