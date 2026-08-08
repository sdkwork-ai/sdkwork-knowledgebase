import React from 'react';
import { isBlank } from '@sdkwork/utils';
import { Search, Sparkles, Plus, Trash2, Edit3, MessageSquare } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { SearchSession } from '../types';
import { getSessionPreview, groupSessionsByDate } from '../utils/sessionHelpers';

export interface SearchSessionSidebarProps {
  sessions: SearchSession[];
  activeSessionId: string;
  sessionFilter: string;
  editingSessionId: string | null;
  editingTitle: string;
  onSessionFilterChange: (value: string) => void;
  onCreateSession: () => void;
  onSelectSession: (session: SearchSession) => void;
  onDeleteSession: (id: string, e: React.MouseEvent) => void;
  onStartRename: (id: string, title: string, e: React.MouseEvent) => void;
  onEditingTitleChange: (value: string) => void;
  onSaveRename: (id: string) => void;
  onKeyDownRename: (id: string, e: React.KeyboardEvent) => void;
}

export function SearchSessionSidebar({
  sessions,
  activeSessionId,
  sessionFilter,
  editingSessionId,
  editingTitle,
  onSessionFilterChange,
  onCreateSession,
  onSelectSession,
  onDeleteSession,
  onStartRename,
  onEditingTitleChange,
  onSaveRename,
  onKeyDownRename
}: SearchSessionSidebarProps) {
  const { t } = useTranslation('search');
  const filteredSessions = sessions.filter((s) => {
    if (isBlank(sessionFilter)) return true;
    const q = sessionFilter.toLowerCase();
    return s.title.toLowerCase().includes(q) || getSessionPreview(s, t('notStartedYet')).toLowerCase().includes(q);
  });
  const groupedSessions = groupSessionsByDate(filteredSessions);

  return (
    <div className="w-[280px] border-r border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/40 flex flex-col shrink-0 overflow-hidden animate-[fadeIn_0.2s_ease-out]">
      <div className="p-4 border-b border-[var(--color-kb-panel-border)]/70 flex flex-col gap-3 select-none shrink-0">
        <div className="flex items-center justify-between">
          <h2 className="text-[13px] font-extrabold tracking-wide text-[var(--color-kb-text-heading)] flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-[var(--color-kb-accent)]" />
            {t('aiSearchTitle')}
          </h2>
        </div>
        <button
          type="button"
          onClick={onCreateSession}
          className="w-full flex items-center justify-center gap-2 py-2.5 px-3 text-xs font-bold text-[var(--color-kb-accent)] bg-[var(--color-kb-editor)] border border-[color-mix(in_srgb,var(--color-kb-accent)_22%,var(--color-kb-panel-border))] rounded-xl hover:bg-[var(--color-kb-panel-active)] transition-all shadow-sm active:scale-[0.98]"
        >
          <Plus className="w-4 h-4" strokeWidth={2.5} />
          <span>{t('newChat')}</span>
        </button>
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-[var(--color-kb-text-muted)]" />
          <input
            type="text"
            value={sessionFilter}
            onChange={(e) => onSessionFilterChange(e.target.value)}
            placeholder={t('searchHistoryPlaceholder')}
            className="w-full pl-8 pr-3 py-2 text-xs rounded-lg bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] placeholder-[var(--color-kb-text-muted)] outline-none focus:ring-1 focus:ring-[color-mix(in_srgb,var(--color-kb-accent)_35%,transparent)]"
          />
        </div>
      </div>

      <div className="flex-1 min-h-0 search-theme-scrollbar p-3 space-y-4">
        {groupedSessions.length === 0 ? (
          <div className="px-3 py-8 text-center">
            <MessageSquare className="w-8 h-8 text-[var(--color-kb-text-muted)]/40 mx-auto mb-2" />
            <p className="text-xs text-[var(--color-kb-text-muted)]">{t('noMatchingSessions')}</p>
          </div>
        ) : null}
        {groupedSessions.map((group) => (
          <div key={group.key}>
            <p className="px-2 mb-1.5 text-[10px] font-extrabold uppercase tracking-widest text-[var(--color-kb-text-muted)]">
              {t(group.key)}
            </p>
            <div className="space-y-1">
              {group.sessions.map((s) => {
                const isActive = s.id === activeSessionId;
                const isEditing = s.id === editingSessionId;

                return (
                  <div
                    key={s.id}
                    onClick={() => {
                      if (!isEditing) onSelectSession(s);
                    }}
                    className={`group flex items-center justify-between gap-2 px-3 py-3 rounded-xl cursor-pointer transition-all ${
                      isActive
                        ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-accent)] font-semibold border border-[color-mix(in_srgb,var(--color-kb-accent)_18%,var(--color-kb-panel-border))] shadow-sm'
                        : 'hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] border border-transparent'
                    }`}
                  >
                    <div className="flex items-center gap-2.5 min-w-0 flex-1">
                      <MessageSquare
                        className={`w-4 h-4 shrink-0 ${
                          isActive
                            ? 'text-[var(--color-kb-accent)]'
                            : 'text-[var(--color-kb-text-muted)] group-hover:text-[var(--color-kb-text)]'
                        }`}
                      />

                      {isEditing ? (
                        <input
                          type="text"
                          value={editingTitle}
                          onChange={(e) => onEditingTitleChange(e.target.value)}
                          onBlur={() => onSaveRename(s.id)}
                          onKeyDown={(e) => onKeyDownRename(s.id, e)}
                          className="w-full text-xs font-bold bg-[var(--color-kb-editor)] px-1.5 py-0.5 border border-[var(--color-kb-accent)] rounded outline-none focus:ring-1 focus:ring-[color-mix(in_srgb,var(--color-kb-accent)_35%,transparent)]"
                          autoFocus
                          onClick={(e) => e.stopPropagation()}
                        />
                      ) : (
                        <div className="flex flex-col min-w-0">
                          <span className="text-[13px] truncate font-medium leading-snug text-[var(--color-kb-text-heading)]">
                            {s.title}
                          </span>
                          <span className="text-[10px] text-[var(--color-kb-text-muted)] truncate mt-0.5">
                            {getSessionPreview(s, t('notStartedYet'))}
                          </span>
                        </div>
                      )}
                    </div>

                    {!isEditing && (
                      <div className="opacity-0 group-hover:opacity-100 flex items-center gap-0.5 transition-opacity shrink-0">
                        <button
                          type="button"
                          onClick={(e) => onStartRename(s.id, s.title, e)}
                          className="p-1 hover:bg-[var(--color-kb-panel-hover)] rounded text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]"
                          title={t('renameSession')}
                        >
                          <Edit3 className="w-3 h-3" />
                        </button>
                        <button
                          type="button"
                          onClick={(e) => onDeleteSession(s.id, e)}
                          className="p-1 hover:bg-red-500/10 rounded text-[var(--color-kb-text-muted)] hover:text-red-500"
                          title={t('deleteSession')}
                        >
                          <Trash2 className="w-3 h-3" />
                        </button>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>

      <div className="p-3 border-t border-[var(--color-kb-panel-border)]/70 flex flex-col gap-1.5 shrink-0 select-none bg-[var(--color-kb-panel)]/20">
        <span className="text-[10px] font-mono text-[var(--color-kb-text-muted)] text-center font-bold">
          ENGINES INTEGRATIVE RAG • v2026
        </span>
      </div>
    </div>
  );
}
