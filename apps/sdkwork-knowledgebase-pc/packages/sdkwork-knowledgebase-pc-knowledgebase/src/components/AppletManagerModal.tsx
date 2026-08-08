import { useEffect, useState } from 'react';
import { X, LayoutGrid, LayoutList, Plus, Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { WechatAppletConfig } from '../services/wechat';
import { toast } from './ui/toast-manager';
import {
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';

import { AppletSidebar } from './applets/AppletSidebar';
import { AppletGrid } from './applets/AppletGrid';
import { AppletEditorDrawer } from './applets/AppletEditorDrawer';
import { AddGroupModal } from './applets/AddGroupModal';

interface AppletManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (applet: WechatAppletConfig) => void;
  initialApplets: WechatAppletConfig[];
  initialGroups: string[];
  onSaveApplets: (applets: WechatAppletConfig[], groups: string[]) => Promise<void>;
}

export function AppletManagerModal({
  isOpen,
  onClose,
  onSelect,
  initialApplets,
  initialGroups,
  onSaveApplets
}: AppletManagerModalProps) {
  const { t } = useTranslation('applet');

  // Main states
  const [applets, setApplets] = useState<WechatAppletConfig[]>(initialApplets);
  const [groups, setGroups] = useState<string[]>(initialGroups);

  // Filter/Listing states
  const [selectedGroupFilter, setSelectedGroupFilter] = useState<string>('all');
  const [showGroupManager, setShowGroupManager] = useState<boolean>(false);
  const [newGroupNameInput, setNewGroupNameInput] = useState<string>('');

  const [selectedAppletId, setSelectedAppletId] = useState<string | null>(null);

  // Editing state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [appletData, setAppletData] = useState<WechatAppletConfig | undefined>(undefined);
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setApplets(initialApplets);
    setGroups(initialGroups);
    setSaveError(null);
  }, [initialApplets, initialGroups, isOpen]);

  if (!isOpen) return null;

  const openEditor = (applet?: WechatAppletConfig) => {
    setEditingId(applet ? applet.id : 'new');
    setAppletData(applet);
  };

  const closeEditor = () => {
    setEditingId(null);
    setAppletData(undefined);
  };

  const persistApplets = async (
    nextApplets: WechatAppletConfig[],
    nextGroups: string[],
  ): Promise<boolean> => {
    setIsSaving(true);
    setSaveError(null);
    try {
      await onSaveApplets(nextApplets, nextGroups);
      setApplets(nextApplets);
      setGroups(nextGroups);
      return true;
    } catch (error) {
      setSaveError(
        resolveUserFacingErrorMessage(error, t as unknown as ErrorTranslateFn),
      );
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveApplet = async (newApplet: WechatAppletConfig) => {
    let newApplets;
    if (editingId === 'new') {
      newApplets = [...applets, newApplet];
    } else if (editingId) {
      newApplets = applets.map(app => app.id === editingId ? newApplet : app);
    } else {
      return;
    }

    if (await persistApplets(newApplets, groups)) {
      closeEditor();
      toast.success(t('errors.saveSuccess'));
    }
  };

  // const handleDeleteApplet = (id: string) => { // Removed because it seems not directly exposed in the UI except through editor maybe? Actually keeping it out of editor for now or add it later if needed.

  const handleGroupDelete = async (grp: string) => {
    if (confirm(t('errors.confirmDeleteGroup', { group: grp }))) {
      const newGroups = groups.filter(g => g !== grp);
      const newApplets = applets.map(app => app.group === grp ? { ...app, group: '未分组' } : app);

      if (await persistApplets(newApplets, newGroups) && selectedGroupFilter === grp) {
        setSelectedGroupFilter('all');
      }
    }
  };

  const handleGroupAdd = async () => {
    if (newGroupNameInput.trim()) {
      const trimmed = newGroupNameInput.trim();
      if (groups.includes(trimmed)) {
        toast.error(t('errors.groupExists'));
        return;
      }
      const newGroups = [...groups, trimmed];
      if (await persistApplets(applets, newGroups)) {
        setNewGroupNameInput('');
        setShowGroupManager(false);
      }
    }
  };

  const filteredApplets = applets.filter(app => 
    selectedGroupFilter === 'all' || app.group === selectedGroupFilter
  );

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-[600] flex items-center justify-center p-4 md:p-8 animate-in fade-in duration-200">
      <div className={`bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-2xl w-full max-w-[1040px] h-full max-h-[80vh] shadow-2xl flex flex-col overflow-hidden transition-all duration-300 ${editingId ? 'scale-[0.98] opacity-60 pointer-events-none' : 'scale-100 opacity-100'}`}>
        
        {/* Modal Header */}
        <div className="h-16 px-6 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between shrink-0 bg-[var(--color-kb-panel)]">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-[#07c160]/10 flex items-center justify-center text-[#07c160]">
              <LayoutGrid size={22} />
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

        {saveError && (
          <div role="alert" className="px-6 py-2 text-xs text-red-500 bg-red-500/5 border-b border-red-500/20">
            {saveError}
          </div>
        )}

        {/* Modal Body */}
        <div className="flex flex-1 overflow-hidden relative bg-[var(--color-kb-panel)]">
          
          <AppletSidebar 
            groups={groups}
            applets={applets}
            selectedGroupFilter={selectedGroupFilter}
            setSelectedGroupFilter={setSelectedGroupFilter}
            showGroupManager={showGroupManager}
            setShowGroupManager={setShowGroupManager}
            handleGroupDelete={handleGroupDelete}
          />

          <div className="flex-1 flex flex-col relative overflow-hidden">
            <div className="px-6 py-4 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between bg-[var(--color-kb-panel)] shrink-0">
              <h3 className="text-[14px] font-bold text-[var(--color-kb-text-heading)] flex items-center gap-2">
                {selectedGroupFilter === 'all' ? t('allAppletConfigs') : selectedGroupFilter}
                <span className="text-xs font-medium text-[var(--color-kb-text-muted)] bg-[var(--color-kb-editor)] px-2 py-0.5 rounded-full border border-[var(--color-kb-panel-border)]">{filteredApplets.length}</span>
              </h3>
              <div className="flex items-center gap-3">
                <div className="hidden sm:flex bg-[var(--color-kb-editor)] p-1 rounded-lg border border-[var(--color-kb-panel-border)]">
                  <button onClick={() => setViewMode('list')} className={`p-1.5 rounded-md transition-colors ${viewMode === 'list' ? 'bg-[var(--color-kb-panel)] text-[#07c160] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`}>
                    <LayoutList size={16} strokeWidth={2.5} />
                  </button>
                  <button onClick={() => setViewMode('grid')} className={`p-1.5 rounded-md transition-colors ${viewMode === 'grid' ? 'bg-[var(--color-kb-panel)] text-[#07c160] shadow-sm' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'}`}>
                    <LayoutGrid size={16} strokeWidth={2.5} />
                  </button>
                </div>
                <button 
                   onClick={() => openEditor()}
                   className="flex items-center gap-1.5 px-4 py-2 bg-[#07c160] hover:bg-[#06ad56] text-white text-[13px] font-bold rounded-lg transition-all shadow-sm"
                >
                  <Plus size={16} />
                  {t('addNewApplet')}
                </button>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-6 bg-[var(--color-kb-editor)]">
              <div className={`grid ${viewMode === 'grid' ? 'grid-cols-1 lg:grid-cols-2 gap-4' : 'grid-cols-1 gap-3.5'}`}>
                <AppletGrid 
                  filteredApplets={filteredApplets}
                  viewMode={viewMode}
                  selectedAppletId={selectedAppletId}
                  setSelectedAppletId={setSelectedAppletId}
                  openEditor={openEditor}
                />
              </div>
            </div>

            {/* Right main panel footer */}
            <div className="p-4 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] flex justify-between items-center shrink-0 z-20">
               <div className="flex items-center gap-2 pl-2">
                 <span className="text-sm text-[var(--color-kb-text)]">
                   {t('selectedAppletCount', { count: selectedAppletId ? 1 : 0 })}
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
                   onClick={() => {
                     if (selectedAppletId) {
                       const selectedApplet = applets.find(app => app.id === selectedAppletId);
                       if (selectedApplet) {
                         onSelect(selectedApplet);
                       }
                     }
                   }} 
                   disabled={!selectedAppletId}
                   className="px-6 py-2 text-[13px] font-bold bg-[#07c160] hover:bg-[#06ad56] disabled:opacity-40 disabled:hover:bg-[#07c160] text-white rounded-lg transition-all flex items-center gap-2"
                 >
                   {t('confirmAndInsert')}
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

      <AppletEditorDrawer 
        editingId={editingId}
        appletData={appletData}
        groups={groups}
        onClose={closeEditor}
        onSave={handleSaveApplet}
        isSaving={isSaving}
        saveError={saveError}
      />
    </div>
  );
}
