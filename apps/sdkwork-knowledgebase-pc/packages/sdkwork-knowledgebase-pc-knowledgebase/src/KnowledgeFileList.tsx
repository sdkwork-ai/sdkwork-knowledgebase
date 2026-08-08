import React, { useRef, useState, useEffect, useMemo, useCallback } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { isBlank } from '@sdkwork/utils';
import { KnowledgeFileItem } from './KnowledgeFileItem';
import { X, FileUp, Link, FileEdit, ChevronRight, FileText, FolderPlus, Trash2, BookOpen } from 'lucide-react';
import { FolderNode, DocumentMeta, KnowledgeBase, DocumentService } from './services/document';
import { isKnowledgebaseApiAvailable, KnowledgebaseErrorCodes, assertKnowledgebasePreviewFeature, throwKnowledgebaseError } from 'sdkwork-knowledgebase-pc-core';
import { toast } from './components/ui/toast-manager';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';
import { useTranslation } from 'react-i18next';

import { LinkModal } from './LinkModal';
import { CloudDriveModal } from './CloudDriveModal';
import { NotesAppModal } from './NotesAppModal';
import { PersonalKbModal } from './PersonalKbModal';
import { ChatFileModal } from './ChatFileModal';
import { ChatDialogModal } from './ChatDialogModal';
import { KnowledgeFileHeader } from './KnowledgeFileHeader';
import { MoveCopyModal } from './MoveCopyModal';
import { TagsModal } from './TagsModal';
import { PermissionsModal } from './PermissionsModal';
import { VersionHistoryModal } from './VersionHistoryModal';

const FILE_ROW_HEIGHT = 68;
const VIRTUALIZE_THRESHOLD = 50;

export interface KnowledgeFileListProps {
  activeKb: KnowledgeBase | null;
  docs: (FolderNode | DocumentMeta)[];
  loadingDocs: boolean;
  activeDoc: DocumentMeta | null;
  selectedDocIds: Set<string>;
  onSelectDoc: (doc: DocumentMeta) => void;
  onToggleDocSelection: (e: React.MouseEvent, id: string) => void;
  onClearSelection: () => void;
  onDeleteSelection: () => void;
  onMenuCreate: (actionType: string, parentId?: string, payload?: any) => Promise<any> | void;
  onPublishDocs: (docsToPublish: DocumentMeta[]) => void;
  publishEnabled?: boolean;
  onUpdateDocs?: () => void; // Call this when tree dropped to refresh data
  width?: number;
  isDragging?: boolean;
  onMouseDownDrag?: () => void;
}

export function KnowledgeFileList({
  activeKb, docs, loadingDocs, activeDoc, selectedDocIds,
  onSelectDoc, onToggleDocSelection, onClearSelection, onDeleteSelection, onMenuCreate, onPublishDocs, onUpdateDocs,
  publishEnabled = true,
  width = 260, isDragging, onMouseDownDrag
}: KnowledgeFileListProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [isAddMenuOpen, setIsAddMenuOpen] = useState(false);
  const [isMultiSelectMode, setIsMultiSelectMode] = useState(false);
  const addMenuRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const audioInputRef = useRef<HTMLInputElement>(null);
  const musicInputRef = useRef<HTMLInputElement>(null);
  const folderInputRef = useRef<HTMLInputElement>(null);
  const [renameItem, setRenameItem] = useState<{ id: string, title: string, type: string } | null>(null);
  const [currentFolderId, setCurrentFolderId] = useState<string | null>(null);

  // Modals for actions
  const [isLinkModalOpen, setIsLinkModalOpen] = useState(false);
  const [linkUrl, setLinkUrl] = useState('');
  const [isCloudDriveOpen, setIsCloudDriveOpen] = useState(false);
  const [isNotesAppOpen, setIsNotesAppOpen] = useState(false);
  const [isPersonalKbOpen, setIsPersonalKbOpen] = useState(false);
  const [isChatFileOpen, setIsChatFileOpen] = useState(false);
  const [isChatDialogOpen, setIsChatDialogOpen] = useState(false);
  const [moveCopyConfig, setMoveCopyConfig] = useState<{ action: 'move' | 'copy'; item: any } | null>(null);
  const [actionModal, setActionModal] = useState<{ action: string; item: any; text?: string } | null>(null);
  const [highlightDocId, setHighlightDocId] = useState<string | null>(null);

  useEffect(() => {
    if (!activeKb?.id || !currentFolderId || !onUpdateDocs) {
      return;
    }
    let cancelled = false;
    DocumentService.ensureFolderChildrenLoaded(activeKb.id, currentFolderId)
      .then(() => {
        if (!cancelled) {
          onUpdateDocs();
        }
      })
      .catch(() => {
        // Folder lazy-load is best-effort; the current view remains usable.
      });
    return () => {
      cancelled = true;
    };
  }, [activeKb?.id, currentFolderId, onUpdateDocs]);

  useEffect(() => {
    const handleAction = (e: CustomEvent) => {
      const detail = e.detail;
      if (detail.action === 'pin') {
        const newPinnedState = !detail.item.isPinned;
        DocumentService.updateDocument(detail.item.id, { isPinned: newPinnedState }).then(() => onUpdateDocs && onUpdateDocs());
        setActionModal({ action: 'toast', item: detail.item, text: newPinnedState ? t('pinned') : t('unpinned') });
        setTimeout(() => setActionModal(null), 2000);
      } else if (detail.action === 'split') {
        // Split view is unsupported for stale menu events; ignore them.
      } else {
        setActionModal(detail);
      }
    };
    window.addEventListener('kb-action', handleAction as EventListener);
    return () => window.removeEventListener('kb-action', handleAction as EventListener);
  }, [onUpdateDocs]);

  const handleMenuCreateWrapped = async (actionType: string, parentId?: string, payload?: any) => {
    const newItem = await onMenuCreate(actionType, parentId, payload);
    if (newItem) {
      if (parentId && parentId !== currentFolderId) {
         setCurrentFolderId(parentId);
      }
      setTimeout(() => {
        setRenameItem({ id: newItem.id, title: newItem.title, type: newItem.type });
      }, 100);
    }
  };

  const { currentFolder, ancestors } = useMemo(() => {
    if (!currentFolderId) return { currentFolder: null, ancestors: [] };
    const get = (items: any[], id: string, path: any[]): any => {
       for (const item of items) {
           if (item.id === id) return { currentFolder: item, ancestors: path };
           if (item.type === 'folder' && item.children) {
               const res = get(item.children, id, [...path, item]);
               if (res) return res;
           }
       }
       return null;
    };
    return get(docs, currentFolderId, []) || { currentFolder: null, ancestors: [] };
  }, [docs, currentFolderId]);

  const currentDocs = useMemo(() => {
    let rawList = currentFolder ? currentFolder.children || [] : docs;
    
    // Sort pinned items to the top
    rawList = [...rawList].sort((a, b) => {
      if (a.isPinned && !b.isPinned) return -1;
      if (!a.isPinned && b.isPinned) return 1;
      const orderA = a.order ?? Number.MAX_SAFE_INTEGER;
      const orderB = b.order ?? Number.MAX_SAFE_INTEGER;
      if (orderA !== orderB) return orderA - orderB;
      return 0;
    });

    if (isBlank(searchQuery)) return rawList;
    
    const matchQuery = searchQuery.toLowerCase().trim();
    const filter = (items: any[]): any[] => {
      const result: any[] = [];
      for (const item of items) {
        const matches = item.title.toLowerCase().includes(matchQuery);
        let childrenMatches: any[] = [];
        if (item.type === 'folder' && item.children) {
          childrenMatches = filter(item.children);
        }
        if (matches || childrenMatches.length > 0) {
          result.push({
            ...item,
            children: item.type === 'folder' ? childrenMatches : undefined
          });
        }
      }
      return result;
    };
    return filter(rawList);
  }, [docs, currentFolder, searchQuery]);

  const listScrollRef = useRef<HTMLDivElement>(null);
  const shouldVirtualize = currentDocs.length > VIRTUALIZE_THRESHOLD;
  const virtualizer = useVirtualizer({
    count: currentDocs.length,
    getScrollElement: () => listScrollRef.current,
    estimateSize: () => FILE_ROW_HEIGHT,
    overscan: 8});

  const renderFileItem = useCallback((item: FolderNode | DocumentMeta) => (
    <KnowledgeFileItem
      item={item}
      activeDoc={activeDoc}
      selectedDocIds={selectedDocIds}
      isMultiSelectMode={isMultiSelectMode}
      onToggleDocSelection={onToggleDocSelection}
      setCurrentFolderId={setCurrentFolderId}
      onSelectDoc={onSelectDoc}
      onMenuCreate={handleMenuCreateWrapped}
      fileInputRef={fileInputRef}
      audioInputRef={audioInputRef}
      musicInputRef={musicInputRef}
      folderInputRef={folderInputRef}
      setIsCloudDriveOpen={setIsCloudDriveOpen}
      setIsLinkModalOpen={setIsLinkModalOpen}
      setLinkUrl={setLinkUrl}
      renameItem={renameItem}
      setRenameItem={setRenameItem}
      onUpdateDocs={onUpdateDocs}
      onMoveItem={(moveItem) => setMoveCopyConfig({ action: 'move', item: moveItem })}
      onCopyItem={(copyItem) => setMoveCopyConfig({ action: 'copy', item: copyItem })}
      isLocateHighlight={highlightDocId === item.id}
      t={t}
    />
  ), [
    activeDoc,
    selectedDocIds,
    isMultiSelectMode,
    onToggleDocSelection,
    onSelectDoc,
    handleMenuCreateWrapped,
    renameItem,
    onUpdateDocs,
    highlightDocId,
    t,
  ]);

  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<{ docId: string; parentId?: string | null }>).detail;
      if (!detail?.docId) return;
      if (detail.parentId) {
        setCurrentFolderId(detail.parentId);
      } else {
        setCurrentFolderId(null);
      }
      setHighlightDocId(detail.docId);
      window.setTimeout(() => {
        if (currentDocs.length > VIRTUALIZE_THRESHOLD) {
          const index = currentDocs.findIndex((item: FolderNode | DocumentMeta) => item.id === detail.docId);
          if (index >= 0) {
            virtualizer.scrollToIndex(index, { align: 'auto', behavior: 'smooth' });
            return;
          }
        }
        document.getElementById(`kb-file-item-${detail.docId}`)?.scrollIntoView({
          behavior: 'smooth',
          block: 'nearest'
        });
      }, 100);
      window.setTimeout(() => setHighlightDocId(null), 2400);
    };
    window.addEventListener('kb-locate-file', handler);
    return () => window.removeEventListener('kb-locate-file', handler);
  }, [currentDocs, virtualizer]);

  const treeContainerRef = useRef<HTMLDivElement>(null);
  const [treeSize, setTreeSize] = useState({ width: 260, height: 600 });

  useEffect(() => {
    if (!treeContainerRef.current) return;
    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        if (entry.contentRect.height > 0 || entry.contentRect.width > 0) {
          setTreeSize({ width: entry.contentRect.width, height: entry.contentRect.height });
        }
      }
    });
    observer.observe(treeContainerRef.current);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (shouldVirtualize) {
      virtualizer.measure();
    }
  }, [shouldVirtualize, treeSize.width, treeSize.height, currentDocs.length, virtualizer]);

  return (
    <div 
      className="flex flex-col bg-[var(--color-kb-editor)] border-r border-[var(--color-kb-panel-border)] overflow-hidden flex-shrink-0 relative"
      style={{ width }}
    >
      <KnowledgeFileHeader 
        activeKb={activeKb}
        isSearchOpen={isSearchOpen} setIsSearchOpen={setIsSearchOpen}
        searchQuery={searchQuery} setSearchQuery={setSearchQuery}
        isAddMenuOpen={isAddMenuOpen} setIsAddMenuOpen={setIsAddMenuOpen}
        isMultiSelectMode={isMultiSelectMode} setIsMultiSelectMode={setIsMultiSelectMode}
        addMenuRef={addMenuRef} fileInputRef={fileInputRef} audioInputRef={audioInputRef} musicInputRef={musicInputRef} folderInputRef={folderInputRef}
        currentFolderId={currentFolderId}
        onMenuCreate={handleMenuCreateWrapped}
        setIsLinkModalOpen={setIsLinkModalOpen} setLinkUrl={setLinkUrl}
        setIsCloudDriveOpen={setIsCloudDriveOpen} setIsNotesAppOpen={setIsNotesAppOpen} setIsPersonalKbOpen={setIsPersonalKbOpen} 
        setIsChatFileOpen={setIsChatFileOpen} setIsChatDialogOpen={setIsChatDialogOpen}
        setRenameItem={setRenameItem}
      />

      <div className="flex-1 flex flex-col overflow-hidden px-0 relative bg-[color-mix(in_srgb,var(--color-kb-panel)_12%,var(--color-kb-editor))]">
         {selectedDocIds.size > 0 && isMultiSelectMode && (
            <div className="flex-none z-10 mx-1 mb-2 py-2 px-3 bg-[var(--color-kb-panel-active)] border border-[var(--color-kb-editor-border)] rounded-lg text-[var(--color-kb-panel-text)] text-xs flex justify-between items-center shadow-md animate-in fade-in slide-in-from-top-2">
               <span className="font-medium">{t('selectedItems', { ns: 'common', count: selectedDocIds.size })}</span>
               <div className="flex items-center space-x-3">
                  {publishEnabled && (
                    <button onClick={() => {
                    const docsToPublish: DocumentMeta[] = [];
                    const findSelectedDocs = (items: (FolderNode | DocumentMeta)[], publishAll: boolean = false) => {
                      for (const item of items) {
                        const isSelected = publishAll || selectedDocIds.has(item.id);
                        if (isSelected && item.type !== 'folder') {
                          // Prevent duplicate pushes if it's both parent-selected and self-selected
                          if (!docsToPublish.find(d => d.id === item.id)) {
                             docsToPublish.push(item as DocumentMeta);
                          }
                        }
                        if (item.type === 'folder' && (item as FolderNode).children) {
                          findSelectedDocs((item as FolderNode).children!, isSelected);
                        }
                      }
                    };
                    findSelectedDocs(docs);
                    if (docsToPublish.length > 0) {
                      onPublishDocs(docsToPublish);
                    }
                    }} className="hover:text-[var(--color-kb-accent)] transition-colors"><FileUp size={14}/></button>
                  )}
                  <button onClick={onDeleteSelection} className="hover:text-red-500 transition-colors" title={t('delete', { ns: 'common' })}><Trash2 size={14}/></button>
                  <button onClick={onClearSelection} className="hover:text-[var(--color-kb-text-muted)] transition-colors"><X size={14}/></button>
               </div>
            </div>
         )}
        
         {currentFolderId && (
            <div className="flex items-center px-4 py-2 mb-1 text-[13px] text-[var(--color-kb-text-muted)] sticky top-0 bg-[var(--color-kb-editor)] z-10 mx-0 flex-none overflow-x-auto no-scrollbar whitespace-nowrap border-b border-[var(--color-kb-panel-border)]/50">
               <button 
                  onClick={() => setCurrentFolderId(null)}
                  className="hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] px-1.5 py-0.5 rounded transition-colors flex items-center shrink-0 max-w-[80px] truncate"
               >
                  {activeKb?.title || 'Root'}
               </button>
               {ancestors.map((anc: any) => (
                  <React.Fragment key={anc.id}>
                     <ChevronRight size={13} className="mx-0 shrink-0 text-[var(--color-kb-text-muted)] opacity-50" />
                     <button 
                        onClick={() => setCurrentFolderId(anc.id)}
                        className="hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] px-1.5 py-0.5 rounded transition-colors truncate max-w-[80px] shrink-0"
                     >
                        {anc.title}
                     </button>
                  </React.Fragment>
               ))}
               <ChevronRight size={13} className="mx-0 shrink-0 text-[var(--color-kb-text-muted)] opacity-50" />
               <span className="text-[var(--color-kb-text-heading)] font-medium min-w-0 truncate flex-1 px-1.5 py-0.5">{currentFolder?.title}</span>
            </div>
         )}

        <div ref={treeContainerRef} className="flex-1 flex flex-col min-h-0 min-w-0 space-y-0.5 overflow-hidden kb-tree-container relative w-full">
          {loadingDocs ? (
            <div className="px-3 py-4 text-xs font-medium text-zinc-400 dark:text-[var(--color-kb-text-muted)] animate-pulse">{t('loading', { ns: 'common' })}</div>
          ) : !activeKb ? (
            <div className="flex flex-col items-center justify-center h-full text-center px-6 select-none bg-gradient-to-b from-transparent to-zinc-50 dark:to-[var(--color-kb-panel-border)]/5">
              <div className="w-16 h-16 rounded-2xl bg-white dark:bg-[var(--color-kb-editor)] flex items-center justify-center mb-5 border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/40 shadow-sm relative overflow-hidden group">
                <div className="absolute inset-0 bg-gradient-to-br from-indigo-50/50 dark:from-[var(--color-kb-panel-hover)] to-transparent opacity-50 group-hover:opacity-100 transition-opacity"></div>
                <BookOpen size={28} className="text-zinc-300 dark:text-[var(--color-kb-text-muted)] opacity-80 relative z-10 group-hover:scale-110 transition-transform duration-300" strokeWidth={2} />
              </div>
              <h4 className="text-[14px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] mb-1.5 tracking-tight">{t('noKbSelected')}</h4>
              <p className="text-[12px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] leading-relaxed max-w-[200px]">{t('selectKbPrompt')}</p>
            </div>
          ) : currentDocs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-center px-6 select-none py-10">
              <div className="w-12 h-12 rounded-xl bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60 flex items-center justify-center mb-3.5 relative overflow-hidden">
                <FileText size={20} className="text-zinc-400 dark:text-zinc-500" strokeWidth={1.8} />
              </div>
              <h4 className="text-[13px] font-bold text-zinc-900 dark:text-[var(--color-kb-text-heading)] mb-1 tracking-tight">{t('noFiles')}</h4>
              <p className="text-[11px] text-zinc-400 dark:text-[var(--color-kb-text-muted)] leading-relaxed max-w-[200px] mb-6">{t('addFilesPrompt')}</p>
              
              <div className="grid grid-cols-2 gap-2 w-full max-w-[240px] px-1">
                <button
                  type="button"
                  onClick={() => handleMenuCreateWrapped('note_doc', currentFolderId ?? undefined)}
                  className="p-3 bg-zinc-50/50 hover:bg-zinc-100/60 active:bg-zinc-150 dark:bg-zinc-900/30 dark:hover:bg-zinc-900/80 dark:active:bg-zinc-900 text-zinc-700 dark:text-zinc-300 border border-zinc-200/80 dark:border-zinc-800/80 rounded-lg flex flex-col items-center justify-center gap-1.5 transition-colors duration-150 cursor-pointer"
                >
                  <FileEdit size={14} className="text-indigo-500/85 dark:text-indigo-400/85" />
                  <span className="text-[11px] font-medium">{t('note')}</span>
                </button>

                <button
                  type="button"
                  onClick={() => fileInputRef.current?.click()}
                  className="p-3 bg-zinc-50/50 hover:bg-zinc-100/60 active:bg-zinc-150 dark:bg-zinc-900/30 dark:hover:bg-zinc-900/80 dark:active:bg-zinc-900 text-zinc-700 dark:text-zinc-300 border border-zinc-200/80 dark:border-zinc-800/80 rounded-lg flex flex-col items-center justify-center gap-1.5 transition-colors duration-150 cursor-pointer"
                >
                  <FileUp size={14} className="text-blue-500/85 dark:text-blue-400/85" />
                  <span className="text-[11px] font-medium">{t('localFile')}</span>
                </button>

                <button
                  type="button"
                  onClick={() => handleMenuCreateWrapped('folder', currentFolderId ?? undefined)}
                  className="p-3 bg-zinc-50/50 hover:bg-zinc-100/60 active:bg-zinc-150 dark:bg-zinc-900/30 dark:hover:bg-zinc-900/80 dark:active:bg-zinc-900 text-zinc-700 dark:text-zinc-300 border border-zinc-200/80 dark:border-zinc-800/80 rounded-lg flex flex-col items-center justify-center gap-1.5 transition-colors duration-150 cursor-pointer"
                >
                  <FolderPlus size={14} className="text-emerald-500/85 dark:text-emerald-400/85" />
                  <span className="text-[11px] font-medium">{t('newFolder')}</span>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setLinkUrl('');
                    setIsLinkModalOpen(true);
                  }}
                  className="p-3 bg-zinc-50/50 hover:bg-zinc-100/60 active:bg-zinc-150 dark:bg-zinc-900/30 dark:hover:bg-zinc-900/80 dark:active:bg-zinc-900 text-zinc-700 dark:text-zinc-300 border border-zinc-200/80 dark:border-zinc-800/80 rounded-lg flex flex-col items-center justify-center gap-1.5 transition-colors duration-150 cursor-pointer"
                >
                  <Link size={14} className="text-amber-500/85 dark:text-amber-400/85" />
                  <span className="text-[11px] font-medium">{t('webLink')}</span>
                </button>
              </div>
            </div>
          ) : (
            <div
              ref={listScrollRef}
              className="flex-1 hover-scrollbar overflow-y-auto h-full w-full px-0"
            >
              {shouldVirtualize ? (
                <div
                  style={{
                    height: virtualizer.getTotalSize(),
                    position: 'relative',
                    width: '100%'}}
                >
                  {virtualizer.getVirtualItems().map((virtualRow) => {
                    const item = currentDocs[virtualRow.index];
                    return (
                      <div
                        key={item.id}
                        className="kb-file-list-row"
                        style={{
                          position: 'absolute',
                          top: 0,
                          left: 0,
                          width: '100%',
                          transform: `translateY(${virtualRow.start}px)`}}
                      >
                        {renderFileItem(item)}
                      </div>
                    );
                  })}
                </div>
              ) : (
                currentDocs.map((item: FolderNode | DocumentMeta) => (
                  <div
                    key={item.id}
                    className="kb-file-list-row"
                    style={{ contentVisibility: 'auto', containIntrinsicSize: `0 ${FILE_ROW_HEIGHT}px` }}
                  >
                    {renderFileItem(item)}
                  </div>
                ))
              )}
              <div className="h-6 w-full shrink-0"></div>
            </div>
          )}
        </div>
      </div>

      <div 
        className={`absolute top-0 right-[-3px] w-[6px] h-full cursor-col-resize z-20 group ${isDragging ? 'bg-[var(--color-kb-accent)]/20' : 'hover:bg-[var(--color-kb-accent)]/10'}`}
        onMouseDown={onMouseDownDrag}
      >
        <div className={`absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[2px] h-8 rounded-full ${isDragging ? 'bg-[var(--color-kb-accent)]' : 'bg-transparent group-hover:bg-[var(--color-kb-accent)]/50'}`} />
      </div>

      <LinkModal
        isOpen={isLinkModalOpen}
        onClose={() => setIsLinkModalOpen(false)}
        linkUrl={linkUrl}
        setLinkUrl={setLinkUrl}
        onConfirm={() => { handleMenuCreateWrapped('link', currentFolderId ?? undefined, { url: linkUrl }); setIsLinkModalOpen(false); }}
      />
      
      <CloudDriveModal
        isOpen={isCloudDriveOpen}
        onClose={() => setIsCloudDriveOpen(false)}
        spaceId={activeKb?.id}
        targetParentFolderId={currentFolderId}
        onConfirm={async (selectedItems) => {
          setIsCloudDriveOpen(false);
          if (selectedItems.length > 0) {
            onUpdateDocs?.();
            toast.success(t('importSuccess', { defaultValue: '从网盘中导入成功！' }));
          }
        }}
      />
      
      <NotesAppModal
        isOpen={isNotesAppOpen}
        onClose={() => setIsNotesAppOpen(false)}
        onConfirm={async (selectedItems) => {
          try {
            if (selectedItems && selectedItems.length > 0) {
              assertKnowledgebasePreviewFeature('notes-app-import');
              await handleMenuCreateWrapped('batch_create', currentFolderId ?? undefined, selectedItems);
            } else {
              await handleMenuCreateWrapped('notesApp', currentFolderId ?? undefined);
            }
          } catch (error) {
            toastKnowledgebaseError(error, t);
          }
          setIsNotesAppOpen(false);
        }}
      />
      
      <PersonalKbModal
        isOpen={isPersonalKbOpen}
        onClose={() => setIsPersonalKbOpen(false)}
        onConfirm={async (selectedItems) => {
          if (selectedItems && selectedItems.length > 0) {
            if (isKnowledgebaseApiAvailable()) {
              try {
                if (!activeKb?.id) {
                  throwKnowledgebaseError(KnowledgebaseErrorCodes.KB_ID_REQUIRED);
                }
                for (const item of selectedItems) {
                  if (!item.id) {
                    throwKnowledgebaseError(KnowledgebaseErrorCodes.IMPORT_DOCUMENT_ID_MISSING);
                  }
                  await DocumentService.copyDocument(
                    item.id,
                    activeKb.id,
                    currentFolderId ?? null,
                  );
                }
                onUpdateDocs?.();
                toast.success(t('importSuccess', { defaultValue: '导入成功' }));
              } catch (error) {
                toastKnowledgebaseError(error, t);
              }
            } else {
              await handleMenuCreateWrapped('batch_create', currentFolderId ?? undefined, selectedItems);
            }
          } else {
            await handleMenuCreateWrapped('personalKb', currentFolderId ?? undefined);
          }
          setIsPersonalKbOpen(false);
        }}
      />
      
      <ChatFileModal
        isOpen={isChatFileOpen}
        onClose={() => setIsChatFileOpen(false)}
        onConfirm={async (selectedItems) => {
          try {
            if (selectedItems && selectedItems.length > 0) {
              assertKnowledgebasePreviewFeature('chat-file-import');
              await handleMenuCreateWrapped('batch_create', currentFolderId ?? undefined, selectedItems);
            } else {
              await handleMenuCreateWrapped('chat_file', currentFolderId ?? undefined);
            }
          } catch (error) {
            toastKnowledgebaseError(error, t);
          }
          setIsChatFileOpen(false);
        }}
      />

      <ChatDialogModal
        isOpen={isChatDialogOpen}
        onClose={() => setIsChatDialogOpen(false)}
        onConfirm={async (selectedItems) => {
          try {
            if (selectedItems && selectedItems.length > 0) {
              assertKnowledgebasePreviewFeature('chat-dialog-import');
              await handleMenuCreateWrapped('batch_create', currentFolderId ?? undefined, selectedItems);
            } else {
              await handleMenuCreateWrapped('chat_dialog', currentFolderId ?? undefined);
            }
          } catch (error) {
            toastKnowledgebaseError(error, t);
          }
          setIsChatDialogOpen(false);
        }}
      />

      {moveCopyConfig && (
        <MoveCopyModal
          action={moveCopyConfig.action}
          item={moveCopyConfig.item as any}
          activeKb={activeKb}
          onClose={() => setMoveCopyConfig(null)}
          onSubmit={async (targetKbId, targetFolderId) => {
            const sourceKbId =
              ('kbId' in moveCopyConfig.item ? moveCopyConfig.item.kbId : null)
              ?? activeKb?.id
              ?? null;

            const flatDocs: (FolderNode | DocumentMeta)[] = [];
            const traverseDocs = (items: (FolderNode | DocumentMeta)[]) => {
              items.forEach((item) => {
                flatDocs.push(item);
                if (item.type === 'folder' && 'children' in item) {
                  traverseDocs(item.children);
                }
              });
            };
            traverseDocs(docs);

            try {
              if (moveCopyConfig.action === 'move') {
                if (isKnowledgebaseApiAvailable()) {
                  const updates = sourceKbId && targetKbId !== sourceKbId
                    ? { kbId: targetKbId, parentId: targetFolderId }
                    : { parentId: targetFolderId };
                  await DocumentService.updateDocument(moveCopyConfig.item.id, updates);
                } else {
                  const moveItem = async (originalItem: FolderNode | DocumentMeta, newParentId: string | null) => {
                    const updates = sourceKbId && targetKbId !== sourceKbId
                      ? { kbId: targetKbId, parentId: newParentId }
                      : { parentId: newParentId };
                    await DocumentService.updateDocument(originalItem.id, updates);
                    const children = flatDocs.filter((doc) => doc.parentId === originalItem.id);
                    for (const child of children) {
                      await moveItem(child, originalItem.id);
                    }
                  };
                  await moveItem(moveCopyConfig.item as FolderNode | DocumentMeta, targetFolderId);
                }
              } else {
                const copyItem = async (originalItem: FolderNode | DocumentMeta, newParentId: string | null) => {
                  if (isKnowledgebaseApiAvailable()) {
                    await DocumentService.copyDocument(
                      originalItem.id,
                      targetKbId,
                      newParentId,
                      {
                        titleSuffix: originalItem.id === moveCopyConfig.item.id ? t('copySuffix') : undefined},
                    );
                    return;
                  }

                  const newDoc = await DocumentService.createDocument({
                    ...originalItem,
                    title: originalItem.id === moveCopyConfig.item.id
                      ? `${originalItem.title}${t('copySuffix')}`
                      : originalItem.title,
                    kbId: targetKbId,
                    parentId: newParentId});
                  const children = flatDocs.filter((doc) => doc.parentId === originalItem.id);
                  for (const child of children) {
                    await copyItem(child, newDoc.id);
                  }
                };
                await copyItem(moveCopyConfig.item as FolderNode | DocumentMeta, targetFolderId);
              }
              setMoveCopyConfig(null);
              onUpdateDocs?.();
            } catch (error) {
              toastKnowledgebaseError(error, t);
            }
          }}
        />
      )}

      {actionModal && actionModal.action === 'toast' && (
        <div className="fixed bottom-6 right-6 bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] shadow-xl rounded-xl px-5 py-3.5 z-[1000] flex items-center animate-in slide-in-from-bottom-5 fade-in duration-300">
           <span className="text-[13px] text-white font-medium">{actionModal.text}</span>
        </div>
      )}
      
      <TagsModal
        isOpen={actionModal?.action === 'tags'}
        item={actionModal ? actionModal.item : null}
        onClose={() => setActionModal(null)}
        onSave={async (tagInput) => {
          if (actionModal?.item?.id) {
            const tagsArray = tagInput.split(',').map(s => s.trim()).filter(Boolean);
            await DocumentService.updateDocument(actionModal.item.id, { tags: tagsArray });
            if (onUpdateDocs) onUpdateDocs();
          }
          setActionModal({ action: 'toast', item: actionModal!.item, text: t('tagsSaved') });
          setTimeout(() => setActionModal(null), 2000);
        }}
      />

      <PermissionsModal
        isOpen={actionModal?.action === 'permissions'}
        item={actionModal ? { ...actionModal.item, kbId: activeKb?.id } : null}
        onClose={() => setActionModal(null)}
      />

      <VersionHistoryModal
        isOpen={actionModal?.action === 'history'}
        item={actionModal ? actionModal.item : null}
        onClose={() => setActionModal(null)}
      />

    </div>
  );
}
