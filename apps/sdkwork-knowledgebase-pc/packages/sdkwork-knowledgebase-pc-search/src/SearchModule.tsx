import { isBlank, trim } from '@sdkwork/utils';
import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { DocumentService } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase/services/document';
import { SearchChatHeader } from './components/SearchChatHeader';
import { SearchChatThread } from './components/SearchChatThread';
import { SearchComposer } from './components/SearchComposer';
import { SearchComposerDock } from './components/SearchComposerDock';
import { SearchLandingPage } from './components/SearchLandingPage';
import { SearchMediaViewerHost } from './components/SearchMediaViewerHost';
import { SearchSessionSidebar } from './components/SearchSessionSidebar';
import { SEARCH_SESSIONS_STORAGE_KEY } from './constants';
import { persistSearchSessions, loadSearchSessions } from './searchSessionStorage';
import { buildRelatedMedia } from './services/buildRelatedMedia';
import { generateCitationsAndResults } from './services/searchQueryEngine';
import type { SearchMessage, SearchModuleProps, SearchSession } from './types';
import { hasRelatedMedia } from './utils/mediaResults';
import { showSearchToast } from './utils/searchToast';

export type { SearchMessage, SearchModuleProps, SearchSession, SearchSource } from './types';

// Startup rehydration is deliberately bounded: at most RECENT_SESSIONS are migrated and
// each runs its searches sequentially, so opening the app can never fire hundreds of
// concurrent RAG calls against historical messages.
const REHYDRATE_MAX_SESSIONS = 5;
const REHYDRATE_MAX_MESSAGES_PER_SESSION = 10;

async function rehydrateMissingRelatedMedia(
  sessionList: SearchSession[]
): Promise<{ sessions: SearchSession[]; changed: boolean }> {
  let changed = false;

  const sessions: SearchSession[] = [];
  for (const session of sessionList.slice(0, REHYDRATE_MAX_SESSIONS)) {
    const webSearchEnabled = session.webSearchEnabled ?? true;
    let sessionChanged = false;

    const messages = [...session.messages];
    const candidates = session.messages
      .slice(0, REHYDRATE_MAX_MESSAGES_PER_SESSION)
      .map((msg, msgIndex) => ({ msg, msgIndex }))
      .filter(({ msg, msgIndex }) => {
        if (
          msg.role !== 'assistant' ||
          msg.isSearching ||
          isBlank(msg.content) ||
          hasRelatedMedia(msg.relatedMedia)
        ) {
          return false;
        }
        return session.messages
          .slice(0, msgIndex)
          .reverse()
          .some((m) => m.role === 'user');
      });

    // Sequential per-session migration: bounded latency at startup, no request burst.
    for (const { msg, msgIndex } of candidates) {
      sessionChanged = true;
      changed = true;
      const userMsg = [...session.messages.slice(0, msgIndex)]
        .reverse()
        .find((m) => m.role === 'user');
      if (!userMsg) continue;
      try {
        const searchResults = await DocumentService.searchAll(userMsg.content);
        messages[msgIndex] = {
          ...msg,
          relatedMedia: buildRelatedMedia(
            userMsg.content,
            searchResults.docs,
            webSearchEnabled
          )
        };
      } catch {
        messages[msgIndex] = {
          ...msg,
          relatedMedia: buildRelatedMedia(userMsg.content, [], webSearchEnabled)
        };
      }
    }

    sessions.push(sessionChanged ? { ...session, messages } : session);
  }

  return { sessions, changed };
}

export function SearchModule({ onGoToKb, onGoToFile, onOpenWebLink }: SearchModuleProps) {
  const { t } = useTranslation('search');
  const [sessions, setSessions] = useState<SearchSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState('');
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState('');
  const [sessionFilter, setSessionFilter] = useState('');
  const [expandedStepIds, setExpandedStepIds] = useState<Record<string, boolean>>({});
  const [webSearchEnabled, setWebSearchEnabled] = useState(true);
  const [deepThinkEnabled, setDeepThinkEnabled] = useState(false);

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const streamIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  // Monotonic generation id: bumping it on stop invalidates an in-flight retrieval so a
  // late response can never resume streaming after the user pressed stop.
  const generationSeqRef = useRef(0);
  // Latest session snapshot for debounced stream persistence without re-render.
  const sessionsRef = useRef<SearchSession[] | null>(null);
  const shouldAutoScrollRef = useRef(true);
  const wasInChatModeRef = useRef(false);
  const mediaMigrationDoneRef = useRef(false);
  // Hydration gate: the persistence effect must not write until the load
  // effect below has either restored stored sessions or seeded the first one.
  // Without this gate the initial empty `sessions` array would overwrite
  // localStorage before the load ran, wiping the user's saved history on
  // every mount.
  const sessionsHydratedRef = useRef(false);

  // Single persistence point: every committed sessions change is trimmed and
  // written through the bounded, failure-safe storage helpers. During active
  // streaming the interval writer persists on its own throttled schedule, so
  // the effect stands down until the stream interval is cleared.
  useEffect(() => {
    if (!sessionsHydratedRef.current || streamIntervalRef.current) {
      return;
    }
    persistSearchSessions(SEARCH_SESSIONS_STORAGE_KEY, sessions);
  }, [sessions]);

  useEffect(() => {
    // Hydration runs exactly once: a locale switch re-runs this effect via the
    // `t` dependency, and re-seeding would reset the active session and the
    // composer toggles mid-use.
    if (sessionsHydratedRef.current) return;
    const stored = loadSearchSessions(SEARCH_SESSIONS_STORAGE_KEY);
    if (stored && stored.length > 0) {
      sessionsHydratedRef.current = true;
      setSessions(stored);
      setActiveSessionId(stored[0].id);
      setWebSearchEnabled(stored[0].webSearchEnabled ?? true);
      setDeepThinkEnabled(stored[0].deepThinkEnabled ?? false);
      return;
    }

    const firstSession: SearchSession = {
      id: 'session-' + Date.now(),
      title: t('newAiChat'),
      createdAt: new Date().toISOString(),
      messages: [],
      webSearchEnabled: true,
      deepThinkEnabled: false
    };
    sessionsHydratedRef.current = true;
    setSessions([firstSession]);
    setActiveSessionId(firstSession.id);
    persistSearchSessions(SEARCH_SESSIONS_STORAGE_KEY, [firstSession]);
  }, [t]);

  useEffect(() => {
    if (mediaMigrationDoneRef.current || sessions.length === 0) return;

    const needsMigration = sessions.some((session) =>
      session.messages.some(
        (msg) =>
          msg.role === 'assistant' &&
          !msg.isSearching &&
          !isBlank(msg.content) &&
          !hasRelatedMedia(msg.relatedMedia)
      )
    );

    if (!needsMigration) {
      mediaMigrationDoneRef.current = true;
      return;
    }

    mediaMigrationDoneRef.current = true;
    void rehydrateMissingRelatedMedia(sessions).then(({ sessions: migrated, changed }) => {
      if (!changed) return;
      // Merge by session/message id instead of overwriting: migration is slow
      // (sequential search lookups), and any session change committed while it
      // ran (queries sent, streams appended) must survive. Only the missing
      // relatedMedia payloads are adopted; everything else keeps its live value.
      setSessions((prev) => {
        const migratedById = new Map(migrated.map((session) => [session.id, session]));
        return prev.map((session) => {
          const migratedSession = migratedById.get(session.id);
          if (!migratedSession) return session;
          const migratedMessages = new Map(migratedSession.messages.map((m) => [m.id, m]));
          return {
            ...session,
            messages: session.messages.map((message) => {
              const migratedMessage = migratedMessages.get(message.id);
              if (!migratedMessage || hasRelatedMedia(message.relatedMedia) || !hasRelatedMedia(migratedMessage.relatedMedia)) {
                return message;
              }
              return { ...message, relatedMedia: migratedMessage.relatedMedia };
            }),
          };
        });
      });
    });
  }, [sessions]);

  const saveSessionsToStorage = (updatedSessions: SearchSession[]) => {
    // Persistence itself goes through the sessions effect; this helper only
    // commits the new list.
    setSessions(updatedSessions);
  };

  const activeSession = sessions.find((s) => s.id === activeSessionId) || sessions[0];

  const scrollToBottom = useCallback((behavior: ScrollBehavior = 'smooth') => {
    const el = scrollContainerRef.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior });
  }, []);

  const handleScrollContainer = () => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    shouldAutoScrollRef.current = distanceFromBottom < 120;
  };

  useEffect(() => {
    if (shouldAutoScrollRef.current) {
      scrollToBottom(isTyping ? 'auto' : 'smooth');
    }
  }, [activeSession?.messages, isTyping, scrollToBottom]);

  useEffect(() => {
    const timer = window.setTimeout(() => textareaRef.current?.focus(), 80);
    return () => window.clearTimeout(timer);
  }, [activeSessionId, activeSession?.messages.length]);

  useEffect(() => {
    const inChat = (activeSession?.messages.length ?? 0) > 0;
    if (inChat && !wasInChatModeRef.current) {
      setSidebarOpen(true);
    }
    wasInChatModeRef.current = inChat;
  }, [activeSession?.messages.length]);

  useEffect(() => {
    return () => {
      if (streamIntervalRef.current) clearInterval(streamIntervalRef.current);
    };
  }, []);

  const handleCreateNewSession = useCallback(() => {
    const newSession: SearchSession = {
      id: 'session-' + Date.now(),
      title: t('newAiChat'),
      createdAt: new Date().toISOString(),
      messages: [],
      webSearchEnabled,
      deepThinkEnabled
    };
    setSessions((prev) => {
      const updated = [newSession, ...prev];
      return updated;
    });
    setActiveSessionId(newSession.id);
    setInputValue('');
    shouldAutoScrollRef.current = true;
  }, [webSearchEnabled, deepThinkEnabled]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
        e.preventDefault();
        handleCreateNewSession();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handleCreateNewSession]);

  const handleDeleteSession = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const updated = sessions.filter((s) => s.id !== id);
    if (updated.length === 0) {
      const fallback: SearchSession = {
        id: 'session-' + Date.now(),
        title: t('newAiChat'),
        createdAt: new Date().toISOString(),
        messages: []
      };
      saveSessionsToStorage([fallback]);
      setActiveSessionId(fallback.id);
    } else {
      saveSessionsToStorage(updated);
      if (activeSessionId === id) {
        setActiveSessionId(updated[0].id);
      }
    }
  };

  const handleStartRename = (id: string, currentTitle: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingSessionId(id);
    setEditingTitle(currentTitle);
  };

  const handleSaveRename = (id: string) => {
    if (isBlank(editingTitle)) return;
    const updated = sessions.map((s) =>
      s.id === id ? { ...s, title: trim(editingTitle) } : s
    );
    saveSessionsToStorage(updated);
    setEditingSessionId(null);
  };

  const handleKeyDownRename = (id: string, e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSaveRename(id);
    else if (e.key === 'Escape') setEditingSessionId(null);
  };

  const handleSendQuery = async (queryText?: string, sessionOverride?: SearchSession) => {
    const rawVal = queryText !== undefined ? queryText : inputValue;
    if (isBlank(rawVal) || isTyping) return;

    const query = trim(rawVal);
    setInputValue('');

    const currentSession =
      sessionOverride ?? sessions.find((s) => s.id === activeSessionId) ?? sessions[0];
    if (!currentSession) return;

    const isFirstInSession = currentSession.messages.length === 0;

    const userMsg: SearchMessage = {
      id: 'msg-user-' + Date.now(),
      role: 'user',
      content: query,
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    };

    const initialSteps: NonNullable<SearchMessage['searchSteps']> = [
      { id: 'intent', label: t('stepIntent'), status: 'running' },
      { id: 'local', label: t('stepLocal'), status: 'idle' },
      ...(webSearchEnabled
        ? [{ id: 'web', label: t('stepWeb'), status: 'idle' as const }]
        : []),
      {
        id: 'synthesis',
        label: deepThinkEnabled ? t('stepSynthesisDeep') : t('stepSynthesis'),
        status: 'idle' as const
      }
    ];

    const assistantMsgId = 'msg-ai-' + Date.now();
    const assistantMsg: SearchMessage = {
      id: assistantMsgId,
      role: 'assistant',
      content: '',
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      isSearching: true,
      searchSteps: initialSteps
    };

    const updatedMessages = [...currentSession.messages, userMsg, assistantMsg];
    const sessionTitle = isFirstInSession
      ? query.length > 14
        ? query.substring(0, 14) + '...'
        : query
      : currentSession.title;

    const updatedSession = {
      ...currentSession,
      title: sessionTitle,
      messages: updatedMessages,
      webSearchEnabled,
      deepThinkEnabled
    };

    const newSessionsList = sessions.map((s) =>
      s.id === currentSession.id ? updatedSession : s
    );
    saveSessionsToStorage(newSessionsList);
    setIsTyping(true);
    shouldAutoScrollRef.current = true;

    const steps = [...initialSteps];
    const updateStepsState = (stepId: string, status: 'running' | 'success' | 'failed') => {
      const idx = steps.findIndex((st) => st.id === stepId);
      if (idx !== -1) {
        steps[idx].status = status;
        if (status === 'success' && idx < steps.length - 1) {
          steps[idx + 1].status = 'running';
        }

        setSessions((prev) =>
          prev.map((s) => {
            if (s.id === currentSession.id) {
              return {
                ...s,
                messages: s.messages.map((m) =>
                  m.id === assistantMsgId ? { ...m, searchSteps: [...steps] } : m
                )
              };
            }
            return s;
          })
        );
      }
    };

    const generation = ++generationSeqRef.current;

    // Steps advance on real retrieval outcome, never on simulated delays.
    const { sources, relatedMedia, responseText } = await generateCitationsAndResults(query, {
      webSearchEnabled
    });
    if (generation !== generationSeqRef.current) {
      // The user stopped this generation while it was in flight; discard the result.
      return;
    }

    updateStepsState('intent', 'success');
    updateStepsState('local', 'success');
    if (webSearchEnabled) {
      updateStepsState('web', 'success');
    }
    updateStepsState('synthesis', 'success');

    if (streamIntervalRef.current) clearInterval(streamIntervalRef.current);

    const sessionIdForStream = currentSession.id;

    setSessions((prev) => {
      const list = prev.map((s) => {
        if (s.id !== sessionIdForStream) return s;
        return {
          ...s,
          messages: s.messages.map((m) =>
            m.id === assistantMsgId
              ? { ...m, sources, relatedMedia, isSearching: false }
              : m
          )
        };
      });
      return list;
    });

    const chars = Array.from(responseText);
    let index = 0;
    // Persist only once per chunk of streamed characters: serializing the full
    // session list to localStorage on every 10-12ms tick is wasteful and blocks
    // the main thread (each write is a synchronous JSON.stringify + storage IO).
    const PERSIST_EVERY_CHARS = 32;
    let persistedUpTo = -1;
    const persistStreamedSession = () => {
      if (persistedUpTo >= index) {
        return;
      }
      persistedUpTo = index;
      const snapshot = sessionsRef.current;
      if (snapshot) {
        persistSearchSessions(SEARCH_SESSIONS_STORAGE_KEY, snapshot);
      }
    };
    streamIntervalRef.current = setInterval(() => {
      setSessions((prev) => {
        const list = prev.map((s) => {
          if (s.id === sessionIdForStream) {
            return {
              ...s,
              messages: s.messages.map((m) => {
                if (m.id === assistantMsgId) {
                  return {
                    ...m,
                    content: chars.slice(0, index + 1).join(''),
                    sources,
                    relatedMedia,
                    isSearching: false
                  };
                }
                return m;
              })
            };
          }
          return s;
        });
        sessionsRef.current = list;
        if (index % PERSIST_EVERY_CHARS === 0) {
          persistSearchSessions(SEARCH_SESSIONS_STORAGE_KEY, list);
          persistedUpTo = index;
        }
        return list;
      });

      index++;
      if (index >= chars.length) {
        if (streamIntervalRef.current) clearInterval(streamIntervalRef.current);
        streamIntervalRef.current = null;
        setIsTyping(false);
        // Final flush guarantees the completed answer is persisted even when
        // the last chunk boundary was not a persist tick.
        persistStreamedSession();
      }
    }, deepThinkEnabled ? 10 : 12);
  };

  const handleStopGeneration = () => {
    generationSeqRef.current += 1;
    if (streamIntervalRef.current) {
      clearInterval(streamIntervalRef.current);
      streamIntervalRef.current = null;
    }
    setSessions((prev) => {
      const list = prev.map((s) => {
        if (s.id !== activeSessionId) return s;
        const msgs = s.messages;
        if (msgs.length === 0) return s;
        const last = msgs[msgs.length - 1];
        if (last.role !== 'assistant' || !last.isSearching) return s;
        return {
          ...s,
          messages: msgs.map((m, i) =>
            i === msgs.length - 1 ? { ...m, isSearching: false } : m
          )
        };
      });
      return list;
    });
    setIsTyping(false);
    showSearchToast(t('stoppedGeneration'));
  };

  const handleRegenerate = (userQuery: string) => {
    if (isTyping || !activeSession) return;
    const msgs = activeSession.messages;
    if (msgs.length < 2) return;

    const last = msgs[msgs.length - 1];
    const prev = msgs[msgs.length - 2];
    if (last.role !== 'assistant' || prev.role !== 'user' || prev.content !== userQuery) return;

    const trimmedSession = { ...activeSession, messages: msgs.slice(0, -2) };
    const updated = sessions.map((s) => (s.id === activeSessionId ? trimmedSession : s));
    saveSessionsToStorage(updated);
    shouldAutoScrollRef.current = true;
    void handleSendQuery(userQuery, trimmedSession);
  };

  const handleKeyDownInput = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendQuery();
    }
  };

  const handleCopyContent = async (text: string) => {
    await navigator.clipboard.writeText(text);
    showSearchToast(t('copiedToClipboard'));
  };

  const handleOneClickExport = async (text: string) => {
    try {
      const cleanContent = text.replace(/<[^>]*>?/gm, '');
      const titleMatch = cleanContent.match(/###\s*(.*)/) || cleanContent.match(/#\s*(.*)/);
      const title = titleMatch ? titleMatch[1].trim() : t('exportNoteTitle');

      const resultKbs = await DocumentService.getKnowledgeBases();
      const kbGroup = resultKbs.personal.length > 0 ? resultKbs.personal[0] : resultKbs.team[0];

      if (kbGroup) {
        await DocumentService.createDocument({
          title: t('exportDocTitlePrefix', { title: title.substring(0, 24) }),
          type: 'richtext',
          kbId: kbGroup.id,
          content: `<p>${t('exportDocIntro')}</p>` + text.replace(/\n/g, '<br/>')
        });

        window.dispatchEvent(
          new CustomEvent('local-storage', {
            detail: { key: 'app-active-kb', value: kbGroup }
          })
        );

        showSearchToast(t('savedToKb', { title: kbGroup.title }), 'success');
      }
    } catch (e) {
      showSearchToast(
        e instanceof Error ? e.message : t('saveToKbFailed', { defaultValue: '保存到知识库失败，请稍后重试。' }),
      );
    }
  };

  const isInChatMode = (activeSession?.messages.length ?? 0) > 0;
  const showSidebar = isInChatMode && sidebarOpen;
  const sidebarSessions = sessions.filter((s) => s.messages.length > 0);

  const composerProps = {
    inputValue,
    onInputChange: setInputValue,
    onKeyDown: handleKeyDownInput,
    onSend: () => handleSendQuery(),
    isTyping,
    webSearchEnabled,
    deepThinkEnabled,
    onToggleWeb: () => setWebSearchEnabled(!webSearchEnabled),
    onToggleDeep: () => setDeepThinkEnabled(!deepThinkEnabled),
    textareaRef
  };

  return (
    <div className="flex-1 flex min-h-0 h-full bg-[var(--color-kb-editor)] text-[var(--color-kb-text)] font-sans overflow-hidden">
      {showSidebar && (
        <SearchSessionSidebar
          sessions={sidebarSessions}
          activeSessionId={activeSessionId}
          sessionFilter={sessionFilter}
          editingSessionId={editingSessionId}
          editingTitle={editingTitle}
          onSessionFilterChange={setSessionFilter}
          onCreateSession={handleCreateNewSession}
          onSelectSession={(s) => {
            setActiveSessionId(s.id);
            setWebSearchEnabled(s.webSearchEnabled ?? true);
            setDeepThinkEnabled(s.deepThinkEnabled ?? false);
          }}
          onDeleteSession={handleDeleteSession}
          onStartRename={handleStartRename}
          onEditingTitleChange={setEditingTitle}
          onSaveRename={handleSaveRename}
          onKeyDownRename={handleKeyDownRename}
        />
      )}

      <div className="flex-1 flex flex-col min-h-0 overflow-hidden relative search-chat-surface">
        {isInChatMode && activeSession && (
          <SearchChatHeader
            title={activeSession.title}
            sidebarOpen={sidebarOpen}
            onToggleSidebar={() => setSidebarOpen(!sidebarOpen)}
          />
        )}

        {isInChatMode && activeSession ? (
          <>
            <div
              ref={scrollContainerRef}
              onScroll={handleScrollContainer}
              className="flex-1 min-h-0 search-theme-scrollbar w-full"
            >
              <SearchChatThread
                messages={activeSession.messages}
                isTyping={isTyping}
                expandedStepIds={expandedStepIds}
                onToggleSteps={(messageId) =>
                  setExpandedStepIds((prev) => ({
                    ...prev,
                    [messageId]: !(prev[messageId] ?? false)
                  }))
                }
                onRegenerate={handleRegenerate}
                onExport={handleOneClickExport}
                onCopy={handleCopyContent}
                onFollowUp={(text) => handleSendQuery(text)}
                onGoToKb={onGoToKb}
                onGoToFile={onGoToFile}
                onOpenWebLink={onOpenWebLink}
              />
            </div>

            <SearchComposerDock>
              <SearchComposer
                {...composerProps}
                onStop={handleStopGeneration}
                variant="chat"
                placeholder={t('composerPlaceholderFollowUp')}
              />
            </SearchComposerDock>
          </>
        ) : (
          <div
            ref={scrollContainerRef}
            onScroll={handleScrollContainer}
            className="flex-1 min-h-0 search-theme-scrollbar w-full"
          >
            <SearchLandingPage
              {...composerProps}
              onPresetClick={(text) => {
                if (!isTyping) handleSendQuery(text);
              }}
            />
          </div>
        )}
      </div>

      <SearchMediaViewerHost onGoToFile={onGoToFile} onOpenWebLink={onOpenWebLink} />
    </div>
  );
}
