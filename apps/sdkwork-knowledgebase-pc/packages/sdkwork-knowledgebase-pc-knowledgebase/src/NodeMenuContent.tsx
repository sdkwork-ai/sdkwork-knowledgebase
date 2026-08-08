import React from 'react';
import { FileText, FolderPlus, FileUp, FolderUp, Cloud, Link, Edit2, Trash2, Pin, Tag, UserCog, Copy, ExternalLink, History, FolderOutput, Mic, Music } from 'lucide-react';
import { DropdownMenuItem, DropdownMenuSeparator } from './components/ui/dropdown-menu';
import { ContextMenuItem, ContextMenuSeparator } from './components/ui/context-menu';

export interface NodeMenuItemsProps {
  isFolder: boolean;
  onNewDoc: (e: React.MouseEvent) => void;
  onNewFolder: (e: React.MouseEvent) => void;
  onLocalFile: (e: Event) => void;
  onLocalAudio: (e: Event) => void;
  onLocalMusic: (e: Event) => void;
  onLocalFolder: (e: Event) => void;
  onCloudDrive: (e: Event) => void;
  onWebLink: (e: React.MouseEvent) => void;
  onRename: (e: React.MouseEvent) => void;
  onDelete: (e: React.MouseEvent) => void;
  onMoveTo?: (e: React.MouseEvent) => void;
  onCopyTo?: (e: React.MouseEvent) => void;
  onPin?: (e: React.MouseEvent) => void;
  onEditTags?: (e: React.MouseEvent) => void;
  onPermissions?: (e: React.MouseEvent) => void;
  onOpenNewTab?: (e: React.MouseEvent) => void;
  onViewHistory?: (e: React.MouseEvent) => void;
  showDocumentPermissions?: boolean;
  showDocumentVersionHistory?: boolean;
  t: (key: string, options?: any) => string;
}

export function NodeDropdownItems({ 
  isFolder, onNewDoc, onNewFolder, onLocalFile, onLocalAudio, onLocalMusic, onLocalFolder, onCloudDrive, onWebLink, onRename, onDelete, onMoveTo, onCopyTo, 
  onPin, onEditTags, onPermissions, onOpenNewTab, onViewHistory,
  showDocumentPermissions = false,
  showDocumentVersionHistory = false,
  t 
}: NodeMenuItemsProps) {
  return (
    <>
      {isFolder && (
        <>
          <DropdownMenuItem onSelect={onNewDoc as any}>
            <FileText size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('newDoc')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onNewFolder as any}>
            <FolderPlus size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('newFolder')}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onLocalFile}>
            <FileUp size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('localFile')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onLocalAudio}>
            <Mic size={14} className="mr-2 text-amber-500" />
            {t('audioRecord', { defaultValue: '上传录音文件' })}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onLocalMusic}>
            <Music size={14} className="mr-2 text-rose-500" />
            {t('musicRecord', { defaultValue: '上传音乐文件' })}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onLocalFolder}>
            <FolderUp size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('localFolder')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onCloudDrive}>
            <Cloud size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('cloudDrive')}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onWebLink as any}>
            <Link size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('webLink')}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onRename as any}>
            <Edit2 size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('rename', { ns: 'common' })}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onMoveTo as any}>
            <FolderOutput size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            移动到...
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onCopyTo as any}>
            <Copy size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            复制到...
          </DropdownMenuItem>
        </>
      )}
      {!isFolder && (
        <>
          <DropdownMenuItem onSelect={onPin as any}>
            <Pin size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            置顶
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onEditTags as any}>
            <Tag size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            编辑标签
          </DropdownMenuItem>
          {showDocumentPermissions && (
            <DropdownMenuItem onSelect={onPermissions as any}>
              <UserCog size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
              成员权限
            </DropdownMenuItem>
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onOpenNewTab as any}>
            <ExternalLink size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            在新标签页中打开
          </DropdownMenuItem>
          {showDocumentVersionHistory && (
            <DropdownMenuItem onSelect={onViewHistory as any}>
              <History size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
              查看历史版本
            </DropdownMenuItem>
          )}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onMoveTo as any}>
            <FolderOutput size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            移动到...
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onCopyTo as any}>
            <Copy size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            复制到...
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={onRename as any}>
            <Edit2 size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('rename', { ns: 'common' })}
          </DropdownMenuItem>
        </>
      )}
      <DropdownMenuSeparator />
      <DropdownMenuItem onSelect={onDelete as any} className="text-red-500 focus:text-red-500">
        <Trash2 size={14} className="mr-2" />
        {t('delete', { ns: 'common' })}
      </DropdownMenuItem>
    </>
  );
}

export function NodeContextItems({ 
  isFolder, onNewDoc, onNewFolder, onLocalFile, onLocalAudio, onLocalMusic, onLocalFolder, onCloudDrive, onWebLink, onRename, onDelete, onMoveTo, onCopyTo, 
  onPin, onEditTags, onPermissions, onOpenNewTab, onViewHistory,
  showDocumentPermissions = false,
  showDocumentVersionHistory = false,
  t 
}: NodeMenuItemsProps) {
  return (
    <>
      {isFolder && (
        <>
          <ContextMenuItem onSelect={onNewDoc as any}>
            <FileText size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('newDoc')}
          </ContextMenuItem>
          <ContextMenuItem onSelect={onNewFolder as any}>
            <FolderPlus size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('newFolder')}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={onLocalFile}>
            <FileUp size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('localFile')}
          </ContextMenuItem>
          <ContextMenuItem onSelect={onLocalAudio}>
            <Mic size={14} className="mr-2 text-amber-500" />
            {t('audioRecord', { defaultValue: '上传录音文件' })}
          </ContextMenuItem>
          <ContextMenuItem onSelect={onLocalMusic}>
            <Music size={14} className="mr-2 text-rose-500" />
            {t('musicRecord', { defaultValue: '上传音乐文件' })}
          </ContextMenuItem>
          <ContextMenuItem onSelect={onLocalFolder}>
            <FolderUp size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('localFolder')}
          </ContextMenuItem>
          <ContextMenuItem onSelect={onCloudDrive}>
            <Cloud size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('cloudDrive')}
          </ContextMenuItem>
          <ContextMenuItem onSelect={onWebLink as any}>
            <Link size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('webLink')}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={onRename as any}>
            <Edit2 size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('rename', { ns: 'common' })}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={onMoveTo as any}>
            <FolderOutput size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            移动到...
          </ContextMenuItem>
          <ContextMenuItem onSelect={onCopyTo as any}>
            <Copy size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            复制到...
          </ContextMenuItem>
        </>
      )}
      {!isFolder && (
        <>
          <ContextMenuItem onSelect={onPin as any}>
            <Pin size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            置顶
          </ContextMenuItem>
          <ContextMenuItem onSelect={onEditTags as any}>
            <Tag size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            编辑标签
          </ContextMenuItem>
          {showDocumentPermissions && (
            <ContextMenuItem onSelect={onPermissions as any}>
              <UserCog size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
              成员权限
            </ContextMenuItem>
          )}
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={onOpenNewTab as any}>
            <ExternalLink size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            在新标签页中打开
          </ContextMenuItem>
          {showDocumentVersionHistory && (
            <ContextMenuItem onSelect={onViewHistory as any}>
              <History size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
              查看历史版本
            </ContextMenuItem>
          )}
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={onMoveTo as any}>
            <FolderOutput size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            移动到...
          </ContextMenuItem>
          <ContextMenuItem onSelect={onCopyTo as any}>
            <Copy size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            复制到...
          </ContextMenuItem>
          <ContextMenuItem onSelect={onRename as any}>
            <Edit2 size={14} className="mr-2 text-[var(--color-kb-text-muted)]" />
            {t('rename', { ns: 'common' })}
          </ContextMenuItem>
        </>
      )}
      <ContextMenuSeparator />
      <ContextMenuItem onSelect={onDelete as any} className="text-red-500 focus:text-red-500">
        <Trash2 size={14} className="mr-2" />
        {t('delete', { ns: 'common' })}
      </ContextMenuItem>
    </>
  );
}


