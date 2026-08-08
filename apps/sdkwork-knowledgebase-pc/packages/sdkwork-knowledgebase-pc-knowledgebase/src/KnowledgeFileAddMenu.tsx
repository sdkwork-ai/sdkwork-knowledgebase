import React from 'react';
import { FileUp, FolderUp, MessageSquare, Lightbulb, Link, FileEdit, Cloud, Notebook, Mic, FolderPlus, Plus, MessageCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { createRuntimeConfig, KnowledgebaseErrorCodes, throwKnowledgebaseError } from 'sdkwork-knowledgebase-pc-core';
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuGroup, DropdownMenuLabel } from './components/ui/dropdown-menu';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';

export interface KnowledgeFileAddMenuProps {
  isAddMenuOpen: boolean;
  setIsAddMenuOpen: (open: boolean) => void;
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  audioInputRef: React.RefObject<HTMLInputElement | null>;
  musicInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
  currentFolderId: string | null;
  onMenuCreate: (actionType: string, parentId?: string, payload?: any) => Promise<any> | void;
  setIsLinkModalOpen: (open: boolean) => void;
  setLinkUrl: (url: string) => void;
  setIsCloudDriveOpen: (open: boolean) => void;
  setIsNotesAppOpen: (open: boolean) => void;
  setIsPersonalKbOpen: (open: boolean) => void;
  setIsChatFileOpen: (open: boolean) => void;
  setIsChatDialogOpen: (open: boolean) => void;
  setRenameItem: (item: any) => void;
}

export function KnowledgeFileAddMenu({
  isAddMenuOpen, setIsAddMenuOpen,
  fileInputRef, audioInputRef, musicInputRef, folderInputRef,
  currentFolderId,
  onMenuCreate,
  setIsLinkModalOpen, setLinkUrl,
  setIsCloudDriveOpen, setIsNotesAppOpen, setIsPersonalKbOpen, setIsChatFileOpen, setIsChatDialogOpen}: KnowledgeFileAddMenuProps) {
  const { t } = useTranslation(['kb', 'common']);
  const featureFlags = createRuntimeConfig().featureFlags;

  const handleOpenModal = (e: any, setter: (open: boolean) => void) => {
    // Prevent default Radix behavior first to avoid race conditions with Modals
    if (e && e.preventDefault) e.preventDefault();
    setIsAddMenuOpen(false);
    // Add wait time before opening modal so that DropdownMenu can cleanly unmount focus return
    setTimeout(() => {
      setter(true);
    }, 150);
  };

  const handleOpenOfflineImportModal = (
    e: any,
    setter: (open: boolean) => void,
    featureLabel: string,
    enabled: boolean,
  ) => {
    if (e && e.preventDefault) e.preventDefault();
    setIsAddMenuOpen(false);
    setTimeout(() => {
      if (!enabled) {
        try {
          throwKnowledgebaseError(KnowledgebaseErrorCodes.FEATURE_PREVIEW_ONLY, { cause: featureLabel });
        } catch (error) {
          toastKnowledgebaseError(error, t);
        }
        return;
      }
      setter(true);
    }, 150);
  };
  
  const handleCreate = async (e: any, type: string) => {
    if (e && e.preventDefault) e.preventDefault();
    setIsAddMenuOpen(false);
    setTimeout(async () => {
      await onMenuCreate(type, currentFolderId ?? undefined);
    }, 150);
  };

  return (
    <DropdownMenu open={isAddMenuOpen} onOpenChange={setIsAddMenuOpen}>
      <DropdownMenuTrigger asChild>
        <button data-testid="knowledgebase-pc-add-menu-trigger" className={`p-1.5 rounded-md transition-all ${isAddMenuOpen ? 'bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)]' : 'hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-accent)]'}`} title={t('new')}>
          <Plus size={15}/>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" sideOffset={8} className="w-64 p-2 z-[150] shadow-[0_10px_40px_rgba(0,0,0,0.1)] rounded-xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)]">
        
        {/* Create Group */}
        <DropdownMenuGroup>
          <DropdownMenuLabel className="px-3 py-1.5 text-[11px] font-semibold text-[var(--color-kb-text-muted)] tracking-wider uppercase mb-1">Create</DropdownMenuLabel>
          <DropdownMenuItem onSelect={(e) => handleCreate(e, 'note_doc')} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <FileEdit size={16} className="mr-3 text-[var(--color-kb-accent)]" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('note')}</span>
            </div>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={(e) => handleCreate(e, 'folder')} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <FolderPlus size={16} className="mr-3 text-[var(--color-kb-accent)]" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('newFolder')}</span>
            </div>
          </DropdownMenuItem>
        </DropdownMenuGroup>

        <DropdownMenuSeparator className="my-2 bg-[var(--color-kb-panel-border)]/50" />
        
        {/* Local Upload Group */}
        <DropdownMenuGroup>
          <DropdownMenuLabel className="px-3 py-1.5 text-[11px] font-semibold text-[var(--color-kb-text-muted)] tracking-wider uppercase mb-1">Local Upload</DropdownMenuLabel>
          <DropdownMenuItem onSelect={(e) => { e.preventDefault(); setIsAddMenuOpen(false); setTimeout(() => fileInputRef.current?.click(), 150); }} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <FileUp size={16} className="mr-3 text-blue-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('localFile')}</span>
            </div>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={(e) => { e.preventDefault(); setIsAddMenuOpen(false); setTimeout(() => audioInputRef.current?.click(), 150); }} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <Mic size={16} className="mr-3 text-amber-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('audioRecord')}</span>
            </div>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={(e) => { e.preventDefault(); setIsAddMenuOpen(false); setTimeout(() => musicInputRef.current?.click(), 150); }} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="mr-3 text-rose-500"><path d="M9 18V5l12-2v13"></path><circle cx="6" cy="18" r="3"></circle><circle cx="18" cy="16" r="3"></circle></svg>
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('musicRecord', { defaultValue: '上传音乐文件' })}</span>
            </div>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={(e) => { e.preventDefault(); setIsAddMenuOpen(false); setTimeout(() => folderInputRef.current?.click(), 150); }} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <FolderUp size={16} className="mr-3 text-blue-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('localFolder')}</span>
            </div>
          </DropdownMenuItem>
        </DropdownMenuGroup>

        <DropdownMenuSeparator className="my-2 bg-[var(--color-kb-panel-border)]/50" />

        {/* Import From Group */}
        <DropdownMenuGroup>
          <DropdownMenuLabel className="px-3 py-1.5 text-[11px] font-semibold text-[var(--color-kb-text-muted)] tracking-wider uppercase mb-1">Import From</DropdownMenuLabel>
          {featureFlags.chatFileImport ? (
          <DropdownMenuItem onSelect={(e) => handleOpenOfflineImportModal(e, setIsChatFileOpen, '聊天文件导入', featureFlags.chatFileImport)} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <MessageSquare size={16} className="mr-3 text-[#07C160]" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('fromChatFile', { defaultValue: '聊天文件' })}</span>
            </div>
          </DropdownMenuItem>
          ) : null}
          {featureFlags.chatDialogImport ? (
          <DropdownMenuItem onSelect={(e) => handleOpenOfflineImportModal(e, setIsChatDialogOpen, '聊天对话导入', featureFlags.chatDialogImport)} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <MessageCircle size={16} className="mr-3 text-[#07C160]" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('fromChatDialog', { defaultValue: '聊天对话' })}</span>
            </div>
          </DropdownMenuItem>
          ) : null}
          <DropdownMenuItem onSelect={(e) => handleOpenModal(e, setIsPersonalKbOpen)} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <Lightbulb size={16} className="mr-3 text-amber-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('importedPersonalKb')}</span>
            </div>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={(e) => {
            if (e && e.preventDefault) e.preventDefault();
            setIsAddMenuOpen(false);
            setTimeout(() => {
              setLinkUrl('');
              setIsLinkModalOpen(true);
            }, 150);
          }} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <Link size={16} className="mr-3 text-indigo-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('webLink')}</span>
            </div>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={(e) => handleOpenModal(e, setIsCloudDriveOpen)} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <Cloud size={16} className="mr-3 text-cyan-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('cloudDrive')}</span>
            </div>
          </DropdownMenuItem>
          {featureFlags.notesImport ? (
          <DropdownMenuItem onSelect={(e) => handleOpenOfflineImportModal(e, setIsNotesAppOpen, '备忘录导入', featureFlags.notesImport)} className="cursor-pointer py-2 px-3 rounded-lg focus:bg-[var(--color-kb-panel-hover)] focus:text-[var(--color-kb-text-heading)]">
            <div className="flex items-center w-full">
               <Notebook size={16} className="mr-3 text-purple-500" /> 
               <span className="font-medium text-[13px] text-[var(--color-kb-text)]">{t('notesApp')}</span>
            </div>
          </DropdownMenuItem>
          ) : null}
        </DropdownMenuGroup>
        
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
