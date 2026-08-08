import { createRuntimeConfig } from 'sdkwork-knowledgebase-pc-core';
import React from 'react';
import { MoreHorizontal, Folder, Image as ImageIcon, Pin } from 'lucide-react';
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent } from './components/ui/dropdown-menu';
import { ContextMenu, ContextMenuTrigger, ContextMenuContent } from './components/ui/context-menu';
import { NodeDropdownItems, NodeContextItems } from './NodeMenuContent';
import { DocumentMeta, DocumentService } from './services/document';
import type { ReactKeyedComponentProps } from '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps';

export interface KnowledgeFileItemProps extends ReactKeyedComponentProps {
  item: any;
  activeDoc: DocumentMeta | null;
  selectedDocIds: Set<string>;
  isMultiSelectMode: boolean;
  onToggleDocSelection: (e: React.MouseEvent, id: string) => void;
  setCurrentFolderId: (id: string) => void;
  onSelectDoc: (doc: DocumentMeta) => void;
  onMenuCreate: (actionType: string, parentId?: string, payload?: any) => void;
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  audioInputRef: React.RefObject<HTMLInputElement | null>;
  musicInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
  setIsCloudDriveOpen: (val: boolean) => void;
  setIsLinkModalOpen: (val: boolean) => void;
  setLinkUrl: (url: string) => void;
  renameItem: any;
  setRenameItem: (item: any) => void;
  onUpdateDocs?: () => void;
  onMoveItem?: (item: any) => void;
  onCopyItem?: (item: any) => void;
  isLocateHighlight?: boolean;
  t: (key: string, options?: any) => string;
}

export const KnowledgeFileItem = React.memo(function KnowledgeFileItem({
  item, 
  activeDoc,
  selectedDocIds,
  onToggleDocSelection,
  setCurrentFolderId,
  onSelectDoc,
  onMenuCreate,
  fileInputRef,
  audioInputRef,
  musicInputRef,
  folderInputRef,
  setIsCloudDriveOpen,
  setIsLinkModalOpen,
  setLinkUrl,
  renameItem,
  setRenameItem,
  onUpdateDocs,
  onMoveItem,
  onCopyItem,
  isLocateHighlight = false,
  t
}: KnowledgeFileItemProps) {
  const featureFlags = createRuntimeConfig().featureFlags;
  const menuFeatureProps = {
    showDocumentPermissions: featureFlags.documentPermissionsModal,
    showDocumentVersionHistory: featureFlags.documentVersionHistory,
  };
  const isFolder = item.type === 'folder';
  const isActive = activeDoc?.id === item.id;
  const isSelected = selectedDocIds.has(item.id);
  const isRenaming = renameItem?.id === item.id;
  
  const [renameValue, setRenameValue] = React.useState(item.title);
  const inputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    if (isRenaming) {
      setRenameValue(item.title);
      setTimeout(() => {
        if (inputRef.current) {
          inputRef.current.focus();
          inputRef.current.select();
        }
      }, 50);
    }
  }, [isRenaming, item.title]);

  const handleRenameSubmit = () => {
    if (isRenaming) {
      if (renameValue.trim() && renameValue !== item.title) {
        DocumentService.updateDocument(item.id, { title: renameValue }).then(() => {
          if (onUpdateDocs) onUpdateDocs();
        });
      }
      setRenameItem(null);
    }
  };
  
  const handleRenameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.stopPropagation();
      handleRenameSubmit();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      setRenameItem(null);
    }
  };

  const openWebLinkImport = () => {
    setIsLinkModalOpen(true);
    setLinkUrl('');
  };
  
  const renderThumbnail = (type: string) => {
    const title = (item.title || '').toLowerCase();
    const isWord = title.endsWith('.doc') || title.endsWith('.docx') || title.endsWith('.wps');
    const isExcel = title.endsWith('.xls') || title.endsWith('.xlsx') || title.endsWith('.csv');
    const isPPT = title.endsWith('.ppt') || title.endsWith('.pptx');
    const isZip = title.endsWith('.zip') || title.endsWith('.rar') || title.endsWith('.7z') || title.endsWith('.tar');
    const isTxt = title.endsWith('.txt');

    if (type === 'folder') {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-[#10b981]/10 text-[#10b981] dark:bg-[#10b981]/15 dark:text-[#a7f3d0] flex items-center justify-center mr-2.5 shrink-0 shadow-sm">
          <Folder size={18} fill="currentColor" strokeWidth={1} />
        </div>
      );
    }
    if (type === 'pdf') {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-rose-500/10 text-rose-600 dark:bg-rose-500/15 dark:text-rose-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[10px] tracking-tighter">
          PDF
        </div>
      );
    }
    if (isWord) {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-blue-600/10 text-blue-600 dark:bg-blue-600/15 dark:text-blue-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[9px] tracking-tighter">
          WORD
        </div>
      );
    }
    if (isExcel) {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-emerald-600/10 text-emerald-600 dark:bg-emerald-600/15 dark:text-emerald-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[9px] tracking-tighter">
          EXCEL
        </div>
      );
    }
    if (isPPT) {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-orange-600/10 text-orange-600 dark:bg-orange-600/15 dark:text-orange-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[9px] tracking-tighter">
          PPTX
        </div>
      );
    }
    if (isZip) {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-purple-600/10 text-purple-600 dark:bg-purple-600/15 dark:text-purple-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[9px] tracking-tighter">
          ZIP
        </div>
      );
    }
    if (isTxt) {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-zinc-500/10 text-zinc-650 dark:bg-zinc-500/15 dark:text-zinc-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[9px] tracking-tighter">
          TXT
        </div>
      );
    }
    if (type === 'code') {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-orange-500/15 text-orange-600 dark:bg-orange-500/20 dark:text-orange-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[10px] tracking-tighter">
          CODE
        </div>
      );
    }
    if (type === 'image') {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-blue-500/10 text-blue-600 dark:bg-blue-500/15 dark:text-blue-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm">
          <ImageIcon size={18} strokeWidth={1.5} />
        </div>
      );
    }
    if (type === 'audio') {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-amber-500/10 text-amber-600 dark:bg-amber-500/15 dark:text-amber-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"></path><path d="M19 10v2a7 7 0 0 1-14 0v-2"></path><line x1="12" x2="12" y1="19" y2="22"></line></svg>
        </div>
      );
    }
    if (type === 'music') {
      return (
        <div className="w-[38px] h-[38px] rounded-xl bg-rose-500/10 text-rose-500 flex items-center justify-center mr-2.5 shrink-0 shadow-sm">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 18V5l12-2v13"></path><circle cx="6" cy="18" r="3"></circle><circle cx="18" cy="16" r="3"></circle></svg>
        </div>
      );
    }
    return (
      <div className="w-[38px] h-[38px] rounded-xl bg-emerald-500/10 text-emerald-600 dark:bg-emerald-500/15 dark:text-emerald-400 flex items-center justify-center mr-2.5 shrink-0 shadow-sm font-extrabold text-[9px] tracking-tighter">
        DOC
      </div>
    );
  };

  const getSubtext = () => {
    const formattedDate = new Date(item.updatedAt || Date.now()).toLocaleDateString('zh-CN', {
      year: '2-digit',
      month: 'numeric',
      day: 'numeric'
    });
    
    if (item.type === 'folder') {
      const itemsCount = item.children?.length ?? 0;
      return `${itemsCount}项 | ${formattedDate}创建`;
    }

    const title = (item.title || '').toLowerCase();
    const isWord = title.endsWith('.doc') || title.endsWith('.docx') || title.endsWith('.wps');
    const isExcel = title.endsWith('.xls') || title.endsWith('.xlsx') || title.endsWith('.csv');
    const isPPT = title.endsWith('.ppt') || title.endsWith('.pptx');
    const isZip = title.endsWith('.zip') || title.endsWith('.rar') || title.endsWith('.7z') || title.endsWith('.tar');
    const isTxt = title.endsWith('.txt');

    if (isWord) {
      return (
        <span className="flex items-center gap-1">
          <span className="text-blue-500 font-bold shrink-0">📄 WORD</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }
    if (isExcel) {
      return (
        <span className="flex items-center gap-1">
          <span className="text-emerald-500 font-bold shrink-0">📊 EXCEL</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }
    if (isPPT) {
      return (
        <span className="flex items-center gap-1">
          <span className="text-orange-500 font-bold shrink-0">🎯 PPTX</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }
    if (isZip) {
      return (
        <span className="flex items-center gap-1">
          <span className="text-purple-500 font-bold shrink-0">📦 ZIP</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }
    if (isTxt) {
      return (
        <span className="flex items-center gap-1">
          <span className="text-zinc-500 dark:text-zinc-400 font-bold shrink-0">📝 TXT</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }
    
    if (item.type === 'pdf') {
      return (
        <span className="flex items-center gap-1">
          <span className="text-rose-500 font-bold shrink-0">🔺 PDF</span>
          <span className="text-zinc-500 dark:text-zinc-400 font-normal">| {formattedDate}</span>
        </span>
      );
    }
    
    if (item.type === 'code') {
      return (
        <span className="flex items-center gap-1">
          <span className="text-orange-500 font-bold shrink-0">{t('codeTag', { ns: 'kb' })}</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }

    if (item.type === 'richtext' || item.type === 'markdown' || !item.type) {
      return (
        <span className="flex items-center gap-1">
          <span className="text-emerald-500 font-bold shrink-0">{t('noteTag', { ns: 'kb' })}</span>
          <span className="text-zinc-400 dark:text-zinc-500 font-normal">| {formattedDate}</span>
        </span>
      );
    }

    return `${formattedDate}`;
  };

  return (
    <div className="box-border w-full min-w-0 px-0" id={!isFolder ? `kb-file-item-${item.id}` : undefined}>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div 
            className={`flex items-center py-2.5 px-4 w-full min-w-0 relative cursor-pointer group/node transition-all duration-200 border-b border-[var(--color-kb-panel-border)]/50 ${
              isLocateHighlight
                ? 'kb-file-item--highlight bg-[var(--color-kb-panel-active)] text-[var(--color-kb-text-heading)]'
                : isActive && !isSelected
                ? 'bg-black/5 dark:bg-white/10 text-[var(--color-kb-text-heading)] font-medium'
                : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] font-medium'
            } ${
              isSelected 
                ? 'bg-black/5 dark:bg-white/10 text-[var(--color-kb-text-heading)]' 
                : ''
            }`}
            onClick={(e) => {
              if (selectedDocIds.size > 0) {
                onToggleDocSelection(e, item.id);
              } else if (isFolder) {
                setCurrentFolderId(item.id);
              } else {
                onSelectDoc(item as DocumentMeta);
              }
            }}
          >
            {/* 1. Category thumbnail graphic on the left */}
            {renderThumbnail(item.type)}
            
            {/* 2. Text layout vertically aligned */}
            <div className="flex-1 min-w-0 pr-10">
              {isRenaming ? (
                 <input 
                   ref={inputRef}
                   type="text"
                   autoFocus
                   value={renameValue}
                   onChange={(e) => setRenameValue(e.target.value)}
                   onBlur={handleRenameSubmit}
                   onKeyDown={handleRenameKeyDown}
                   onClick={(e) => e.stopPropagation()}
                   className="w-full bg-[var(--color-kb-editor)] border-[1.5px] border-[var(--color-kb-accent)] px-2 py-1 rounded-md text-[13px] font-medium outline-none text-[var(--color-kb-text-heading)] shadow-sm z-20 transition-all focus:ring-2 focus:ring-[var(--color-kb-accent)]/20"
                 />
              ) : (
                 <div className="flex flex-col">
                   <div className="flex items-center gap-1.5 w-full min-w-0">
                     <span className="text-[13px] font-bold text-[var(--color-kb-text-heading)] truncate">{item.title}</span>
                     {item.isPinned && <Pin size={12} className="text-[var(--color-kb-accent)] shrink-0" fill="currentColor" />}
                   </div>
                   <span className="text-[11px] text-[var(--color-kb-text-muted)] mt-1 flex items-center gap-1.5 font-medium leading-none">{getSubtext()}</span>
                 </div>
              )}
            </div>
            
            {/* 3. Action details (Menu & Checkbox) strictly aligned on the right */}
            {!isRenaming && (
              <div className="absolute right-4 flex items-center gap-2">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <button className="hidden group-hover/node:flex data-[state=open]:flex items-center justify-center p-1 rounded-lg hover:bg-[var(--color-kb-panel-border)]/80 text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] transition-all shrink-0" onClick={(e) => e.stopPropagation()}>
                      <MoreHorizontal size={14} />
                    </button>
                  </DropdownMenuTrigger>
 
                  <DropdownMenuContent align="end" side="bottom" className="w-48 z-[150] rounded-xl shadow-lg border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)]">
                    <NodeDropdownItems 
                      {...menuFeatureProps}
                      isFolder={isFolder}
                      onNewDoc={(e) => { e.stopPropagation(); onMenuCreate('richtext', item.id); }}
                      onNewFolder={(e) => { e.stopPropagation(); onMenuCreate('folder', item.id); }}
                      onLocalFile={(e) => { e.preventDefault(); setCurrentFolderId(item.id); fileInputRef.current?.click(); }}
                      onLocalAudio={(e) => { e.preventDefault(); setCurrentFolderId(item.id); audioInputRef.current?.click(); }}
                      onLocalMusic={(e) => { e.preventDefault(); setCurrentFolderId(item.id); musicInputRef.current?.click(); }}
                      onLocalFolder={(e) => { e.preventDefault(); setCurrentFolderId(item.id); folderInputRef.current?.click(); }}
                      onCloudDrive={(e) => { 
                        if (e && e.preventDefault) e.preventDefault();
                        setTimeout(() => {
                          setCurrentFolderId(item.id); 
                          setIsCloudDriveOpen(true);
                        }, 150);
                      }}
                      onWebLink={(e) => { 
                        if (e && e.preventDefault) e.preventDefault();
                        setTimeout(() => {
                          setCurrentFolderId(item.id);
                          openWebLinkImport();
                        }, 150);
                      }}
                      onRename={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); setRenameItem(item); }}
                      onDelete={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); DocumentService.deleteDocument(item.id).then(() => onUpdateDocs && onUpdateDocs()); }}
                      onMoveTo={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); onMoveItem?.(item); }}
                      onCopyTo={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); onCopyItem?.(item); }}
                      onPin={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'pin', item } })); }}
                      onEditTags={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'tags', item } })); }}
                      onPermissions={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'permissions', item } })); }}
                      onOpenNewTab={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.open(location.href, '_blank'); }}
                      onViewHistory={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'history', item } })); }}
                      t={t}
                    />
                  </DropdownMenuContent>
                </DropdownMenu>

                {/* Nice right Checkbox shown on hover / check */}
                <div 
                  className={`transition-all duration-200 shrink-0 ${isSelected ? 'flex' : 'hidden group-hover/node:flex'}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    onToggleDocSelection(e, item.id);
                  }}
                >
                  {isSelected ? (
                    <div className="w-4.5 h-4.5 rounded border border-emerald-500 bg-emerald-500 flex items-center justify-center text-white shadow-sm scale-95 active:scale-90 transition-transform">
                      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M2 5L4 7L8 2" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"/>
                      </svg>
                    </div>
                  ) : (
                    <div className="w-4.5 h-4.5 rounded border border-[var(--color-kb-panel-border)] hover:border-emerald-500 bg-transparent flex items-center justify-center transition-all scale-95 hover:scale-100" />
                  )}
                </div>
              </div>
            )}
          </div>
        </ContextMenuTrigger>
 
        <ContextMenuContent className="w-48 z-[200] rounded-xl shadow-lg border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)]">
             <NodeContextItems 
               {...menuFeatureProps}
               isFolder={isFolder}
               onNewDoc={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); onMenuCreate('richtext', item.id); }}
               onNewFolder={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); onMenuCreate('folder', item.id); }}
               onLocalFile={(e) => { e.preventDefault(); setCurrentFolderId(item.id); fileInputRef.current?.click(); }}
               onLocalAudio={(e) => { e.preventDefault(); setCurrentFolderId(item.id); audioInputRef.current?.click(); }}
               onLocalMusic={(e) => { e.preventDefault(); setCurrentFolderId(item.id); musicInputRef.current?.click(); }}
               onLocalFolder={(e) => { e.preventDefault(); setCurrentFolderId(item.id); folderInputRef.current?.click(); }}
               onCloudDrive={(e) => { 
                 if (e && e.preventDefault) e.preventDefault();
                 setTimeout(() => {
                   setCurrentFolderId(item.id); 
                   setIsCloudDriveOpen(true);
                 }, 150);
               }}
               onWebLink={(e) => { 
                 if (e && e.preventDefault) e.preventDefault();
                 setTimeout(() => {
                   setCurrentFolderId(item.id);
                   openWebLinkImport();
                 }, 150);
               }}
               onRename={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); setRenameItem(item); }}
               onDelete={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); DocumentService.deleteDocument(item.id).then(() => onUpdateDocs && onUpdateDocs()); }}
               onMoveTo={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); onMoveItem?.(item); }}
               onCopyTo={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); onCopyItem?.(item); }}
               onPin={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'pin', item } })); }}
               onEditTags={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'tags', item } })); }}
               onPermissions={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'permissions', item } })); }}
               onOpenNewTab={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.open(location.href, '_blank'); }}
               onViewHistory={(e) => { if (e && e.preventDefault) e.preventDefault(); e.stopPropagation(); window.dispatchEvent(new CustomEvent('kb-action', { detail: { action: 'history', item } })); }}
               t={t}
             />
        </ContextMenuContent>
      </ContextMenu>
    </div>
  );
});

