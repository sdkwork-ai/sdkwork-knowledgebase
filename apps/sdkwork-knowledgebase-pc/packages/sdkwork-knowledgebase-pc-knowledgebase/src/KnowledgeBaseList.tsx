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

function KnowledgeBaseGroupSkeleton({ rows, loadingLabel }: { rows: number; loadingLabel: string }) {
  return (
    <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col" role="status">
      <span className="sr-only">{loadingLabel}</span>
      {Array.from({ length: rows }, (_, index) => (
        <div
          key={index}
          aria-hidden="true"
          className="flex items-center h-[32px] px-2 mb-[2px] w-full rounded-[8px]"
        >
          <div className="w-[22px] h-[22px] rounded-lg mr-2.5 flex-shrink-0 bg-[var(--color-kb-panel-hover)] animate-pulse" />
          <div
            className="h-[10px] rounded-full bg-[var(--color-kb-panel-hover)] animate-pulse"
            style={{ width: `${58 + ((index * 17) % 26)}%` }}
          />
        </div>
      ))}
    </div>
  );
}

interface KnowledgeBaseGroupHeaderProps {
  label: string;
  count: number;
  expanded: boolean;
  onToggle: () => void;
  actionTitle: string;
  onAction: () => void;
}

function KnowledgeBaseGroupHeader({ label, count, expanded, onToggle, actionTitle, onAction }: KnowledgeBaseGroupHeaderProps) {
  return (
    <div
      className="flex items-center justify-between px-4 py-2 group/header mb-1 cursor-pointer select-none"
      onClick={onToggle}
    >
      <div className="flex items-center gap-1.5 min-w-0 text-[12px] font-semibold text-[var(--color-kb-text-heading)] uppercase tracking-wider pl-1 font-sans">
        <span className="text-[var(--color-kb-text-muted)] group-hover/header:text-[var(--color-kb-accent)] transition-colors shrink-0">
          {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        </span>
        <span className="truncate">{label}</span>
        {!expanded && count > 0 && (
          <span className="ml-0.5 px-1.5 py-px rounded-full text-[10px] font-semibold leading-[14px] bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] shrink-0">
            {count}
          </span>
        )}
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onAction();
        }}
        className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-colors p-0.5 rounded hover:bg-[var(--color-kb-panel-hover)] opacity-0 group-hover/header:opacity-100 focus-visible:opacity-100"
        title={actionTitle}
      >
        <Plus size={14} />
      </button>
    </div>
  );
}

export function KnowledgeBaseList({ kbs, loadingKbs, activeKb, onSelectKb, onCreateKbSelect, onOpenSettings, onOpenMarket, onImportGit, onSyncGit, onImportCloudDrive, onUpdateKbs, width = 240, isDragging, onMouseDownDrag }: KnowledgeBaseListProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [renameItem, setRenameItem] = useState<KnowledgeBase | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({
    personal: true,
    team: true,
    subscribed: true,
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
      <div className="px-5 h-[40px] flex items-center justify-between min-w-0 bg-[var(--color-kb-panel)] z-10 border-b border-[var(--color-kb-panel-border)] shadow-sm flex-none">
        <div className="flex items-center min-w-0 pr-2">
          <div className="w-6 h-6 rounded-md bg-gradient-to-br from-[var(--color-kb-accent)] to-[var(--color-kb-accent-hover)] text-white flex items-center justify-center mr-2.5 shadow-sm shrink-0">
            <Box size={14} />
          </div>
          <h2 className="font-bold text-[14px] tracking-wide text-[var(--color-kb-text-heading)] truncate">{t('kbManagement')}</h2>
        </div>
        <button onClick={() => onCreateKbSelect('team')} className="p-1.5 hover:bg-[var(--color-kb-panel-hover)] rounded-md text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] transition-all" title={t('newKb')}>
          <Plus size={15} />
        </button>
      </div>

      <div className="flex-1 hover-scrollbar overflow-y-auto overflow-x-hidden min-w-0">

        <div className="py-2" aria-busy={loadingKbs}>
            {/* 1. 个人知识库 (Personal Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <KnowledgeBaseGroupHeader
                label={t('personalKb')}
                count={kbs.personal.length}
                expanded={expanded.personal}
                onToggle={() => setExpanded(prev => ({ ...prev, personal: !prev.personal }))}
                actionTitle={t('newKb')}
                onAction={() => onCreateKbSelect('personal')}
              />
              {expanded.personal && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {loadingKbs ? (
                    <KnowledgeBaseGroupSkeleton rows={3} loadingLabel={t('loading', { ns: 'common' })} />
                  ) : kbs.personal.length === 0 ? (
                    <div className="px-5 py-2 text-[11px] text-[var(--color-kb-text-muted)] italic">
                      {t('noPersonalKb')}
                    </div>
                  ) : (
                    visiblePersonal.map((kb) => (
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
                  {!loadingKbs && (kbs.personal.length > limitPersonal ? (
                    <button
                      onClick={() => setLimitPersonal(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: kbs.personal.length - limitPersonal })}
                    </button>
                  ) : kbs.personal.length > 5 ? (
                    <button
                      onClick={() => setLimitPersonal(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null)}
                </div>
              )}
            </div>

            {/* 2. 团队知识库 (Team Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <KnowledgeBaseGroupHeader
                label={t('teamKb')}
                count={kbs.team.length}
                expanded={expanded.team}
                onToggle={() => setExpanded(prev => ({ ...prev, team: !prev.team }))}
                actionTitle={t('newKb')}
                onAction={() => onCreateKbSelect('team')}
              />
              {expanded.team && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {loadingKbs ? (
                    <KnowledgeBaseGroupSkeleton rows={3} loadingLabel={t('loading', { ns: 'common' })} />
                  ) : kbs.team.length === 0 ? (
                    <div className="px-5 py-2 text-[11px] text-[var(--color-kb-text-muted)] italic">
                      {t('noTeamKb')}
                    </div>
                  ) : (
                    visibleTeam.map((kb) => (
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
                  {!loadingKbs && (kbs.team.length > limitTeam ? (
                    <button
                      onClick={() => setLimitTeam(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: kbs.team.length - limitTeam })}
                    </button>
                  ) : kbs.team.length > 5 ? (
                    <button
                      onClick={() => setLimitTeam(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null)}
                </div>
              )}
            </div>

            {/* 3. 订阅知识库 (Subscribed Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <KnowledgeBaseGroupHeader
                label={t('subscribedKb')}
                count={subscribedKbs.length}
                expanded={expanded.subscribed}
                onToggle={() => setExpanded(prev => ({ ...prev, subscribed: !prev.subscribed }))}
                actionTitle={t('subscribeNewSharedKb')}
                onAction={onOpenMarket}
              />
              {expanded.subscribed && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {loadingKbs ? (
                    <KnowledgeBaseGroupSkeleton rows={2} loadingLabel={t('loading', { ns: 'common' })} />
                  ) : subscribedKbs.length === 0 ? (
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
                  {!loadingKbs && (subscribedKbs.length > limitSubscribed ? (
                    <button
                      onClick={() => setLimitSubscribed(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: subscribedKbs.length - limitSubscribed })}
                    </button>
                  ) : subscribedKbs.length > 5 ? (
                    <button
                      onClick={() => setLimitSubscribed(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null)}
                </div>
              )}
            </div>

            {/* 4. 共享知识库 (Shared Knowledge Bases) */}
            <div className="flex flex-col mb-2 min-w-0">
              <KnowledgeBaseGroupHeader
                label={t('sharedKb')}
                count={regularPublicKbs.length}
                expanded={expanded.public}
                onToggle={() => setExpanded(prev => ({ ...prev, public: !prev.public }))}
                actionTitle={t('newKb')}
                onAction={() => onCreateKbSelect('public')}
              />
              {expanded.public && (
                <div className="space-y-[2px] px-[5px] min-w-0 overflow-hidden flex flex-col">
                  {loadingKbs ? (
                    <KnowledgeBaseGroupSkeleton rows={2} loadingLabel={t('loading', { ns: 'common' })} />
                  ) : regularPublicKbs.length === 0 ? (
                    <div className="px-5 py-2 text-[11px] text-[var(--color-kb-text-muted)] italic">
                      {t('noSharedKb')}
                    </div>
                  ) : (
                    visiblePublic.map((kb) => (
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
                  {!loadingKbs && (regularPublicKbs.length > limitPublic ? (
                    <button
                      onClick={() => setLimitPublic(prev => prev + 5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-semibold text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-accent)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showMore', { count: regularPublicKbs.length - limitPublic })}
                    </button>
                  ) : regularPublicKbs.length > 5 ? (
                    <button
                      onClick={() => setLimitPublic(5)}
                      className="mt-1 mx-[5px] py-1 text-[11px] font-medium text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-all flex items-center justify-center gap-1 shrink-0 select-none cursor-pointer"
                    >
                      {t('showLess')}
                    </button>
                  ) : null)}
                </div>
              )}
            </div>
            <div className="h-4 w-full shrink-0"></div>
          </div>
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
