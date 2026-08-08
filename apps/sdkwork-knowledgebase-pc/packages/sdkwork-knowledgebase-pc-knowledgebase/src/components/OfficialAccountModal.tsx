import { useEffect, useState } from 'react';
import { X, Settings2, LayoutList, LayoutGrid, Plus, Check } from 'lucide-react';
import { useTranslation, Trans } from 'react-i18next';
import {
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';
import { OfficialAccount } from '../services/wechat';
import { toast } from './ui/toast-manager';
import { OASidebar } from './officialAccounts/OASidebar';
import { OAGrid } from './officialAccounts/OAGrid';
import { OAEditorDrawer } from './officialAccounts/OAEditorDrawer';
import { AddGroupModal } from './officialAccounts/AddGroupModal';

interface OfficialAccountModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (data: { 
    officialAccounts: OfficialAccount[]; 
    selectedOfficialAccountIds: string[]; 
    oaGroups: string[];
  }) => Promise<void>;
  initialOfficialAccounts: OfficialAccount[];
  initialSelectedAccountIds: string[];
  initialOaGroups: string[];
}

export function OfficialAccountModal({
  isOpen,
  onClose,
  onConfirm,
  initialOfficialAccounts,
  initialSelectedAccountIds,
  initialOaGroups
}: OfficialAccountModalProps) {
  const { t } = useTranslation('officialAccount');
  const [officialAccounts, setOfficialAccounts] = useState<OfficialAccount[]>(initialOfficialAccounts);
  const [selectedOfficialAccountIds, setSelectedOfficialAccountIds] = useState<string[]>(initialSelectedAccountIds);
  const [oaGroups, setOaGroups] = useState<string[]>(initialOaGroups);

  const [selectedGroupFilter, setSelectedGroupFilter] = useState<string>('all');
  const [showGroupManager, setShowGroupManager] = useState<boolean>(false);
  const [newGroupNameInput, setNewGroupNameInput] = useState<string>('');

  const [oaEditingId, setOaEditingId] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const [confirmError, setConfirmError] = useState<string | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setOfficialAccounts(initialOfficialAccounts);
    setSelectedOfficialAccountIds(initialSelectedAccountIds);
    setOaGroups(initialOaGroups);
    setConfirmError(null);
  }, [initialOaGroups, initialOfficialAccounts, initialSelectedAccountIds, isOpen]);

  if (!isOpen) return null;

  const openEditor = (oa?: OfficialAccount) => {
    if (oa) {
      setOaEditingId(oa.id);
    } else {
      setOaEditingId('new');
    }
  };

  const closeEditor = () => {
    setOaEditingId(null);
  };

  const handleSaveOA = (newOA: OfficialAccount) => {
    let newAccounts;
    if (oaEditingId === 'new') {
      newAccounts = [...officialAccounts, newOA];
      if (!selectedOfficialAccountIds.includes(newOA.id)) {
        setSelectedOfficialAccountIds(prev => [...prev, newOA.id]);
      }
    } else if (oaEditingId) {
      newAccounts = officialAccounts.map(app => app.id === oaEditingId ? newOA : app);
    } else {
      return;
    }

    setOfficialAccounts(newAccounts);
    closeEditor();
  };

  const handleGroupDelete = (grp: string) => {
    if (confirm(t('errors.confirmDeleteGroup', { group: grp }))) {
      const newGroups = oaGroups.filter(g => g !== grp);
      const newAccounts = officialAccounts.map(app => app.group === grp ? { ...app, group: t('unassignedGroup') } : app);
      setOaGroups(newGroups);
      setOfficialAccounts(newAccounts);
      if (selectedGroupFilter === grp) {
        setSelectedGroupFilter('all');
      }
    }
  };

  const handleGroupAdd = () => {
    if (newGroupNameInput.trim()) {
      const trimmedName = newGroupNameInput.trim();
      if (oaGroups.includes(trimmedName)) {
        toast.error(t('errors.groupExists'));
        return;
      }
      const newGroups = [...oaGroups, trimmedName];
      setOaGroups(newGroups);
      setNewGroupNameInput('');
      setShowGroupManager(false);
    }
  };

  const handleConfirmAndClose = async () => {
    setConfirmError(null);
    setIsConfirming(true);
    try {
      await onConfirm({
        officialAccounts,
        selectedOfficialAccountIds,
        oaGroups
      });
      toast.success(t('errors.saveSuccess'));
    } catch (error) {
      setConfirmError(
        resolveUserFacingErrorMessage(error, t as unknown as ErrorTranslateFn),
      );
    } finally {
      setIsConfirming(false);
    }
  };

  const filteredList = officialAccounts.filter(app => selectedGroupFilter === 'all' || app.group === selectedGroupFilter);

  return (
    <div className="fixed inset-0 bg-zinc-950/40 backdrop-blur-md z-[600] flex items-center justify-center p-4 md:p-8 animate-in fade-in duration-200">
      <div className={`bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-2xl w-full max-w-[1040px] h-full max-h-[80vh] shadow-2xl flex flex-col overflow-hidden transition-all duration-300 ${oaEditingId ? 'scale-[0.98] opacity-60 pointer-events-none' : 'scale-100 opacity-100'}`}>
        
        {/* Modal Header */}
        <div className="h-16 px-6 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between shrink-0 bg-[var(--color-kb-panel)]">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-[#07c160]/10 flex items-center justify-center text-[#07c160]">
              <Settings2 size={22} />
            </div>
            <div>
              <h2 className="text-sm font-bold text-[var(--color-kb-text-heading)]">{t('managerTitle')}</h2>
              <p className="text-xs text-[var(--color-kb-text-muted)] mt-0.5">{t('managerDesc')}</p>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="w-8 h-8 flex items-center justify-center hover:bg-[var(--color-kb-panel-hover)] rounded-lg text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] cursor-pointer transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {confirmError && (
          <div role="alert" className="px-6 py-2 text-xs text-red-500 bg-red-500/5 border-b border-red-500/20">
            {confirmError}
          </div>
        )}

        {/* Modal Body */}
        <div className="flex flex-1 overflow-hidden relative bg-[var(--color-kb-panel)]">
          
          <OASidebar 
            groups={oaGroups}
            officialAccounts={officialAccounts}
            selectedGroupFilter={selectedGroupFilter}
            setSelectedGroupFilter={setSelectedGroupFilter}
            showGroupManager={showGroupManager}
            setShowGroupManager={setShowGroupManager}
            handleGroupDelete={handleGroupDelete}
          />

          <div className="flex-1 flex flex-col relative overflow-hidden">
            <div className="px-6 py-4 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between bg-[var(--color-kb-panel)] shrink-0">
              <h3 className="text-sm font-bold text-[var(--color-kb-text-heading)] flex items-center gap-2">
                {selectedGroupFilter === 'all' ? t('pendingAccountsList') : selectedGroupFilter}
                <span className="text-xs font-medium text-[var(--color-kb-text-muted)] bg-[var(--color-kb-editor)] px-2 py-0.5 rounded-full border border-[var(--color-kb-panel-border)]">{filteredList.length}</span>
              </h3>
              <div className="flex items-center gap-3">
                <div className="hidden sm:flex bg-[var(--color-kb-editor)] p-1 rounded-lg border border-[var(--color-kb-panel-border)]">
                  <button onClick={() => setViewMode('list')} className={`p-1.5 rounded-md transition-colors ${viewMode === 'list' ? 'bg-[var(--color-kb-panel)] text-[#07c160] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`} title={t('listView')}>
                    <LayoutList size={16} strokeWidth={2.5} />
                  </button>
                  <button onClick={() => setViewMode('grid')} className={`p-1.5 rounded-md transition-colors ${viewMode === 'grid' ? 'bg-[var(--color-kb-panel)] text-[#07c160] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`} title={t('gridView')}>
                    <LayoutGrid size={16} strokeWidth={2.5} />
                  </button>
                </div>
                <button 
                  onClick={() => openEditor()}
                  className="flex items-center gap-1.5 px-4 py-2 bg-[#07c160] hover:bg-[#06ad56] text-white text-[13px] font-bold rounded-lg transition-all shadow-sm"
                >
                  <Plus size={16} />
                  {t('addNewConfig')}
                </button>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-6 bg-[var(--color-kb-editor)]">
              <div className={`grid ${viewMode === 'grid' ? 'grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4' : 'grid-cols-1 gap-3'}`}>
                 <OAGrid 
                   filteredList={filteredList}
                   viewMode={viewMode}
                   selectedOfficialAccountIds={selectedOfficialAccountIds}
                   setSelectedOfficialAccountIds={setSelectedOfficialAccountIds}
                   openEditor={openEditor}
                 />
              </div>
             </div>
             
             {/* Right main panel footer */}
             <div className="p-4 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] flex justify-between items-center shrink-0 z-20">
                <div className="flex items-center gap-2 pl-2">
                  <span className="text-sm text-[var(--color-kb-text)]">
                    <Trans
                      i18nKey="selectedAccountCount"
                      ns="officialAccount"
                      count={selectedOfficialAccountIds.length}
                      components={{
                        1: <span className="font-bold text-[#07c160] text-base px-1" />
                      }}
                    />
                  </span>
                </div>
                <div className="flex items-center gap-3">
                  <button 
                    onClick={onClose} 
                    className="px-5 py-2 text-[13px] font-bold bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-heading)] rounded-lg transition-all"
                  >
                    {t('cancel')}
                  </button>
                  <button 
                    onClick={handleConfirmAndClose} 
                    disabled={selectedOfficialAccountIds.length === 0 || isConfirming}
                    className="px-6 py-2 text-[13px] font-bold bg-[#07c160] hover:bg-[#06ad56] disabled:opacity-40 disabled:hover:bg-[#07c160] text-white rounded-lg transition-all flex items-center gap-2"
                  >
                    {t('confirmAndContinue')}
                    <Check size={16} strokeWidth={3} />
                  </button>
                </div>
             </div>
          </div>
        </div>

        <AddGroupModal
          showGroupManager={showGroupManager}
          setShowGroupManager={setShowGroupManager}
          newGroupNameInput={newGroupNameInput}
          setNewGroupNameInput={setNewGroupNameInput}
          handleGroupAdd={handleGroupAdd}
        />
      </div>

      <OAEditorDrawer 
        oaEditingId={oaEditingId}
        officialAccountData={officialAccounts.find(oa => oa.id === oaEditingId)}
        groups={oaGroups}
        onClose={closeEditor}
        onSave={handleSaveOA}
      />
    </div>
  );
}
