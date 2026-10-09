import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { isBlank } from '@sdkwork/utils';
import { useNavigate } from 'react-router-dom';
import {
  clearKbNavIntent,
  dispatchLocateKbFile,
  readKbNavIntent,
  useLocalStorage
} from '@sdkwork/sdkwork-knowledgebase-pc-commons';
import {
  assertKnowledgebasePreviewFeature,
  getKnowledgebaseTenantId,
  KnowledgebaseErrorCodes,
  shouldUseKnowledgebaseDemoFallback,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';
import { findDocInTree } from './utils/docTreeUtils';
import { DocumentService, FolderNode, DocumentMeta, KnowledgeBase } from './services/document';
import {
  EphemeralTabCacheService,
  TabCacheService,
  type KnowledgebaseTabCache,
} from './services/tabService';
import { KnowledgeBaseList } from './KnowledgeBaseList';
import { KnowledgeFileList } from './KnowledgeFileList';
import { EditorPanel } from './EditorPanel';
import { CreateKbModal } from './CreateKbModal';
import { PublishModal } from './PublishModal';
import { KnowledgeBaseSettingsModal } from './KnowledgeBaseSettingsModal';
import { KnowledgeBaseMarketModal } from './KnowledgeBaseMarketModal';
import { CloudDriveModal } from './CloudDriveModal';
import type { CloudDriveImportResultItem } from './services/cloudDriveService';
// Removed WechatPublishModal
import { useTranslation } from 'react-i18next';
import { KnowledgeBaseMarketView } from './KnowledgeBaseMarketView';
import { GitIntegrationModal } from './components/GitIntegrationModal';
import { useKnowledgeBaseDocumentPersistence } from './hooks/useKnowledgeBaseDocumentPersistence';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';
import { toast } from './components/ui/toast-manager';
import { invalidateKnowledgeBrowserNodeCacheForSpaceIds } from './services/knowledgeBrowserListService';
import {
  activateEphemeralFixedKnowledgebaseWorkspace,
  shouldPersistKnowledgebaseWorkspaceState,
  type KnowledgebaseWorkspaceMode,
} from './workspaceMode';

export * from './components/ui/toast-manager';
export * from './services/tabService';
export interface KnowledgeBaseAppProps {
  activeTab?: 'kb' | 'market';
  /** Locks the workspace to one server-authorized space with no fallback selection. */
  fixedKnowledgeBase?: KnowledgeBase;
  onActiveTabChange?: (tab: 'kb' | 'market') => void;
  /** Uses memory-only state for a server-authorized fixed group workspace. */
  workspaceMode?: KnowledgebaseWorkspaceMode;
}

export function KnowledgeBaseApp({
  activeTab: propActiveTab,
  fixedKnowledgeBase,
  onActiveTabChange,
  workspaceMode = 'standard',
}: KnowledgeBaseAppProps = {}) {
  const fixedKnowledgeBaseId = fixedKnowledgeBase?.id.trim() || undefined;
  const effectiveWorkspaceMode: KnowledgebaseWorkspaceMode = fixedKnowledgeBaseId
    ? 'ephemeral-fixed'
    : workspaceMode;
  const isEphemeralFixedWorkspace = effectiveWorkspaceMode === 'ephemeral-fixed';
  const persistWorkspaceState = shouldPersistKnowledgebaseWorkspaceState(effectiveWorkspaceMode);
  const [localActiveTab, setLocalActiveTab] = useLocalStorage<'kb' | 'market'>(
    'app-kb-active-tab',
    'kb',
    { enabled: persistWorkspaceState },
  );
  const activeTab = propActiveTab !== undefined ? propActiveTab : localActiveTab;
  
  const setActiveTab = (tab: 'kb' | 'market') => {
    if (onActiveTabChange) {
      onActiveTabChange(tab);
    } else {
      setLocalActiveTab(tab);
    }
  };

  const { t } = useTranslation(['kb', 'common']);
  const navigate = useNavigate();
  const [storedActiveKb, setStoredActiveKb] = useLocalStorage<KnowledgeBase | null>(
    'app-active-kb',
    null,
    { enabled: persistWorkspaceState },
  );
  const [fixedActiveKb, setFixedActiveKb] = useState<KnowledgeBase | null>(null);
  const activeKb = fixedKnowledgeBase ? fixedActiveKb : storedActiveKb;
  const setActiveKb = fixedKnowledgeBase ? setFixedActiveKb : setStoredActiveKb;
  const [activeDoc, setActiveDoc] = useState<DocumentMeta | null>(null);
  const [openDocs, setOpenDocs] = useState<DocumentMeta[]>([]);
  const [kbs, setKbs] = useState<{ team: KnowledgeBase[], personal: KnowledgeBase[], public: KnowledgeBase[] }>({ team: [], personal: [], public: [] });
  const [docs, setDocs] = useState<(FolderNode | DocumentMeta)[]>([]);
  const [docContent, setDocContent] = useState<string>('');
  const [isDocLoading, setIsDocLoading] = useState<boolean>(false);
  const [loadingKbs, setLoadingKbs] = useState<boolean>(true);
  const [loadingDocs, setLoadingDocs] = useState<boolean>(false);
  const [isAIOpen, setIsAIOpen] = useLocalStorage<boolean>(
    'app-is-ai-open',
    true,
    { enabled: persistWorkspaceState },
  );
  const [aiWidth, setAiWidth] = useLocalStorage<number>(
    'app-ai-width',
    420,
    { enabled: persistWorkspaceState },
  );
  const [isDraggingAi, setIsDraggingAi] = useState<boolean>(false);
  const [kbsWidth, setKbsWidth] = useLocalStorage<number>(
    'app-kbs-width',
    240,
    { enabled: persistWorkspaceState },
  );
  const [isDraggingKbs, setIsDraggingKbs] = useState<boolean>(false);

  // Debounced, serialized document persistence: one save per document in flight, pending
  // content flushed on doc switch and window unload (never a request storm per keystroke).
  const { handleContentChange } = useKnowledgeBaseDocumentPersistence({
    activeDoc,
    docs,
    loadingDocs,
    setOpenDocs,
    setActiveDoc,
    setDocContent,
  });

  // Request sequence guards: a stale response (older than the latest selection) is
  // discarded so fast doc/KB switching can never render one document under another.
  const docRequestSeqRef = useRef(0);
  const kbRequestSeqRef = useRef(0);
  const kbRetryTimerRef = useRef<number | null>(null);
  const deleteSelectionInFlightRef = useRef(false);
  const [docsWidth, setDocsWidth] = useLocalStorage<number>(
    'app-docs-width',
    340,
    { enabled: persistWorkspaceState },
  );
  const [isDraggingDocs, setIsDraggingDocs] = useState<boolean>(false);

  // Live drag widths: updated per mousemove as plain state (pure updaters,
  // StrictMode-safe) and committed to the persisted localStorage-backed width
  // only once the drag ends — never a localStorage write per mousemove.
  const [liveAiWidth, setLiveAiWidth] = useState<number | null>(null);
  const [liveKbsWidth, setLiveKbsWidth] = useState<number | null>(null);
  const [liveDocsWidth, setLiveDocsWidth] = useState<number | null>(null);
  const aiWidthLive = liveAiWidth ?? aiWidth;
  const kbsWidthLive = liveKbsWidth ?? kbsWidth;
  const docsWidthLive = liveDocsWidth ?? docsWidth;
  
  const [isCreateKbModalOpen, setIsCreateKbModalOpen] = useState<boolean>(false);
  const [isPublishModalOpen, setIsPublishModalOpen] = useState<boolean>(false);
  const [isMarketOpen, setIsMarketOpen] = useState<boolean>(false);
  const [settingsKb, setSettingsKb] = useState<KnowledgeBase | null>(null);
  const [publishDocsContext, setPublishDocsContext] = useState<DocumentMeta[]>([]);
  const [newKbTitle, setNewKbTitle] = useState('');
  const [newKbType, setNewKbType] = useState<'team' | 'personal' | 'public'>('personal');
  const [newKbIcon, setNewKbIcon] = useState('📘');
  const [selectedDocIds, setSelectedDocIds] = useState<Set<string>>(new Set());
  const [gitModalMode, setGitModalMode] = useState<'import' | 'sync' | null>(null);
  const [gitModalKb, setGitModalKb] = useState<KnowledgeBase | null>(null);
  const [cloudDriveKb, setCloudDriveKb] = useState<KnowledgeBase | null>(null);
  const tabCache = useMemo<KnowledgebaseTabCache>(() => {
    return persistWorkspaceState ? TabCacheService : new EphemeralTabCacheService();
  }, [persistWorkspaceState]);

  useEffect(() => {
    if (!isEphemeralFixedWorkspace || !fixedKnowledgeBaseId) {
      return undefined;
    }

    const releaseWorkspace = activateEphemeralFixedKnowledgebaseWorkspace(fixedKnowledgeBaseId);
    return () => {
      releaseWorkspace();
      invalidateKnowledgeBrowserNodeCacheForSpaceIds(fixedKnowledgeBaseId);
    };
  }, [fixedKnowledgeBaseId, isEphemeralFixedWorkspace]);

  useEffect(() => {
    return () => {
      tabCache.dispose();
    };
  }, [tabCache]);

  useEffect(() => {
    if (!isDraggingAi && !isDraggingKbs && !isDraggingDocs) {
      return;
    }
    const handleMouseMove = (e: MouseEvent) => {
      if (isDraggingAi) {
        const newWidth = Math.round(document.body.clientWidth - e.clientX);
        if (newWidth > 200 && newWidth < 800) setLiveAiWidth(newWidth);
      } else if (isDraggingKbs) {
        const kWidth = Math.round(e.clientX - 64); // Adjust for GlobalNav
        if (kWidth > 150 && kWidth < 500) setLiveKbsWidth(kWidth);
      } else if (isDraggingDocs) {
        const dWidth = Math.round(e.clientX - 64 - kbsWidthLive); // Adjust for GlobalNav + Kbs width
        if (dWidth > 200 && dWidth < 600) setLiveDocsWidth(dWidth);
      }
    };
    const handleMouseUp = () => {
      setIsDraggingAi(false);
      setIsDraggingKbs(false);
      setIsDraggingDocs(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isDraggingAi, isDraggingKbs, isDraggingDocs, kbsWidthLive]);

  // Persist drag results once the drag ends: the localStorage-backed width
  // states are written here (rounded, once per drag) instead of inside the
  // mousemove updater.
  useEffect(() => {
    if (isDraggingAi || isDraggingKbs || isDraggingDocs) {
      return;
    }
    if (liveAiWidth !== null) {
      setAiWidth(liveAiWidth);
      setLiveAiWidth(null);
    }
    if (liveKbsWidth !== null) {
      setKbsWidth(liveKbsWidth);
      setLiveKbsWidth(null);
    }
    if (liveDocsWidth !== null) {
      setDocsWidth(liveDocsWidth);
      setLiveDocsWidth(null);
    }
  }, [isDraggingAi, isDraggingKbs, isDraggingDocs, liveAiWidth, liveKbsWidth, liveDocsWidth, setAiWidth, setKbsWidth, setDocsWidth]);

  useEffect(() => {
    let cancelled = false;
    let tenantRetryAttempts = 0;

    const loadKnowledgeBases = async () => {
      if (isEphemeralFixedWorkspace && (!fixedKnowledgeBase || !fixedKnowledgeBaseId)) {
        setKbs({ team: [], personal: [], public: [] });
        setLoadingKbs(false);
        return;
      }

      if (!getKnowledgebaseTenantId()) {
        // The tenant id may not be materialized yet right after login. Retry with a
        // bounded, cancellable timer (50 attempts ≈ 15 s) so a missing tenant can never
        // produce an infinite polling loop after the component unmounts.
        if (kbRetryTimerRef.current === null && tenantRetryAttempts < 50) {
          tenantRetryAttempts += 1;
          kbRetryTimerRef.current = window.setTimeout(() => {
            kbRetryTimerRef.current = null;
            void loadKnowledgeBases();
          }, 300);
        } else {
          setLoadingKbs(false);
        }
        return;
      }

      if (fixedKnowledgeBase && fixedKnowledgeBaseId) {
        setKbs({ team: [], personal: [], public: [] });
        setLoadingKbs(false);
        handleSelectKb(fixedKnowledgeBase, true);
        return;
      }

      try {
        const data = await DocumentService.getKnowledgeBases();
        if (cancelled) {
          return;
        }

        setKbs(data);
        setLoadingKbs(false);

        const allKbs = [...(data.team || []), ...(data.personal || []), ...(data.public || [])];

        if (activeKb && allKbs.find(k => k.id === activeKb.id)) {
          handleSelectKb(activeKb, true);
        } else if (data.team && data.team.length > 0) {
          handleSelectKb(data.team[0]);
        } else if (data.personal && data.personal.length > 0) {
          handleSelectKb(data.personal[0]);
        } else if (data.public && data.public.length > 0) {
          handleSelectKb(data.public[0]);
        }
      } catch (error) {
        if (cancelled) {
          return;
        }
        toastKnowledgebaseError(error, t);
        setLoadingKbs(false);
      }
    };

    void loadKnowledgeBases();
    return () => {
      cancelled = true;
      if (kbRetryTimerRef.current !== null) {
        window.clearTimeout(kbRetryTimerRef.current);
        kbRetryTimerRef.current = null;
      }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSelectDoc = useCallback(async (doc: DocumentMeta, currentKbId?: string) => {
    if (!doc) return;
    setActiveDoc(doc);
    setSelectedDocIds(new Set()); // Clear multi-selection when clicking a single doc
    
    const kbId = currentKbId || activeKb?.id || doc.kbId;
    if (kbId) {
      tabCache.openDoc(kbId, doc);
      setOpenDocs(tabCache.getOpenDocs(kbId));
    } else {
      // Fallback
      if (doc.type !== 'folder') {
        setOpenDocs(prev => {
          if (prev.some(d => d.id === doc.id)) return prev;
          return [...prev, doc];
        });
      }
    }

    if (doc.type === 'richtext' || doc.type === 'code' || doc.type === 'markdown') {
      const seq = ++docRequestSeqRef.current;
      setIsDocLoading(true);
      setDocContent('');
      try {
        const content = await DocumentService.getDocumentContent(doc.id);
        if (seq !== docRequestSeqRef.current) {
          return;
        }
        setDocContent(content);
        setIsDocLoading(false);
      } catch (error) {
        if (seq !== docRequestSeqRef.current) {
          return;
        }
        toastKnowledgebaseError(error, t);
        setDocContent('');
        setIsDocLoading(false);
      }
    } else {
      docRequestSeqRef.current += 1;
      setDocContent('');
      setIsDocLoading(false);
    }
  }, [activeKb, tabCache, t]);

  const handleSelectKb = useCallback((kb: KnowledgeBase, preserveState = false) => {
    if (fixedKnowledgeBaseId && kb.id !== fixedKnowledgeBaseId) {
      return;
    }
    setActiveKb(kb);
    setActiveTab('kb'); // Force switch back to KB docs view upon selecting a KB
    tabCache.initKb(kb.id);
    
    const cachedDocs = tabCache.getOpenDocs(kb.id);
    const cachedActiveId = tabCache.getActiveDocId(kb.id);
    setOpenDocs(cachedDocs);
    setActiveDoc(null);
    setDocContent('');
    // Any in-flight document load for the previous space is now stale.
    docRequestSeqRef.current += 1;
    setIsDocLoading(false);

    setLoadingDocs(true);
    const kbSeq = ++kbRequestSeqRef.current;
    DocumentService.getDocuments(kb.id).then(data => {
      if (kbSeq !== kbRequestSeqRef.current) {
        return;
      }
      setDocs(data);
      setLoadingDocs(false);

      const flatDocs: DocumentMeta[] = [];
      const flatten = (nodes: any[]) => {
        nodes.forEach(n => {
           if (n.type === 'folder') {
              if (n.children) flatten(n.children);
           } else {
              flatDocs.push(n);
           }
        });
      }
      flatten(data);

      if (cachedActiveId) {
        const docToOpen = flatDocs.find(d => d.id === cachedActiveId);
        if (docToOpen) handleSelectDoc(docToOpen, kb.id);
      } else if (cachedDocs.length > 0) {
        const docToOpen = flatDocs.find(d => d.id === cachedDocs[0].id) || cachedDocs[0];
        handleSelectDoc(docToOpen, kb.id);
      } else if (!preserveState && data.length > 0) {
        const firstDoc = data.find(item => item.type !== 'folder') as DocumentMeta;
        if (firstDoc) {
          handleSelectDoc(firstDoc, kb.id);
        } else if (data[0].type === 'folder' && (data[0] as FolderNode).children?.length > 0) {
          handleSelectDoc((data[0] as FolderNode).children[0] as DocumentMeta, kb.id);
        }
      }

      if (persistWorkspaceState) {
        const intent = readKbNavIntent();
        if (intent?.highlight && intent.docId && intent.kbId === kb.id) {
          const located = findDocInTree(data, intent.docId);
          const parentId = located?.parentId ?? intent.parentId ?? null;
          window.setTimeout(() => {
            dispatchLocateKbFile({ docId: intent.docId!, parentId });
            clearKbNavIntent();
          }, 150);
        }
      }
    }).catch(error => {
      if (kbSeq !== kbRequestSeqRef.current) {
        return;
      }
      setLoadingDocs(false);
      toastKnowledgebaseError(error, t);
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fixedKnowledgeBaseId, handleSelectDoc, persistWorkspaceState, setActiveKb, setActiveDoc, setOpenDocs, tabCache]);

  const handleCloseDoc = useCallback((docId: string) => {
    if (!activeKb) return;
    const { remainingDocs, nextActiveId } = tabCache.closeDoc(activeKb.id, docId);
    setOpenDocs(remainingDocs);
    if (!nextActiveId) {
      setActiveDoc(null);
      setDocContent('');
    } else if (activeDoc?.id !== nextActiveId) {
      const nextDoc = remainingDocs.find(d => d.id === nextActiveId);
      if (nextDoc) handleSelectDoc(nextDoc, activeKb.id);
    }
  }, [activeKb, activeDoc, handleSelectDoc, tabCache]);

  const handleCloseOthers = useCallback((docId: string) => {
    if (!activeKb) return;
    const { remainingDocs } = tabCache.closeOthers(activeKb.id, docId);
    setOpenDocs(remainingDocs);
    if (!activeDoc || activeDoc.id !== docId) {
      const target = remainingDocs.find(d => d.id === docId);
      if (target) handleSelectDoc(target, activeKb.id);
    }
  }, [activeKb, activeDoc, handleSelectDoc, tabCache]);

  const handleCloseToRight = useCallback((docId: string) => {
    if (!activeKb) return;
    const { remainingDocs, nextActiveId } = tabCache.closeToRight(activeKb.id, docId);
    setOpenDocs(remainingDocs);
    if (!nextActiveId) {
       setActiveDoc(null);
       setDocContent('');
    } else if (activeDoc?.id !== nextActiveId) {
       const target = remainingDocs.find(d => d.id === nextActiveId);
       if (target) handleSelectDoc(target, activeKb.id);
    }
  }, [activeKb, activeDoc, handleSelectDoc, tabCache]);

  const handleCloseAll = useCallback(() => {
    if (activeKb) {
      tabCache.closeAll(activeKb.id);
    }
    // Invalidate any in-flight content fetch: without this, a response for the
    // previously active doc would land after the state was cleared.
    docRequestSeqRef.current += 1;
    setOpenDocs([]);
    setActiveDoc(null);
    setDocContent('');
  }, [activeKb, tabCache]);

  const handleTitleChange = useCallback(async (docId: string, newTitle: string) => {
    // Capture the previous title up front so a failed rename can revert the
    // optimistic update instead of leaving UI and server out of sync.
    const previousTitle = openDocs.find((d) => d.id === docId)?.title
      ?? docs.find((d) => d.id === docId)?.title;
    // 1. Update active list/state
    setActiveDoc(prev => (prev && prev.id === docId ? { ...prev, title: newTitle } : prev));
    setOpenDocs(prev => prev.map(d => (d.id === docId ? { ...d, title: newTitle } : d)));

    // 2. Update flat / deep document tree
    setDocs(prev => {
      const updateInTree = (items: any[]): any[] => {
        return items.map(item => {
          if (item.id === docId) {
            return { ...item, title: newTitle };
          }
          if (item.type === 'folder' && item.children) {
            return { ...item, children: updateInTree(item.children) };
          }
          return item;
        });
      };
      return updateInTree(prev);
    });

    // 3. Persist, with revert + toast on failure (this callback's promise is
    // fire-and-forget for callers, so a rejection must never go unhandled).
    try {
      await DocumentService.updateDocument(docId, { title: newTitle });
    } catch (error) {
      toastKnowledgebaseError(error, t);
      const revertInTree = (items: any[]): any[] => items.map(item => {
        if (item.id === docId && previousTitle !== undefined) {
          return { ...item, title: previousTitle };
        }
        if (item.type === 'folder' && item.children) {
          return { ...item, children: revertInTree(item.children) };
        }
        return item;
      });
      setActiveDoc(prev => (prev && prev.id === docId && previousTitle !== undefined ? { ...prev, title: previousTitle } : prev));
      setOpenDocs(prev => prev.map(d => (d.id === docId && previousTitle !== undefined ? { ...d, title: previousTitle } : d)));
      setDocs(revertInTree);
    }
  }, [docs, openDocs, t]);

  const handleCreateKb = async (gitUrl?: string, gitBranch?: string) => {
    if (isBlank(newKbTitle)) return;
    const newKbParams: Partial<KnowledgeBase> = {
      title: newKbTitle,
      icon: newKbIcon,
      type: newKbType,
    };
    try {
      const createdKb = await DocumentService.createKnowledgeBase(newKbParams);
      
      if (gitUrl) {
        await DocumentService.importGitRepository(createdKb.id, gitUrl, gitBranch);
      }
      
      setKbs(prev => ({
        ...prev,
        [newKbType]: [createdKb, ...prev[newKbType]]
      }));
      setNewKbTitle('');
      setNewKbIcon('📘');
      setIsCreateKbModalOpen(false);
    } catch (e) {
      toastKnowledgebaseError(e, t);
    }
  };

  const handleMenuCreate = async (actionType: string, parentId?: string, payload?: any) => {
    let titleStr = t('newDoc');
    let docType = 'richtext';
    
    if (actionType === 'localFile') { titleStr = t('localFile'); }
    if (actionType === 'localFile') { titleStr = t('localFile'); docType = 'file'; }
    if (actionType === 'localFolder') { titleStr = t('localFolder'); docType = 'folder'; }
    if (actionType === 'chat' || actionType === 'chat_file' || actionType === 'chat_dialog') { titleStr = actionType === 'chat_file' ? t('fromChatFile', { defaultValue: '聊天文件' }) : actionType === 'chat_dialog' ? t('fromChatDialog', { defaultValue: '聊天对话' }) : t('chatFile', { defaultValue: '聊天记录' }); docType = 'markdown'; }
    if (actionType === 'personalKb') { titleStr = t('importedPersonalKb'); docType = 'folder'; } // group them in a folder
    if (actionType === 'link') { titleStr = payload?.url || t('webLink'); docType = 'markdown'; }
    if (actionType === 'note_doc') { titleStr = t('newDoc'); docType = 'richtext'; }
    if (actionType === 'note_sheet') { titleStr = t('newSheet', { defaultValue: '表格' }); docType = 'richtext'; }
    if (actionType === 'cloudDrive') { titleStr = t('cloudDrive'); docType = 'folder'; } // import as folder
    if (actionType === 'notesApp') { titleStr = t('notesApp'); docType = 'richtext'; }
    if (actionType === 'audio') { titleStr = t('audioRecord'); docType = 'audio'; }
    if (actionType === 'code') { titleStr = t('codeSnippet'); docType = 'code'; }
    if (actionType === 'folder') { titleStr = t('newFolder'); docType = 'folder'; }

    try {
      // Every creation path below needs a selected knowledge base; `activeKb`
      // can be null while the KB list refreshes, and the non-null assertions
      // further down would turn that into a confusing TypeError toast.
      if (!activeKb) {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.KB_ID_REQUIRED);
      }
      const currentKbId = activeKb.id;
      let resultItem: any = null;
      if (actionType === 'batch_create' && Array.isArray(payload)) {
        assertKnowledgebasePreviewFeature('offline-import-batch-create');
        for (const item of payload) {
          const created = await DocumentService.createDocument({
            title: item.title,
            type: item.type,
            content: item.content || '',
            url: item.url,
            kbId: activeKb?.id,
            parentId: parentId || null
          });
          if (!resultItem) {
            resultItem = created;
          }
        }
      } else if ((actionType === 'localFile' || actionType === 'localFolder' || actionType === 'audioUpload' || actionType === 'musicUpload') && payload && payload.length > 0) {
        // Use custom type for music Upload
        const uploaded = await DocumentService.uploadFiles(payload, currentKbId, parentId, actionType === 'musicUpload' ? 'music' : undefined);
        if (uploaded && uploaded.length > 0) {
          resultItem = uploaded[0];
        }
      } else if (docType) {
        let contentUrl = undefined;
        let initialContent = '';

        if (actionType === 'audio' && shouldUseKnowledgebaseDemoFallback()) {
          contentUrl = '/demo.mp3';
        }
        if (actionType === 'link' && payload && payload.url) contentUrl = payload.url;

        if (actionType === 'notesApp' || actionType === 'chat' || actionType === 'chat_file' || actionType === 'chat_dialog') {
          assertKnowledgebasePreviewFeature(`offline-import-${actionType}`);
        }

        if (shouldUseKnowledgebaseDemoFallback()) {
          if (actionType === 'notesApp') {
            initialContent = '<h1>本周工作计划与总结</h1><p>这是一篇从备忘录同步过来的文件，已经自动转换为富文本格式，你可以继续编辑它。</p><ul><li>完成核心架构设计</li><li>重构前端交互细节</li></ul><p><br></p>';
          } else if (actionType === 'chat' || actionType === 'chat_file' || actionType === 'chat_dialog') {
            initialContent = '# 微信聊天记录同步\n\n**张三 (10:00)**: 大家都看一下这个文档，是明天的会议资料。\n\n**李四 (10:02)**: 收到！\n\n_系统提示：相关文件已经自动转存到知识库。_';
          } else if (actionType === 'link') {
            initialContent = `# [${payload?.url}](${payload?.url})\n\n> 正在抓取网页内容中，请稍候...`;
          }
        }

        if (actionType === 'link' && payload?.url && !shouldUseKnowledgebaseDemoFallback()) {
          // Real web ingest: fetches the page through the bounded importer
          // (SSRF-guarded, size-capped) and creates the document from the
          // retrieved content. The old branch created a placeholder markdown
          // doc whose "fetching, please wait" text was never replaced.
          const imported = await DocumentService.importWebLink({
            kbId: currentKbId,
            parentId: parentId || null,
            url: payload.url,
            title: titleStr,
          });
          resultItem = imported;
        } else {
          const newDocParams: Partial<DocumentMeta> = {
            title: titleStr,
            type: docType as any,
            url: contentUrl,
            content: initialContent,
            kbId: activeKb?.id,
            parentId
          };
          const createdDoc = await DocumentService.createDocument(newDocParams);
          resultItem = createdDoc;
        }
      }

      // Preserve the authoritative mutation result while the browser projection
      // catches up. A tree-refresh failure must not discard the creation
      // result: the document exists server-side, so keep it selected/returned.
      let updatedDocs;
      try {
        updatedDocs = await DocumentService.getDocuments(currentKbId);
      } catch (refreshError) {
        console.error('document tree refresh failed after create', refreshError);
        updatedDocs = null;
      }
      const selectableResult = resultItem?.type !== 'folder' ? resultItem : null;
      if (updatedDocs) {
        const nextDocs = selectableResult && !findDocInTree(updatedDocs, selectableResult.id)
          ? [...updatedDocs, selectableResult]
          : updatedDocs;
        setDocs(nextDocs);
      }
      if (selectableResult) {
        handleSelectDoc(selectableResult);
      }
      return resultItem;
    } catch (e) {
      toastKnowledgebaseError(e, t);
      return null;
    }
  };

  const toggleDocSelection = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    const newKeys = new Set(selectedDocIds);
    if (newKeys.has(id)) {
      newKeys.delete(id);
    } else {
      newKeys.add(id);
    }
    setSelectedDocIds(newKeys);
  };

  const handleImportCloudDrive = async (selectedItems: CloudDriveImportResultItem[]) => {
    if (!cloudDriveKb) return;
    try {
      if (!selectedItems || selectedItems.length === 0) {
        // Drive import already materializes browser nodes; no placeholder folder is needed.
      }

      if (activeKb && activeKb.id === cloudDriveKb.id) {
        setLoadingDocs(true);
        const refreshedDocs = await DocumentService.getDocuments(activeKb.id);
        setDocs(refreshedDocs);
        setLoadingDocs(false);
      }

      setCloudDriveKb(null);
      toast.success(t('importSuccess', { defaultValue: '从网盘中导入成功！' }));
    } catch (e) {
      toastKnowledgebaseError(e, t);
    }
  };

  if (isEphemeralFixedWorkspace && !fixedKnowledgeBaseId) {
    return <div className="flex-1 bg-[var(--color-kb-bg-app)]" />;
  }

  return (
    <div className="flex-1 flex overflow-hidden">
      {!isEphemeralFixedWorkspace && activeTab !== 'market' && (
        <KnowledgeBaseList
          kbs={kbs} loadingKbs={loadingKbs} activeKb={activeKb}
          width={kbsWidthLive} isDragging={isDraggingKbs} onMouseDownDrag={() => setIsDraggingKbs(true)}
          onSelectKb={handleSelectKb} 
          onCreateKbSelect={(type) => { setNewKbType(type); setIsCreateKbModalOpen(true); }} 
          onOpenSettings={(kb) => {
            DocumentService.hydrateKnowledgeBase(kb)
              .then((hydrated) => setSettingsKb(hydrated))
              .catch(() => setSettingsKb(kb));
          }}
          onOpenMarket={() => setIsMarketOpen(true)}
          onImportGit={(kb) => {
            setGitModalKb(kb);
            setGitModalMode('import');
          }}
          onSyncGit={(kb) => {
            setGitModalKb(kb);
            setGitModalMode('sync');
          }}
          onImportCloudDrive={(kb) => {
            handleSelectKb(kb);
            setCloudDriveKb(kb);
          }}
          onUpdateKbs={() => {
            DocumentService.getKnowledgeBases().then(data => {
              setKbs(data);
              if (activeKb) {
                const stillExists = [...(data.team || []), ...(data.personal || []), ...(data.public || [])].find(k => k.id === activeKb.id);
                if (!stillExists) {
                  setActiveKb(null);
                  setDocs([]);
                  setActiveDoc(null);
                } else {
                  setActiveKb(stillExists);
                }
              }
            }).catch(error => {
              toastKnowledgebaseError(error, t);
            });
          }}
        />
      )}

      {!isEphemeralFixedWorkspace && activeTab === 'market' ? (
        <KnowledgeBaseMarketView
          onSubscribedChange={() => {
            DocumentService.getKnowledgeBases().then(data => {
              setKbs(data);
            }).catch(error => {
              toastKnowledgebaseError(error, t);
            });
          }}
        />
      ) : (
        <>
          <KnowledgeFileList
            activeKb={activeKb} docs={docs} loadingDocs={loadingDocs} activeDoc={activeDoc}
            width={docsWidthLive} isDragging={isDraggingDocs} onMouseDownDrag={() => setIsDraggingDocs(true)}
            selectedDocIds={selectedDocIds} onSelectDoc={handleSelectDoc} 
            onToggleDocSelection={toggleDocSelection} 
            onClearSelection={() => setSelectedDocIds(new Set())} 
            onDeleteSelection={async () => {
              if (deleteSelectionInFlightRef.current) return;
              if (confirm(t('confirmDelete', { count: selectedDocIds.size, ns: 'common' }))) {
                deleteSelectionInFlightRef.current = true;
                try {
                  for (const id of selectedDocIds) {
                    await DocumentService.deleteDocument(id);
                  }
                  if (activeKb) {
                     setLoadingDocs(true);
                     const updatedDocs = await DocumentService.getDocuments(activeKb.id);
                     setDocs(updatedDocs);
                     setLoadingDocs(false);
                  }
                  if (activeDoc && selectedDocIds.has(activeDoc.id)) {
                     setActiveDoc(null);
                     setDocContent('');
                  }
                  setSelectedDocIds(new Set());
                } catch (error) {
                  toastKnowledgebaseError(error, t);
                } finally {
                  deleteSelectionInFlightRef.current = false;
                }
              }
            }}
            onMenuCreate={handleMenuCreate}
            onPublishDocs={(docs) => {
              if (isEphemeralFixedWorkspace) {
                return;
              }
              setPublishDocsContext(docs);
              setIsPublishModalOpen(true);
            }}
            publishEnabled={!isEphemeralFixedWorkspace}
            onUpdateDocs={() => {
              if (activeKb) {
                const refreshSeq = ++kbRequestSeqRef.current;
                DocumentService.getDocuments(activeKb.id).then(data => {
                  if (refreshSeq === kbRequestSeqRef.current) {
                    setDocs(data);
                  }
                });
              }
            }}
          />

          <EditorPanel 
            activeKb={activeKb} 
            activeDoc={activeDoc} 
            openDocs={openDocs}
            onSelectDoc={handleSelectDoc}
            onCloseDoc={handleCloseDoc}
            onCloseOthers={handleCloseOthers}
            onCloseToRight={handleCloseToRight}
            onCloseAll={handleCloseAll}
            onTitleChange={handleTitleChange}
            docContent={docContent}
            isDocLoading={isDocLoading}
            loadingDocs={loadingDocs}
            isAIOpen={isAIOpen}
            onToggleAI={() => setIsAIOpen(!isAIOpen)}
            onContentChange={handleContentChange}
            onUpdateDocs={() => {
              if (activeKb) {
                const refreshSeq = ++kbRequestSeqRef.current;
                DocumentService.getDocuments(activeKb.id).then(data => {
                  if (refreshSeq === kbRequestSeqRef.current) {
                    setDocs(data);
                  }
                });
              }
            }}
            onPublishDoc={(doc) => {
              if (isEphemeralFixedWorkspace) {
                return;
              }
              if (selectedDocIds.size > 0) {
                const docsToPublish: DocumentMeta[] = [];
                const findSelectedDocs = (items: (FolderNode | DocumentMeta)[]) => {
                  for (const item of items) {
                    if (selectedDocIds.has(item.id) && item.type !== 'folder') {
                      if (!docsToPublish.find(d => d.id === item.id)) {
                        docsToPublish.push(item as DocumentMeta);
                      }
                    }
                    if (item.type === 'folder' && (item as FolderNode).children) {
                      findSelectedDocs((item as FolderNode).children as any[]);
                    }
                  }
                };
                findSelectedDocs(docs);
                // Fallback if doc not in selection
                if (docsToPublish.length === 0) {
                  docsToPublish.push(doc);
                } else if (!docsToPublish.find(d => d.id === doc.id) && !selectedDocIds.has(doc.id)) {
                   // if the active doc isn't explicitly included/excluded, include it if it makes sense?
                   // Actually we'll just respect the selection.
                }
                setPublishDocsContext(docsToPublish);
              } else {
                setPublishDocsContext([doc]);
              }
              setIsPublishModalOpen(true);
            }}
            docs={docs}
            aiWidth={aiWidthLive}
            isDraggingAi={isDraggingAi}
            onMouseDownDragAi={() => setIsDraggingAi(true)}
            workspaceMode={effectiveWorkspaceMode}
            publishEnabled={!isEphemeralFixedWorkspace}
          />
        </>
      )}

      {!isEphemeralFixedWorkspace && isCreateKbModalOpen && (
        <CreateKbModal 
          newKbTitle={newKbTitle} setNewKbTitle={setNewKbTitle}
          newKbType={newKbType} setNewKbType={setNewKbType}
          newKbIcon={newKbIcon} setNewKbIcon={setNewKbIcon}
          onCancel={() => setIsCreateKbModalOpen(false)}
          onCreate={handleCreateKb}
        />
      )}

      {!isEphemeralFixedWorkspace && isPublishModalOpen && (
        <PublishModal 
          documents={publishDocsContext}
          onClose={() => {
            setIsPublishModalOpen(false);
            setPublishDocsContext([]);
          }}
          onWechatFlow={() => {
            setIsPublishModalOpen(false);
            navigate('/wechat-publish', { state: { documents: publishDocsContext } });
          }}
        />
      )}

      {!isEphemeralFixedWorkspace && settingsKb && (
        <KnowledgeBaseSettingsModal
          kb={settingsKb}
          onClose={() => setSettingsKb(null)}
          onSave={async (updates) => {
            try {
              const updated = await DocumentService.updateKnowledgeBase(settingsKb.id, updates);
              const data = await DocumentService.getKnowledgeBases();
              setKbs(data);
              if (activeKb && activeKb.id === settingsKb.id) {
                setActiveKb(updated);
              }
              setSettingsKb(null);
            } catch (e) {
              toastKnowledgebaseError(e, t);
            }
          }}
        />
      )}

      {!isEphemeralFixedWorkspace && isMarketOpen && (
        <KnowledgeBaseMarketModal
          onClose={() => setIsMarketOpen(false)}
          onSubscribedChange={() => {
            DocumentService.getKnowledgeBases().then(data => {
              setKbs(data);
              if (activeKb) {
                const stillExists = [...(data.team || []), ...(data.personal || []), ...(data.public || [])].find(k => k.id === activeKb.id);
                if (!stillExists) {
                  setActiveKb(null);
                  setDocs([]);
                  setActiveDoc(null);
                }
              }
            }).catch(error => {
              toastKnowledgebaseError(error, t);
            });
          }}
        />
      )}

      {!isEphemeralFixedWorkspace && gitModalKb && gitModalMode && (
        <GitIntegrationModal
          mode={gitModalMode}
          kb={gitModalKb}
          onClose={() => {
            setGitModalKb(null);
            setGitModalMode(null);
          }}
          onSuccess={() => {
            if (activeKb && activeKb.id === gitModalKb.id) {
              setLoadingDocs(true);
              DocumentService.getDocuments(activeKb.id).then(data => {
                setDocs(data);
                setLoadingDocs(false);
              }).catch(error => {
                setLoadingDocs(false);
                toastKnowledgebaseError(error, t);
              });
            }
          }}
        />
      )}

      {!isEphemeralFixedWorkspace && cloudDriveKb && (
        <CloudDriveModal
          isOpen={!!cloudDriveKb}
          onClose={() => setCloudDriveKb(null)}
          spaceId={cloudDriveKb.id}
          onConfirm={handleImportCloudDrive}
        />
      )}
    </div>
  );
}
