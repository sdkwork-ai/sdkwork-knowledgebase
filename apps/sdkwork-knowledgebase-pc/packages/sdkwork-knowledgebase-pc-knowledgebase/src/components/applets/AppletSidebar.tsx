import { Plus, Folder, Tags, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { WechatAppletConfig } from '../../services/wechat';

interface Props {
  groups: string[];
  applets: WechatAppletConfig[];
  selectedGroupFilter: string;
  setSelectedGroupFilter: (group: string) => void;
  showGroupManager: boolean;
  setShowGroupManager: (show: boolean) => void;
  handleGroupDelete: (grp: string) => void;
}

export function AppletSidebar({
  groups,
  applets,
  selectedGroupFilter,
  setSelectedGroupFilter,
  showGroupManager,
  setShowGroupManager,
  handleGroupDelete
}: Props) {
  const { t } = useTranslation('applet');

  return (
    <div className="w-[240px] border-r border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] flex flex-col shrink-0 relative z-10">
      <div className="p-4 flex items-center justify-between">
        <span className="text-xs font-bold text-[var(--color-kb-text-muted)] uppercase tracking-wider">{t('businessGroups')}</span>
        <button 
          onClick={() => setShowGroupManager(!showGroupManager)}
          className="w-6 h-6 flex items-center justify-center hover:bg-[var(--color-kb-panel-hover)] rounded-md text-[var(--color-kb-text-muted)] hover:text-[#07c160] transition-colors"
          title={t('manageCustomGroups')}
        >
          <Plus size={14} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-3 pb-4 space-y-1">
        <button
          onClick={() => setSelectedGroupFilter('all')}
          className={`w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-sm transition-all ${
            selectedGroupFilter === 'all' 
              ? 'bg-[#07c160]/10 text-[#07c160] font-bold' 
              : 'hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text)] font-medium'
          }`}
        >
          <div className="flex items-center gap-2.5">
            <Folder size={16} className={selectedGroupFilter === 'all' ? 'text-[#07c160]' : 'text-[var(--color-kb-text-muted)]'} />
            <span>{t('allGroups')}</span>
          </div>
          <span className={`text-[10px] px-1.5 py-0.5 rounded-md ${selectedGroupFilter === 'all' ? 'bg-[#07c160]/20' : 'bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)]'}`}>{applets.length}</span>
        </button>
        
        {groups.map(g => {
          const count = applets.filter(a => a.group === g).length;
          const isSelected = selectedGroupFilter === g;
          return (
            <div key={g} className="group flex items-center relative">
              <button
                onClick={() => setSelectedGroupFilter(g)}
                className={`w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-sm transition-all ${
                  isSelected 
                    ? 'bg-[#07c160]/10 text-[#07c160] font-bold' 
                    : 'hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text)] font-medium'
                }`}
              >
                <div className="flex items-center gap-2.5">
                   <Tags size={16} className={isSelected ? 'text-[#07c160]' : 'text-[var(--color-kb-text-muted)]'} />
                  <span className="truncate max-w-[100px]">{g}</span>
                </div>
                <span className={`text-[10px] px-1.5 py-0.5 rounded-md ${isSelected ? 'bg-[#07c160]/20' : 'bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)]'}`}>{count}</span>
              </button>
              {showGroupManager && (
                <button 
                  onClick={() => handleGroupDelete(g)}
                  className="absolute right-2 p-1.5 text-[var(--color-kb-text-muted)] hover:text-red-500 hover:bg-red-500/10 rounded-md opacity-0 group-hover:opacity-100 transition-all"
                >
                  <Trash2 size={14} />
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
