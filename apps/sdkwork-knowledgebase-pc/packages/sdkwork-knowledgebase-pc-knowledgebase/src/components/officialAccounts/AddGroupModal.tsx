import { isBlank } from '@sdkwork/utils';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface Props {
  showGroupManager: boolean;
  setShowGroupManager: (show: boolean) => void;
  newGroupNameInput: string;
  setNewGroupNameInput: (val: string) => void;
  handleGroupAdd: () => void;
}

export function AddGroupModal({
  showGroupManager,
  setShowGroupManager,
  newGroupNameInput,
  setNewGroupNameInput,
  handleGroupAdd
}: Props) {
  const { t } = useTranslation('officialAccount');

  if (!showGroupManager) return null;

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/40 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-xl w-[320px] shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        <div className="px-5 py-3.5 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between bg-[var(--color-kb-panel)]">
          <h3 className="text-[14px] font-bold text-[var(--color-kb-text-heading)]">{t('addNewGroup')}</h3>
          <button onClick={() => { setShowGroupManager(false); setNewGroupNameInput(''); }} className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] transition-colors">
            <X size={16} />
          </button>
        </div>
        <div className="p-5">
          <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('groupName')}</label>
          <input
            autoFocus
            type="text"
            value={newGroupNameInput}
            onChange={(e) => setNewGroupNameInput(e.target.value)}
            placeholder={t('inputGroupNamePlaceholder')}
            className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-[13px] text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
            onKeyDown={(e) => e.key === 'Enter' && handleGroupAdd()}
          />
        </div>
        <div className="px-5 py-3 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] flex justify-end gap-2">
          <button onClick={() => { setShowGroupManager(false); setNewGroupNameInput(''); }} className="px-4 py-1.5 text-[12px] font-medium text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)] rounded-md transition-colors">
            {t('cancel')}
          </button>
          <button 
            onClick={handleGroupAdd}
            disabled={isBlank(newGroupNameInput)}
            className="px-4 py-1.5 text-[12px] font-bold text-white bg-[#07c160] hover:bg-[#06ad56] disabled:opacity-50 rounded-md transition-all shadow-sm"
          >
            {t('confirm')}
          </button>
        </div>
      </div>
    </div>
  );
}
