import { Smartphone, Check, Settings2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { WechatAppletConfig } from '../../services/wechat';

interface Props {
  filteredApplets: WechatAppletConfig[];
  viewMode: 'list' | 'grid';
  selectedAppletId: string | null;
  setSelectedAppletId: (id: string | null) => void;
  openEditor: (applet?: WechatAppletConfig) => void;
}

export function AppletGrid({
  filteredApplets,
  selectedAppletId,
  setSelectedAppletId,
  openEditor
}: Props) {
  const { t } = useTranslation('applet');

  if (filteredApplets.length === 0) {
    return (
      <div className="col-span-full flex flex-col items-center justify-center py-24 border-2 border-dashed border-[var(--color-kb-panel-border)] rounded-2xl bg-[var(--color-kb-panel)]">
        <Smartphone size={48} className="text-[var(--color-kb-text-muted)] mb-4 opacity-50" />
        <span className="text-[15px] font-bold text-[var(--color-kb-text-heading)] mb-1">{t('emptyApplets')}</span>
        <span className="text-[13px] font-medium text-[var(--color-kb-text-muted)]">{t('emptyAppletsHint')}</span>
      </div>
    );
  }

  return (
    <>
      {filteredApplets.map((app) => {
        const isSelected = selectedAppletId === app.id;
        return (
          <div 
            key={app.id} 
            className={`group flex items-start gap-4 bg-[var(--color-kb-panel)] border ${isSelected ? 'border-[#07c160] ring-2 ring-[#07c160]/20 bg-[#07c160]/[0.03]' : 'border-[var(--color-kb-panel-border)] hover:border-[#07c160]/40 hover:shadow-md'} rounded-2xl p-5 transition-all cursor-pointer relative`}
            onClick={() => setSelectedAppletId(isSelected ? null : app.id)}
          >
            {/* Checkbox */}
            <div className="pt-1 select-none shrink-0">
               <div className={`w-5 h-5 rounded-full border-[1.5px] flex items-center justify-center transition-all shrink-0 ${isSelected ? 'bg-[#07c160] border-[#07c160] text-white shadow-sm' : 'border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] group-hover:border-[#07c160]/50'}`}>
                  {isSelected && <Check size={12} strokeWidth={4} />}
               </div>
            </div>

            <div className="flex-1 flex flex-col justify-between overflow-hidden">
              <div className="flex items-start gap-4 h-full">
                <div className="w-14 h-14 rounded-[14px] bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] flex items-center justify-center text-3xl shadow-sm shrink-0 object-cover overflow-hidden select-none">
                   {app.avatar.length <= 2 ? app.avatar : <img src={app.avatar} alt="avatar" className="w-full h-full object-cover" />}
                </div>
                <div className="flex-1 min-w-0 pr-10">
                  <h4 className="text-[15px] font-bold text-[var(--color-kb-text-heading)] truncate mb-1">{app.name}</h4>
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-[11px] px-2 py-0.5 rounded-md font-bold tracking-wider shrink-0 shadow-sm bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400 dark:border-blue-800 border border-blue-100">
                      {t('appletLabel')}
                    </span>
                    <span className="text-[12px] text-[var(--color-kb-text-muted)] truncate font-mono">
                      {app.appId || t('unconfiguredAppId')}
                    </span>
                  </div>
                  {app.description && (
                     <p className="text-[12px] text-[var(--color-kb-text)] line-clamp-2 leading-relaxed opacity-80 mb-3">{app.description}</p>
                  )}
                </div>
              </div>
            </div>

            <button 
              onClick={(e) => { e.stopPropagation(); openEditor(app); }}
              className="absolute top-1/2 -translate-y-1/2 right-4 w-8 h-8 flex items-center justify-center text-[var(--color-kb-text-muted)] hover:text-white hover:bg-[#07c160] rounded-full transition-all opacity-0 group-hover:opacity-100 shadow-sm hover:shadow"
              title={t('editConfig', { name: app.name })}
            >
              <Settings2 size={16} />
            </button>
          </div>
        );
      })}
    </>
  );
}
