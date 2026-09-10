import { MoreHorizontal, Users, User, Globe, BookOpen } from 'lucide-react';
import { KnowledgeBase } from './services/document';
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent } from './components/ui/dropdown-menu';
import { ContextMenu, ContextMenuTrigger, ContextMenuContent } from './components/ui/context-menu';
import { KbDropdownItems, KbContextItems } from './KbMenuContent';
import { TFunction } from 'i18next';
import type { ReactKeyedComponentProps } from '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps';

export interface KnowledgeBaseItemProps extends ReactKeyedComponentProps {
  kb: KnowledgeBase;
  activeKb: KnowledgeBase | null;
  onSelectKb: (kb: KnowledgeBase) => void;
  onRename: (kb: KnowledgeBase) => void;
  onDelete: (kb: KnowledgeBase) => void;
  onOpenSettings: (kb: KnowledgeBase) => void;
  onImportGit: (kb: KnowledgeBase) => void;
  onSyncGit: (kb: KnowledgeBase) => void;
  onImportCloudDrive: (kb: KnowledgeBase) => void;
  t: TFunction<"kb" | "common", undefined>;
}

export function KnowledgeBaseItem({
  kb,
  activeKb,
  onSelectKb,
  onRename,
  onDelete,
  onOpenSettings,
  onImportGit,
  onSyncGit,
  onImportCloudDrive,
  t
}: KnowledgeBaseItemProps) {
  const isActive = activeKb?.id === kb.id;

  // Custom visual icon helper
  const renderVisualIcon = () => {
    if (kb.avatar) {
      return (
        <img 
          src={kb.avatar} 
          alt={kb.title} 
          className="w-[22px] h-[22px] rounded-lg object-cover mr-2.5 flex-shrink-0 shadow-[0_1px_3px_rgba(0,0,0,0.1)] border border-[var(--color-kb-panel-border)]" 
        />
      );
    }

    let GradientClass = '';
    let IconComponent = BookOpen;

    if (kb.type === 'team') {
      GradientClass = isActive
        ? 'bg-gradient-to-br from-blue-500 to-indigo-500 text-white border-white/25'
        : 'bg-blue-50 text-blue-600 border-blue-200/70 dark:bg-blue-950/40 dark:text-blue-400 dark:border-blue-500/25';
      IconComponent = Users;
    } else if (kb.type === 'personal') {
      GradientClass = isActive
        ? 'bg-gradient-to-br from-sky-500 to-blue-600 text-white border-white/25'
        : 'bg-sky-50 text-sky-600 border-sky-200/70 dark:bg-sky-950/40 dark:text-sky-400 dark:border-sky-500/25';
      IconComponent = User;
    } else {
      GradientClass = isActive
        ? 'bg-gradient-to-br from-cyan-500 to-teal-500 text-white border-white/25'
        : 'bg-cyan-50 text-cyan-700 border-cyan-200/70 dark:bg-cyan-950/40 dark:text-cyan-400 dark:border-cyan-500/25';
      IconComponent = Globe;
    }

    const hasCustomIcon = kb.icon && kb.icon !== '📁' && kb.icon !== '📘' && kb.icon !== '📂' && kb.icon !== '🌍' && kb.icon !== '📖';

    return (
      <div className={`w-[22px] h-[22px] rounded-lg flex items-center justify-center mr-2.5 flex-shrink-0 transition-all duration-300 shadow-[0_1px_3px_rgba(0,0,0,0.1)] border ${GradientClass} group-hover/node:scale-110 group-hover/node:-rotate-3`}>
        {hasCustomIcon ? (
          <span className="text-[11px] leading-none">{kb.icon}</span>
        ) : (
          <IconComponent size={11} strokeWidth={2.5} />
        )}
      </div>
    );
  };

  return (
    <ContextMenu>
      <DropdownMenu>
        <ContextMenuTrigger asChild>
          <div
            onClick={() => onSelectKb(kb)}
            className={`flex items-center px-2 h-[32px] mx-0 mb-[2px] w-full min-w-0 overflow-hidden rounded-[8px] cursor-pointer text-[13.5px] group/node relative transition-all duration-150 ${isActive ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-text-heading)] font-semibold' : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'}`}
          >
            <span
              aria-hidden="true"
              className={`absolute left-0 top-1/2 -translate-y-1/2 h-[16px] w-[3px] rounded-r-full bg-[var(--color-kb-accent)] transition-opacity ${isActive ? 'opacity-100' : 'opacity-0'}`}
            />
            {renderVisualIcon()}
            <span className={`truncate flex-1 min-w-0 transition-colors tracking-wide pr-6 ${!isActive && 'group-hover/node:text-[var(--color-kb-text-heading)]'}`}>{kb.title}</span>
            
            <DropdownMenuTrigger asChild>
              <button className={`hidden group-hover/node:flex data-[state=open]:flex absolute right-1 items-center justify-center p-1 rounded-md hover:bg-[var(--color-kb-panel-border)] text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-all ${isActive ? '!text-[var(--color-kb-accent)] hover:!bg-[var(--color-kb-accent)]/20' : ''}`} onClick={(e) => e.stopPropagation()}>
                <MoreHorizontal size={15} />
              </button>
            </DropdownMenuTrigger>
          </div>
        </ContextMenuTrigger>
        <DropdownMenuContent align="end" side="bottom" className="w-40">
          <KbDropdownItems 
            onRename={(e) => { e.stopPropagation(); onRename(kb); }}
            onDelete={(e) => { e.stopPropagation(); onDelete(kb); }}
            onOpenSettings={(e) => { e.stopPropagation(); onOpenSettings(kb); }}
            onImportGit={(e) => { e.stopPropagation(); onImportGit(kb); }}
            onSyncGit={(e) => { e.stopPropagation(); onSyncGit(kb); }}
            onImportCloudDrive={(e) => { e.stopPropagation(); onImportCloudDrive(kb); }}
            t={t}
          />
        </DropdownMenuContent>
      </DropdownMenu>

      <ContextMenuContent className="w-40 z-[200]">
        <KbContextItems 
          onRename={(e) => { e.stopPropagation(); onRename(kb); }}
          onDelete={(e) => { e.stopPropagation(); onDelete(kb); }}
          onOpenSettings={(e) => { e.stopPropagation(); onOpenSettings(kb); }}
          onImportGit={(e) => { e.stopPropagation(); onImportGit(kb); }}
          onSyncGit={(e) => { e.stopPropagation(); onSyncGit(kb); }}
          onImportCloudDrive={(e) => { e.stopPropagation(); onImportCloudDrive(kb); }}
          t={t}
        />
      </ContextMenuContent>
    </ContextMenu>
  );
}
