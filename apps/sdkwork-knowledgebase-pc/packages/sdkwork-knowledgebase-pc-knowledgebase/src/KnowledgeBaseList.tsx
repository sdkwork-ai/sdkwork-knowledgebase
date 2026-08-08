import { useState } from 'react';
import { Plus, Box, ChevronDown, ChevronRight } from 'lucide-react';
import { KnowledgeBase, DocumentService } from './services/document';
import { useTranslation } from 'react-i18next';
import { RenameModal } from './RenameModal';
import { KnowledgeBaseItem } from './KnowledgeBaseItem';

interface KnowledgeBaseListProps {
  kbs: { team: KnowledgeBase[], personal: KnowledgeBase[], public: KnowledgeBase[] };
  loadingKbs: boolean;
  activeKb: KnowledgeBase | null;
  onSelectKb: (kb: KnowledgeBase) => void;
  onCreateKbSelect: (type: 'team' | 'personal' | 'public') => void;
  onOpenSettings: (kb: KnowledgeBase) => void;
  onOpenMarket: () => void;
  onImportGit: (kb: KnowledgeBase) => void;
  onSyncGit: (kb: KnowledgeBase) => void;
  onImportCloudDrive: (kb: KnowledgeBase) => void;
  onUpdateKbs?: () => void;
  width?: number;
  isDragging?: boolean;
  onMouseDownDrag?: () => void;
}

export function KnowledgeBaseList({ kbs, loadingKbs, activeKb, onSelectKb, onCreateKbSelect, onOpenSettings, onOpenMarket, onImportGit, onSyncGit, onImportCloudDrive, onUpdateKbs, width = 240, isDragging, onMouseDownDrag }: KnowledgeBaseListProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [renameItem, setRenameItem] = useState<KnowledgeBase | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({
    subscribed: true,
    team: true,
    personal: true,
    public: true
  });

  const [limitSubscribed, setLimitSubscribed] = useState(5);
  const [limitTeam, setLimitTeam] = useState(5);
  const [limitPersonal, setLimitPersonal] = useState(5);
  const [limitPublic, setLimitPublic] = useState(5);

  const handleDeleteKb = (kb: KnowledgeBase) => {
    DocumentService.deleteKnowledgeBase(kb.id).then(() => onUpdateKbs && onUpdateKbs());
  };

  const publicKbs = kbs.public || [];
  const subscribedKbs = publicKbs.filter(kb => kb.id && kb.id.startsWith('m'));
  const regularPublicKbs = publicKbs.filter(kb => kb.id && !kb.id.startsWith('m'));

  const visibleSubscribed = subscribedKbs.slice(0, limitSubscribed);
  const visibleTeam = kbs.team.slice(0, limitTeam);
  const visiblePersonal = kbs.personal.slice(0, limitPersonal);
  const visiblePublic = regularPublicKbs.slice(0, limitPublic);

  return (
    <div 
      className="flex-shrink-0 flex flex-col bg-[var(--color-kb-panel)] border-r border-[var(--color-kb-panel-border)] relative overflow-hidden"
      style={{ width }}
    >
      <div className="px-5 h-[40px] flex items-center justify-between min-w-0 bg-[var(--color-kb-panel)] z-10 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/50 shadow-sm flex-none">
        <div className="flex items-center min-w-0 pr-2">
          <div className="w-6 h-6 rounded bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] flex items-center justify-center mr-2.5 shadow-sm shrink-0">
            <Box size={14} />
          </div>
          <h2 className="font-bold text-[14px] tracking-wide text-[var(--color-kb-text-heading)] truncate">{t('kbManagement')}</h2>
        </div>
        <button onClick={() => onCreateKbSelect('team')} className="p-1.5 hover:bg-[var(--color-kb-panel-border)] rounded-md text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-all shadow-sm" title={t('newKb')}>
          <Plus size={15} />
        </button>
      </div>

      <div className="flex-1 hover-scrollbar overflow-y-auto overflow-x-hidden min-w-0 bg-black/[0.015] dark:bg-black/10">

        {loadingKbs ? (
          <div className="p-4 text-xs text-[var(--color-kb-text-muted)]">{t('loading', { ns: 'common' })}</div>
        ) : (
          <div className="py-2">
            {/* 1. 订阅知识库 (Subscribed Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <div 
                className="flex items-center justify-between px-4 py-2 group/header mb-1 cursor-pointer select-none" 
                onClick={() => setExpanded(prev => ({ ...prev, subscribed: !prev.subscribed }))}
              >
                <div className="flex items-center gap-1.5 min-w-0 text-[13px] font-semibold text-[var(--color-kb-text-heading)] uppercase tracking-wider pl-1 font-sans">
                  <span className="text-[var(--color-kb-text-muted)] group-hover/header:text-[var(--color-kb-text-heading)] transition-colors shrink-0">
                    {expanded.subscribed ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                  </span>
                  <span>{t('subscribedKb')}</span>
                </div>
                <button 
                  onClick={(e) => {
                    e.stopPropagation();
                    onOpenMarket();
                  }}
                  className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-colors p-0.5 rounded hover:bg-[var(--color-kb-panel-hover)]" 
                  title={t('subscribeNewSharedKb')}
                >
                  <Plus size={14} />
                </button>
              </div>
              {expanded.subscribed && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {subscribedKbs.length === 0 ? (
                    <div className="px-5 py-2 text-[11px] text-[var(--color-kb-text-muted)] italic">
                      {t('noSubscribedKb')}
                    </div>
                  ) : (
                    visibleSubscribed.map((kb) => (
                      <KnowledgeBaseItem
                        key={kb.id}
                        kb={kb}
                        activeKb={activeKb}
                        onSelectKb={onSelectKb}
                        onRename={setRenameItem}
                        onDelete={handleDeleteKb}
                        onOpenSettings={onOpenSettings}
                        onImportGit={onImportGit}
                        onSyncGit={onSyncGit}
                        onImportCloudDrive={onImportCloudDrive}
                        t={t}
                      />
                    ))
                  )}
                  {subscribedKbs.length > limitSubscribed ? (
                    <button 
                      onClick={() => setLimitSubscribed(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: subscribedKbs.length - limitSubscribed })}
                    </button>
                  ) : subscribedKbs.length > 5 ? (
                    <button 
                      onClick={() => setLimitSubscribed(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-zinc-500 hover:text-zinc-900 dark:text-[var(--color-kb-text-muted)] dark:hover:text-zinc-200 hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null}
                </div>
              )}
            </div>

            {/* 2. 团队知识库 (Team Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <div 
                className="flex items-center justify-between px-4 py-2 group/header mb-1 cursor-pointer select-none" 
                onClick={() => setExpanded(prev => ({ ...prev, team: !prev.team }))}
              >
                <div className="flex items-center gap-1.5 min-w-0 text-[13px] font-semibold text-[var(--color-kb-text-heading)] uppercase tracking-wider pl-1 font-sans">
                  <span className="text-[var(--color-kb-text-muted)] group-hover/header:text-[var(--color-kb-text-heading)] transition-colors shrink-0">
                    {expanded.team ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                  </span>
                  <span>{t('teamKb')}</span>
                </div>
                <button 
                  onClick={(e) => {
                    e.stopPropagation();
                    onCreateKbSelect('team');
                  }}
                  className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-colors p-0.5 rounded hover:bg-[var(--color-kb-panel-hover)]" 
                  title={t('newKb')}
                >
                  <Plus size={14} />
                </button>
              </div>
              {expanded.team && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {visibleTeam.map((kb) => (
                    <KnowledgeBaseItem
                      key={kb.id}
                      kb={kb}
                      activeKb={activeKb}
                      onSelectKb={onSelectKb}
                      onRename={setRenameItem}
                      onDelete={handleDeleteKb}
                      onOpenSettings={onOpenSettings}
                      onImportGit={onImportGit}
                      onSyncGit={onSyncGit}
                      onImportCloudDrive={onImportCloudDrive}
                      t={t}
                    />
                  ))}
                  {kbs.team.length > limitTeam ? (
                    <button 
                      onClick={() => setLimitTeam(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: kbs.team.length - limitTeam })}
                    </button>
                  ) : kbs.team.length > 5 ? (
                    <button 
                      onClick={() => setLimitTeam(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-zinc-500 hover:text-zinc-900 dark:text-[var(--color-kb-text-muted)] dark:hover:text-zinc-200 hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null}
                </div>
              )}
            </div>
            
            {/* 3. 个人知识库 (Personal Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <div 
                className="flex items-center justify-between px-4 py-2 group/header mb-1 cursor-pointer select-none" 
                onClick={() => setExpanded(prev => ({ ...prev, personal: !prev.personal }))}
              >
                <div className="flex items-center gap-1.5 min-w-0 text-[13px] font-semibold text-[var(--color-kb-text-heading)] uppercase tracking-wider pl-1 font-sans">
                  <span className="text-[var(--color-kb-text-muted)] group-hover/header:text-[var(--color-kb-text-heading)] transition-colors shrink-0">
                    {expanded.personal ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                  </span>
                  <span>{t('personalKb')}</span>
                </div>
                <button 
                  onClick={(e) => {
                    e.stopPropagation();
                    onCreateKbSelect('personal');
                  }}
                  className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-colors p-0.5 rounded hover:bg-[var(--color-kb-panel-hover)]" 
                  title={t('newKb')}
                >
                  <Plus size={14} />
                </button>
              </div>
              {expanded.personal && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {visiblePersonal.map((kb) => (
                    <KnowledgeBaseItem
                      key={kb.id}
                      kb={kb}
                      activeKb={activeKb}
                      onSelectKb={onSelectKb}
                      onRename={setRenameItem}
                      onDelete={handleDeleteKb}
                      onOpenSettings={onOpenSettings}
                      onImportGit={onImportGit}
                      onSyncGit={onSyncGit}
                      onImportCloudDrive={onImportCloudDrive}
                      t={t}
                    />
                  ))}
                  {kbs.personal.length > limitPersonal ? (
                    <button 
                      onClick={() => setLimitPersonal(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: kbs.personal.length - limitPersonal })}
                    </button>
                  ) : kbs.personal.length > 5 ? (
                    <button 
                      onClick={() => setLimitPersonal(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-zinc-500 hover:text-zinc-900 dark:text-[var(--color-kb-text-muted)] dark:hover:text-zinc-200 hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null}
                </div>
              )}
            </div>

            {/* 4. 共享知识库 (Shared Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <div 
                className="flex items-center justify-between px-4 py-2 group/header mb-1 cursor-pointer select-none" 
                onClick={() => setExpanded(prev => ({ ...prev, public: !prev.public }))}
              >
                <div className="flex items-center gap-1.5 min-w-0 text-[13px] font-semibold text-[var(--color-kb-text-heading)] uppercase tracking-wider pl-1 flex items-center gap-1 font-sans">
                  <span className="text-[var(--color-kb-text-muted)] group-hover/header:text-[var(--color-kb-text-heading)] transition-colors shrink-0">
                    {expanded.public ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                  </span>
                  <span>{t('sharedKb')}</span>
                </div>
                <button 
                  onClick={(e) => {
                    e.stopPropagation();
                    onCreateKbSelect('public');
                  }}
                  className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-colors p-0.5 rounded hover:bg-[var(--color-kb-panel-hover)]" 
                  title={t('newKb')}
                >
                  <Plus size={14} />
                </button>
              </div>
              {expanded.public && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {visiblePublic.map((kb) => (
                    <KnowledgeBaseItem
                      key={kb.id}
                      kb={kb}
                      activeKb={activeKb}
                      onSelectKb={onSelectKb}
                      onRename={setRenameItem}
                      onDelete={handleDeleteKb}
                      onOpenSettings={onOpenSettings}
                      onImportGit={onImportGit}
                      onSyncGit={onSyncGit}
                      onImportCloudDrive={onImportCloudDrive}
                      t={t}
                    />
                  ))}
                  {regularPublicKbs.length > limitPublic ? (
                    <button 
                      onClick={() => setLimitPublic(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: regularPublicKbs.length - limitPublic })}
                    </button>
                  ) : regularPublicKbs.length > 5 ? (
                    <button 
                      onClick={() => setLimitPublic(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-zinc-500 hover:text-zinc-900 dark:text-[var(--color-kb-text-muted)] dark:hover:text-zinc-200 hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null}
                </div>
              )}
            </div>
            <div className="h-4 w-full shrink-0"></div>
          </div>
        )}
      </div>

      <div 
        className={`absolute top-0 right-[-3px] w-[6px] h-full cursor-col-resize z-20 group ${isDragging ? 'bg-[var(--color-kb-accent)]/20' : 'hover:bg-[var(--color-kb-accent)]/10'}`}
        onMouseDown={onMouseDownDrag}
      >
        <div className={`absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[2px] h-8 rounded-full ${isDragging ? 'bg-[var(--color-kb-accent)]' : 'bg-transparent group-hover:bg-[var(--color-kb-accent)]/50'}`} />
      </div>

      {renameItem && (
        <RenameModal 
          initialTitle={renameItem.title} 
          onClose={() => setRenameItem(null)} 
          onConfirm={(newTitle) => {
            DocumentService.updateKnowledgeBase(renameItem.id, { title: newTitle }).then(() => {
              setRenameItem(null);
              if (onUpdateKbs) onUpdateKbs();
            });
          }} 
        />
      )}
    </div>
  );
}
