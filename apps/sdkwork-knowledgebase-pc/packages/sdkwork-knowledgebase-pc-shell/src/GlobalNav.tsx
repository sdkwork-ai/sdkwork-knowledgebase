import { useMemo } from 'react';
import { BookOpen, Shield, StickyNote, Store, Settings, Search, UserRound } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useKnowledgebaseRuntime, type KnowledgebaseAccountViewModel } from 'sdkwork-knowledgebase-pc-core';

export interface GlobalNavProps {
  account?: KnowledgebaseAccountViewModel;
  activeTab: string;
  onTabChange: (tab: string) => void;
  onOpenSettings: () => void;
  onOpenProfile: () => void;
  showAdminConsole?: boolean;
  onOpenAdminConsole?: () => void;
}

export function GlobalNav({
  account,
  activeTab,
  onTabChange,
  onOpenSettings,
  onOpenProfile,
  showAdminConsole = false,
  onOpenAdminConsole,
}: GlobalNavProps) {
  const { t } = useTranslation('shell');

  // Consume the composed app runtime instead of rebuilding a fresh runtime
  // config on every render: host-injected overrides must stay authoritative
  // and identical to what the rest of the shell sees.
  const { config } = useKnowledgebaseRuntime();

  const navItems = useMemo(() => {
    const items = [
      { id: 'kb', icon: BookOpen, title: t('myKnowledgeBase') },
      { id: 'notes', icon: StickyNote, title: t('notesNav') },
      { id: 'search', icon: Search, title: t('search') },
    ];
    if (config.featureFlags.knowledgeMarketCatalog) {
      items.push({ id: 'market', icon: Store, title: t('knowledgeBaseMarket') });
    }
    return items;
  }, [config.featureFlags.knowledgeMarketCatalog, t]);

  const userAvatar = account?.avatarUrl?.trim();
  const userInitials = account?.initials?.trim();

  return (
    <div className="w-[64px] min-w-[64px] h-full flex flex-col items-center py-4 space-y-6 bg-[var(--color-kb-nav)] z-10 border-r border-[var(--color-kb-panel-border)]/70">
      <button
        type="button"
        onClick={onOpenProfile}
        className="relative w-10 h-10 rounded-xl overflow-hidden cursor-pointer shrink-0 mt-2 hover:ring-2 hover:ring-indigo-500/55 dark:hover:ring-indigo-400/55 hover:scale-105 active:scale-95 transition-all shadow-md bg-white dark:bg-zinc-800 flex items-center justify-center group focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500"
        title={account?.displayName || t('viewProfile')}
        aria-label={account?.displayName || t('viewProfile')}
      >
        {userAvatar ? (
          <img src={userAvatar} alt="" className="w-full h-full object-cover" />
        ) : (
          <span aria-hidden="true" className="text-sm font-bold text-[var(--color-kb-accent)]">
            {userInitials || <UserRound size={18} />}
          </span>
        )}
        
        {/* Hover overlay indicator */}
        <div className="absolute inset-0 bg-black/10 opacity-0 group-hover:opacity-100 transition-opacity" />
      </button>

      <div className="flex-1 w-full flex flex-col items-center space-y-4 pt-4">
        {navItems.map(item => {
          const Icon = item.icon;
          const isActive = activeTab === item.id;
          return (
            <button 
              key={item.id}
              onClick={() => onTabChange(item.id)}
              title={item.title}
              data-testid={`knowledgebase-pc-nav-${item.id}`}
              className={`w-11 h-11 rounded-xl flex justify-center items-center transition-all duration-350 relative group ${isActive ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-accent)]' : 'text-[var(--color-kb-nav-text)] hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-nav-text-hover)]'}`}
            >
              <Icon size={22} strokeWidth={isActive ? 2.5 : 1.5} />
              
              {/* Tooltip */}
              <div className="absolute left-[70px] px-2.5 py-1.5 rounded-lg bg-zinc-900 text-white text-[11px] font-bold shadow-xl opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-[1000] border border-zinc-800">
                {item.title}
              </div>
            </button>
          )
        })}
      </div>

      <div className="w-full flex flex-col items-center pb-2 space-y-3 col-shrink-0">
        {showAdminConsole && onOpenAdminConsole ? (
          <button
            type="button"
            onClick={onOpenAdminConsole}
            className="text-[var(--color-kb-nav-text)] hover:text-[var(--color-kb-nav-text-hover)] hover:bg-[var(--color-kb-panel-hover)] w-10 h-10 rounded-xl flex items-center justify-center transition-colors"
            title={t('adminConsoleNav')}
            data-testid="knowledgebase-pc-nav-admin"
          >
            <Shield size={22} strokeWidth={1.5} />
          </button>
        ) : null}
        <button 
          onClick={onOpenSettings}
          className="text-[var(--color-kb-nav-text)] hover:text-[var(--color-kb-nav-text-hover)] hover:bg-[var(--color-kb-panel-hover)] w-10 h-10 rounded-xl flex items-center justify-center transition-colors"
          title={t('settings')}
        >
          <Settings size={22} strokeWidth={1.5} />
        </button>
      </div>
    </div>
  );
}

