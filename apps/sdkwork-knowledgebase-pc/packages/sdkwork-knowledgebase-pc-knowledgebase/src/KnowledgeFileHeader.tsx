import React, { } from 'react';
import { Search, X } from 'lucide-react';
import { KnowledgeBase } from './services/document';
import { useTranslation } from 'react-i18next';
import { KnowledgeFileAddMenu } from './KnowledgeFileAddMenu';

export interface KnowledgeFileHeaderProps {
  activeKb: KnowledgeBase | null;
  isSearchOpen: boolean;
  setIsSearchOpen: (val: boolean) => void;
  searchQuery: string;
  setSearchQuery: (val: string) => void;
  isAddMenuOpen: boolean;
  setIsAddMenuOpen: (val: boolean) => void;
  isMultiSelectMode: boolean;
  setIsMultiSelectMode: (val: boolean) => void;
  addMenuRef: React.RefObject<HTMLDivElement | null>;
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  audioInputRef: React.RefObject<HTMLInputElement | null>;
  musicInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
  currentFolderId: string | null;
  onMenuCreate: (actionType: string, parentId?: string, payload?: any) => Promise<any> | void;
  setIsLinkModalOpen: (val: boolean) => void;
  setLinkUrl: (val: string) => void;
  setIsCloudDriveOpen: (val: boolean) => void;
  setIsNotesAppOpen: (val: boolean) => void;
  setIsPersonalKbOpen: (val: boolean) => void;
  setIsChatFileOpen: (val: boolean) => void;
  setIsChatDialogOpen: (val: boolean) => void;
  setRenameItem: (item: any) => void;
}

export function KnowledgeFileHeader({
  activeKb,
  isSearchOpen, setIsSearchOpen,
  searchQuery, setSearchQuery,
  isAddMenuOpen, setIsAddMenuOpen,
  addMenuRef, fileInputRef, audioInputRef, musicInputRef, folderInputRef,
  currentFolderId,
  onMenuCreate,
  setIsLinkModalOpen, setLinkUrl,
  setIsCloudDriveOpen, setIsNotesAppOpen, setIsPersonalKbOpen, setIsChatFileOpen, setIsChatDialogOpen,
  setRenameItem
}: KnowledgeFileHeaderProps) {
  const { t } = useTranslation(['kb', 'common']);

  return (
    <div className="h-[40px] px-5 flex items-center justify-between min-w-0 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/50 sticky top-0 z-10 bg-[var(--color-kb-panel)]/90 backdrop-blur-md shadow-sm">
      {isSearchOpen ? (
        <div className="flex-1 flex items-center h-[28px] bg-white dark:bg-[var(--color-kb-input-bg)] rounded-lg text-zinc-950 dark:text-[var(--color-kb-text)] px-2.5 border border-zinc-200/90 dark:border-[var(--color-kb-panel-border)]/80 hover:border-zinc-350 dark:hover:border-zinc-700 focus-within:border-[var(--color-kb-accent)] focus-within:ring-2 focus-within:ring-[var(--color-kb-accent)]/15 transition-all shadow-sm">
          <Search size={13} strokeWidth={2.5} className="text-zinc-400 dark:text-[var(--color-kb-text-muted)] mr-1.5 flex-shrink-0" />
          <input 
            type="text" 
            placeholder={t('searchPlaceholder')} 
            autoFocus
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 bg-transparent border-none text-[12.5px] font-semibold text-zinc-800 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500 focus:outline-none focus:ring-0 w-full min-w-0"
          />
          <button 
            onClick={() => { setIsSearchOpen(false); setSearchQuery(''); }}
            className="text-zinc-400 hover:text-red-500 dark:text-[var(--color-kb-text-muted)] dark:hover:text-red-400 p-0.5 rounded-md ml-1.5 transition-all active:scale-95 flex-shrink-0"
            title={t('cancelSearch')}
          >
            <X size={14} strokeWidth={2.5} />
          </button>
        </div>
      ) : (
        <>
          <h3 className="font-extrabold text-[15px] tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)] min-w-0 flex-1 truncate pr-2">{activeKb ? activeKb.title : t('pleaseSelectKb')}</h3>
          <div className="flex items-center gap-1.5 text-zinc-500 dark:text-[var(--color-kb-text-muted)] flex-shrink-0">
            <input 
              type="file" 
              ref={musicInputRef} 
              className="hidden" 
              multiple 
              accept="audio/*,video/*"
              onChange={async (e) => {
                if (e.target.files?.length) {
                  const result = await onMenuCreate('musicUpload', currentFolderId ?? undefined, Array.from(e.target.files));
                  if (result) setRenameItem(result);
                  if (musicInputRef.current) musicInputRef.current.value = '';
                  setIsAddMenuOpen(false);
                }
              }}
            />
            <input 
              type="file" 
              ref={audioInputRef} 
              className="hidden" 
              multiple 
              accept="audio/*,video/*"
              onChange={async (e) => {
                if (e.target.files?.length) {
                  const result = await onMenuCreate('audioUpload', currentFolderId ?? undefined, Array.from(e.target.files));
                  if (result) setRenameItem(result);
                  if (audioInputRef.current) audioInputRef.current.value = '';
                  setIsAddMenuOpen(false);
                }
              }}
            />
            <input 
              type="file" 
              ref={fileInputRef} 
              className="hidden" 
              multiple 
              onChange={async (e) => {
                if (e.target.files?.length) {
                  const result = await onMenuCreate('localFile', currentFolderId ?? undefined, Array.from(e.target.files));
                  if (result) setRenameItem(result);
                  if (fileInputRef.current) fileInputRef.current.value = '';
                  setIsAddMenuOpen(false);
                }
              }}
            />
            <button 
              onClick={() => setIsSearchOpen(true)} 
              className="p-1.5 rounded-md transition-all hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-accent)]" 
              title={t('search', { ns: 'common' })}
            >
              <Search size={15} />
            </button>
            <div className="relative" ref={addMenuRef}>
              <input 
                type="file" 
                ref={folderInputRef} 
                className="hidden" 
                {...({ webkitdirectory: '', directory: '' } as React.InputHTMLAttributes<HTMLInputElement>)}
                onChange={async (e: any) => {
                  if (e.target.files?.length) {
                    const result = await onMenuCreate('localFolder', currentFolderId ?? undefined, Array.from(e.target.files));
                    if (result) setRenameItem(result);
                    if (folderInputRef.current) folderInputRef.current.value = '';
                    setIsAddMenuOpen(false);
                  }
                }}
              />
              <KnowledgeFileAddMenu
                isAddMenuOpen={isAddMenuOpen}
                setIsAddMenuOpen={setIsAddMenuOpen}
                fileInputRef={fileInputRef}
                audioInputRef={audioInputRef}
                musicInputRef={musicInputRef}
                folderInputRef={folderInputRef}
                currentFolderId={currentFolderId}
                onMenuCreate={onMenuCreate}
                setIsLinkModalOpen={setIsLinkModalOpen}
                setLinkUrl={setLinkUrl}
                setIsCloudDriveOpen={setIsCloudDriveOpen}
                setIsNotesAppOpen={setIsNotesAppOpen}
                setIsPersonalKbOpen={setIsPersonalKbOpen}
                setIsChatFileOpen={setIsChatFileOpen}
                setIsChatDialogOpen={setIsChatDialogOpen}
                setRenameItem={setRenameItem}
              />
            </div>
          </div>
        </>
      )}
    </div>
  );
}
