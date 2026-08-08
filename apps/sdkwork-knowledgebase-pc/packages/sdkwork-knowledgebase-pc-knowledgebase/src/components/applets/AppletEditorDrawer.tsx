import React, { useState, useEffect, useRef } from 'react';
import { isBlank } from '@sdkwork/utils';
import { X, Smartphone, Edit3, Globe, Server, Plus, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { WechatAppletConfig } from '../../services/wechat';
import {
  formatFileSizeLimit,
  isFileWithinSizeLimit,
  MAX_DOMAIN_VERIFICATION_TEXT_BYTES,
} from '../../services/fileInputLimits';
import { toast } from '../ui/toast-manager';

interface Props {
  editingId: string | null;
  appletData: WechatAppletConfig | undefined;
  groups: string[];
  onClose: () => void;
  onSave: (applet: WechatAppletConfig) => Promise<void>;
  isSaving: boolean;
  saveError: string | null;
}

export function AppletEditorDrawer({
  editingId,
  appletData,
  groups,
  onClose,
  onSave,
  isSaving,
  saveError,
}: Props) {
  const { t } = useTranslation('applet');
  const [activeTab, setActiveTab] = useState<'basic' | 'domains' | 'server'>('basic');

  const [appName, setAppName] = useState('');
  const [appAvatar, setAppAvatar] = useState('📱');
  const [appId, setAppId] = useState('');
  const [appOriginalId, setAppOriginalId] = useState('');
  const [appSecret, setAppSecret] = useState('');
  const [appPath, setAppPath] = useState('');
  const [appGroup, setAppGroup] = useState<string>(groups.length > 0 ? groups[0] : t('unassignedGroup'));
  const [appDescription, setAppDescription] = useState('');
  const [appRequestDomain, setAppRequestDomain] = useState<string[]>([]);
  const [appSocketDomain, setAppSocketDomain] = useState<string[]>([]);
  const [appUploadDomain, setAppUploadDomain] = useState<string[]>([]);
  const [appDownloadDomain, setAppDownloadDomain] = useState<string[]>([]);
  const [appUdpDomain, setAppUdpDomain] = useState<string[]>([]);
  const [appTcpDomain, setAppTcpDomain] = useState<string[]>([]);
  const [appBusinessDomain, setAppBusinessDomain] = useState<string[]>([]);
  const [appDomainVerifyFileName, setAppDomainVerifyFileName] = useState('');
  const [appDomainVerifyFileContent, setAppDomainVerifyFileContent] = useState('');
  const [appMsgToken, setAppMsgToken] = useState('');
  const [appMsgEncodingAESKey, setAppMsgEncodingAESKey] = useState('');
  const [appMsgDataFormat, setAppMsgDataFormat] = useState<'json' | 'xml'>('json');
  const [appMsgEncryptMode, setAppMsgEncryptMode] = useState<'plain' | 'compatible' | 'safe'>('plain');

  const domainVerifyInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setActiveTab('basic');
    if (appletData) {
      setAppName(appletData.name);
      setAppAvatar(appletData.avatar || '📱');
      setAppId(appletData.appId);
      setAppOriginalId(appletData.originalId || '');
      setAppSecret(appletData.appSecret || '');
      setAppPath(appletData.path || '');
      setAppGroup(appletData.group || (groups.length > 0 ? groups[0] : t('unassignedGroup')));
      setAppDescription(appletData.description || '');
      setAppRequestDomain(appletData.requestDomain || []);
      setAppSocketDomain(appletData.socketDomain || []);
      setAppUploadDomain(appletData.uploadDomain || []);
      setAppDownloadDomain(appletData.downloadDomain || []);
      setAppUdpDomain(appletData.udpDomain || []);
      setAppTcpDomain(appletData.tcpDomain || []);
      setAppBusinessDomain(appletData.businessDomain || []);
      setAppDomainVerifyFileName(appletData.domainVerifyFileName || '');
      setAppDomainVerifyFileContent(appletData.domainVerifyFileContent || '');
      setAppMsgToken(appletData.msgToken || '');
      setAppMsgEncodingAESKey(appletData.msgEncodingAESKey || '');
      setAppMsgDataFormat(appletData.msgDataFormat || 'json');
      setAppMsgEncryptMode(appletData.msgEncryptMode || 'plain');
    } else {
      setAppName('');
      setAppAvatar('📱');
      setAppId('');
      setAppOriginalId('');
      setAppSecret('');
      setAppPath('');
      setAppGroup(groups.length > 0 ? groups[0] : t('unassignedGroup'));
      setAppDescription('');
      setAppRequestDomain([]);
      setAppSocketDomain([]);
      setAppUploadDomain([]);
      setAppDownloadDomain([]);
      setAppUdpDomain([]);
      setAppTcpDomain([]);
      setAppBusinessDomain([]);
      setAppDomainVerifyFileName('');
      setAppDomainVerifyFileContent('');
      setAppMsgToken('');
      setAppMsgEncodingAESKey('');
      setAppMsgDataFormat('json');
      setAppMsgEncryptMode('plain');
    }
  }, [appletData, editingId, groups]);

  const handleSave = async () => {
    if (isBlank(appName) || isBlank(appId)) {
      toast.error(t('errors.fillRequired'));
      return;
    }

    const isValidUrlOrDomain = (d: string) => {
      if (isBlank(d)) return true;
      try {
        if (d.trim().includes('://')) {
          new URL(d.trim());
          return true;
        }
        const domainPattern = /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?(?:[\/?#]\S*)?$/;
        return domainPattern.test(d.trim());
      } catch {
        return false;
      }
    };

    const validateDomains = (domains: string[]) => domains.map(d => d.trim()).filter(Boolean).every(isValidUrlOrDomain);
    
    if (!validateDomains(appRequestDomain) || !validateDomains(appSocketDomain) || 
        !validateDomains(appUploadDomain) || !validateDomains(appDownloadDomain) || 
        !validateDomains(appUdpDomain) || !validateDomains(appTcpDomain) || 
        !validateDomains(appBusinessDomain)) {
      toast.error(t('errors.invalidDomains'));
      return;
    }
    
    const newApplet: WechatAppletConfig = {
      id: editingId === 'new' ? `applet-${Date.now()}` : (editingId || ''),
      name: appName.trim(),
      avatar: appAvatar,
      appId: appId.trim(),
      originalId: appOriginalId.trim(),
      appSecret: appSecret.trim(),
      path: appPath.trim(),
      group: appGroup,
      description: appDescription.trim(),
      requestDomain: appRequestDomain.map(d => d.trim()).filter(Boolean),
      socketDomain: appSocketDomain.map(d => d.trim()).filter(Boolean),
      uploadDomain: appUploadDomain.map(d => d.trim()).filter(Boolean),
      downloadDomain: appDownloadDomain.map(d => d.trim()).filter(Boolean),
      udpDomain: appUdpDomain.map(d => d.trim()).filter(Boolean),
      tcpDomain: appTcpDomain.map(d => d.trim()).filter(Boolean),
      businessDomain: appBusinessDomain.map(d => d.trim()).filter(Boolean),
      domainVerifyFileName: appDomainVerifyFileName.trim(),
      domainVerifyFileContent: appDomainVerifyFileContent.trim(),
      msgToken: appMsgToken.trim(),
      msgEncodingAESKey: appMsgEncodingAESKey.trim(),
      msgDataFormat: appMsgDataFormat,
      msgEncryptMode: appMsgEncryptMode
    };

    await onSave(newApplet);
  };

  const handleDomainVerifyFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const input = e.currentTarget;
    const file = input.files?.[0];
    if (file) {
      if (!file.name.toLowerCase().endsWith('.txt')) {
        toast.error(t('errors.invalidTxt'));
        input.value = '';
        return;
      }
      if (!isFileWithinSizeLimit(file, MAX_DOMAIN_VERIFICATION_TEXT_BYTES)) {
        toast.error(t('errors.verificationFileTooLarge', {
          maxSize: formatFileSizeLimit(MAX_DOMAIN_VERIFICATION_TEXT_BYTES),
        }));
        input.value = '';
        return;
      }
      const reader = new FileReader();
      reader.onload = (evt) => {
        const result = evt.target?.result;
        if (typeof result === 'string') {
          setAppDomainVerifyFileContent(result);
          setAppDomainVerifyFileName(file.name);
          toast.success(t('errors.uploadSuccess'));
        } else {
          toast.error(t('errors.verificationFileReadFailed'));
        }
        input.value = '';
      };
      reader.onerror = () => {
        toast.error(t('errors.verificationFileReadFailed'));
        input.value = '';
      };
      reader.readAsText(file);
    }
  };

  return (
    <>
      {/* Editing Drawer Backdrop */}
      <div 
        className={`fixed inset-0 z-[610] bg-black/40 backdrop-blur-sm transition-opacity duration-300 ${editingId ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'}`}
        onClick={onClose}
      />

      {/* Drawer Container */}
      <div 
        className={`fixed top-0 bottom-0 left-0 w-full max-w-[640px] bg-[var(--color-kb-panel)] shadow-[10px_0_30px_rgba(0,0,0,0.15)] border-r border-[var(--color-kb-panel-border)] z-[620] flex flex-col transition-transform duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] ${
          editingId ? 'translate-x-0' : '-translate-x-full'
        }`}
      >
        <div className="px-6 py-5 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
             <div className="w-10 h-10 rounded-xl bg-[#07c160] flex items-center justify-center text-white shadow-sm shadow-[#07c160]/20">
              <Smartphone size={20} />
            </div>
            <div>
              <h2 className="text-sm font-bold text-[var(--color-kb-text-heading)]">
                {editingId === 'new' ? t('newAppletStr') : t('editConfig', { name: appName })}
              </h2>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="w-8 h-8 flex items-center justify-center hover:bg-[var(--color-kb-panel-hover)] rounded-lg text-[var(--color-kb-text-muted)] transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        <div className="flex px-6 gap-6 border-b border-[var(--color-kb-panel-border)] shrink-0 pt-2 bg-[var(--color-kb-editor)]">
          {[
            { id: 'basic', label: t('basicInfo'), icon: <Edit3 size={14}/> },
            { id: 'domains', label: t('serverAndDomains'), icon: <Globe size={14}/> },
            { id: 'server', label: t('messagePush'), icon: <Server size={14}/> }
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`flex items-center gap-2 px-1 py-3 text-[13px] font-bold border-b-[3px] transition-all whitespace-nowrap ${
                activeTab === tab.id 
                  ? 'border-[#07c160] text-[#07c160]' 
                  : 'border-transparent text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)]'
              }`}
            >
              {tab.icon} {tab.label}
            </button>
          ))}
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-6 custom-scrollbar">
          <div className="space-y-5">
            {activeTab === 'basic' && (
              <div className="space-y-5 animate-in fade-in duration-300">
                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('appNameLabel')} <span className="text-red-500">*</span></label>
                  <input 
                    type="text" value={appName} onChange={(e) => setAppName(e.target.value)}
                    placeholder={t('appNamePlaceholder')}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                  />
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('appIdLabel')} <span className="text-red-500">*</span></label>
                  <input 
                    type="text" value={appId} onChange={(e) => setAppId(e.target.value)}
                    placeholder={t('appIdPlaceholder')}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                  />
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('appSecretLabel')}</label>
                  <input 
                    type="password" value={appSecret} onChange={(e) => setAppSecret(e.target.value)}
                    placeholder={t('appSecretPlaceholder')}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                  />
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('originalIdLabel')}</label>
                  <input 
                    type="text" value={appOriginalId} onChange={(e) => setAppOriginalId(e.target.value)}
                    placeholder={t('originalIdPlaceholder')}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                  />
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('avatarLabel')}</label>
                  <div className="flex flex-col gap-3 p-3 bg-[var(--color-kb-editor)] rounded-lg border border-[var(--color-kb-panel-border)]">
                    <div className="grid grid-cols-7 sm:grid-cols-9 gap-2">
                      {['📱', '🛒', '🎮', '🛠️', '🛍️', '📚', '⚡', '✨', '🎁'].map((emoji) => (
                        <button
                          key={emoji}
                          type="button"
                          onClick={() => setAppAvatar(emoji)}
                          className={`w-10 h-10 flex items-center justify-center rounded-md text-xl transition-all ${
                            appAvatar === emoji && !appAvatar.startsWith('data:image/') 
                              ? 'bg-[#07c160]/10 border-2 border-[#07c160]/30 shadow-sm' 
                              : 'hover:bg-[var(--color-kb-panel)] border-2 border-transparent'
                          }`}
                        >
                          {emoji}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                    <span>{t('defaultPagePath')}</span>
                  </label>
                  <input 
                    type="text" value={appPath} onChange={(e) => setAppPath(e.target.value)}
                    placeholder={t('defaultPagePathPlaceholder')}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                  />
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('defaultGroupLabel')}</label>
                  <select 
                    value={appGroup} 
                    onChange={(e) => setAppGroup(e.target.value)}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-[13px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] transition-all shadow-sm h-[36px]"
                  >
                     {groups.length === 0 && <option value="未分组">{t('unassignedGroup')}</option>}
                     {groups.map(g => <option key={g} value={g}>{g}</option>)}
                  </select>
                </div>

                <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('descriptionLabel')}</label>
                  <textarea 
                    value={appDescription} onChange={(e) => setAppDescription(e.target.value)}
                    placeholder={t('descriptionPlaceholder')}
                    className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-[13px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm resize-none h-24"
                  />
                </div>
              </div>
            )}

            {activeTab === 'domains' && (
              <div className="space-y-6 animate-in fade-in duration-300">
                <div className="bg-[#07c160]/5 border border-[#07c160]/20 p-3 rounded-lg flex gap-2.5 text-[#07c160] text-[12px] font-medium mb-2">
                  <Globe className="shrink-0 mt-0.5" size={14} />
                  <div className="leading-snug">{t('domainHint')}</div>
                </div>
                
                {[
                  { label: t('requestDomainLabel'), state: appRequestDomain, setter: setAppRequestDomain, placeholder: 'https://example.com' },
                  { label: t('socketDomainLabel'), state: appSocketDomain, setter: setAppSocketDomain, placeholder: 'wss://example.com' },
                  { label: t('uploadDomainLabel'), state: appUploadDomain, setter: setAppUploadDomain, placeholder: 'https://example.com' },
                  { label: t('downloadDomainLabel'), state: appDownloadDomain, setter: setAppDownloadDomain, placeholder: 'https://example.com' },
                  { label: t('udpDomainLabel'), state: appUdpDomain, setter: setAppUdpDomain, placeholder: 'udp://example.com' },
                  { label: t('tcpDomainLabel'), state: appTcpDomain, setter: setAppTcpDomain, placeholder: 'tcp://example.com' }
                ].map(({ label, state, setter, placeholder }, idx) => (
                  <div key={idx}>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                      <span>{label}</span>
                      {(state.length === 0 || state[state.length - 1] !== '') && (
                        <button type="button" onClick={() => setter([...state, ''])} className="text-[#07c160] flex items-center gap-1 hover:underline text-[11px]">
                          <Plus size={12}/> {t('add')}
                        </button>
                      )}
                    </label>
                    <div className="space-y-2">
                      {state.map((domain, index) => {
                        const isValidPattern = /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?(?:[\/?#]\S*)?$/;
                        const isUrl = domain.includes('://') && (() => { try { new URL(domain); return true; } catch { return false; }})();
                        const isValid = isBlank(domain) || isUrl || isValidPattern.test(domain.trim());
                        return (
                        <div key={index} className="flex flex-col gap-1">
                          <div className="flex items-center gap-2">
                            <input 
                              type="text" value={domain} onChange={(e) => {
                                const newDomains = [...state];
                                newDomains[index] = e.target.value;
                                setter(newDomains);
                              }}
                              placeholder={placeholder}
                              className={`flex-1 bg-[var(--color-kb-editor)] border ${!isValid && domain ? 'border-red-500 focus:border-red-500 focus:ring-red-500/20' : 'border-[var(--color-kb-panel-border)] focus:border-[#07c160] focus:ring-[#07c160]'} rounded-lg px-3 py-2 text-[12px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:outline-none focus:ring-1 transition-all shadow-sm`}
                            />
                            {state.length > 1 && (
                              <button type="button" onClick={() => setter(state.filter((_, i) => i !== index))} className="text-[var(--color-kb-text-muted)] hover:text-red-500 p-1.5 transition-colors">
                                <Trash2 size={14}/>
                              </button>
                            )}
                          </div>
                          {!isValid && domain && <span className="text-[10px] text-red-500 pl-1">{t('invalidDomainFormat')}</span>}
                        </div>
                      )})}
                      {state.length === 0 && (
                        <button 
                          type="button" 
                          onClick={() => setter([''])}
                          className="w-full border border-dashed border-[var(--color-kb-panel-border)] hover:border-[#07c160] text-[var(--color-kb-text-muted)] hover:text-[#07c160] py-2 rounded-lg text-[12px] transition-colors flex items-center justify-center gap-1 bg-[var(--color-kb-panel)] cursor-pointer"
                        >
                          <Plus size={14}/> {t('addDomain')}
                        </button>
                      )}
                    </div>
                  </div>
                ))}

                <div className="pt-4 border-t border-[var(--color-kb-panel-border)]">
                  <h4 className="flex items-center justify-between text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-3">
                    <span className="flex items-center gap-1.5"><Globe size={14} className="text-[#07c160]" /> {t('businessDomainLabel')}</span>
                    {(appBusinessDomain.length === 0 || appBusinessDomain[appBusinessDomain.length - 1] !== '') && (
                      <button type="button" onClick={() => setAppBusinessDomain([...appBusinessDomain, ''])} className="text-[#07c160] flex items-center gap-1 hover:underline text-[11px]">
                        <Plus size={12}/> {t('add')}
                      </button>
                    )}
                  </h4>
                  
                  <div className="mb-4 space-y-2">
                     {appBusinessDomain.map((domain, index) => {
                        const isValidPattern = /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?(?:[\/?#]\S*)?$/;
                        const isUrl = domain.includes('://') && (() => { try { new URL(domain); return true; } catch { return false; }})();
                        const isValid = isBlank(domain) || isUrl || isValidPattern.test(domain.trim());
                       return (
                        <div key={index} className="flex flex-col gap-1">
                          <div className="flex items-center gap-2">
                            <input 
                              type="text" value={domain} onChange={(e) => {
                                const newDomains = [...appBusinessDomain];
                                newDomains[index] = e.target.value;
                                setAppBusinessDomain(newDomains);
                              }}
                              placeholder={t('businessDomainPlaceholder')}
                              className={`flex-1 bg-[var(--color-kb-editor)] border ${!isValid && domain ? 'border-red-500 focus:border-red-500 focus:ring-red-500/20' : 'border-[var(--color-kb-panel-border)] focus:border-[#07c160] focus:ring-[#07c160]'} rounded-lg px-3 py-2 text-[12px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:outline-none focus:ring-1 transition-all shadow-sm`}
                            />
                            {appBusinessDomain.length > 1 && (
                              <button type="button" onClick={() => setAppBusinessDomain(appBusinessDomain.filter((_, i) => i !== index))} className="text-[var(--color-kb-text-muted)] hover:text-red-500 p-1.5 transition-colors">
                                <Trash2 size={14}/>
                              </button>
                            )}
                          </div>
                          {!isValid && domain && <span className="text-[10px] text-red-500 pl-1">{t('invalidDomainFormat')}</span>}
                        </div>
                      )})}
                      {appBusinessDomain.length === 0 && (
                        <button 
                          type="button" 
                          onClick={() => setAppBusinessDomain([''])}
                          className="w-full border border-dashed border-[var(--color-kb-panel-border)] hover:border-[#07c160] text-[var(--color-kb-text-muted)] hover:text-[#07c160] py-2 rounded-lg text-[12px] transition-colors flex items-center justify-center gap-1 bg-[var(--color-kb-panel)] cursor-pointer"
                        >
                          <Plus size={14}/> {t('addDomain')}
                        </button>
                      )}
                  </div>
                  
                  <div className="bg-[var(--color-kb-editor)] p-4 rounded-xl border border-[var(--color-kb-panel-border)]">
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                      <span>{t('domainVerifyFileLabel')}</span>
                    </label>
                    <div className="text-[11px] text-[var(--color-kb-text-muted)] mb-3 leading-relaxed">
                      {t('domainVerifyFileHint')}
                    </div>
                    
                    <input 
                      type="file" 
                      accept=".txt"
                      ref={domainVerifyInputRef}
                      className="hidden"
                      onChange={handleDomainVerifyFileUpload}
                    />
                    
                    {appDomainVerifyFileName ? (
                        <div className="flex items-center justify-between bg-white dark:bg-black/20 border border-[var(--color-kb-panel-border)] rounded-md px-3 py-2">
                          <div className="flex items-center gap-2 overflow-hidden">
                             <div className="w-5 h-5 rounded bg-blue-100 dark:bg-blue-900/40 text-blue-500 flex items-center justify-center font-bold text-[10px] shrink-0">TXT</div>
                             <span className="text-[12px] font-medium text-[var(--color-kb-text-heading)] truncate">{appDomainVerifyFileName}</span>
                          </div>
                          <button 
                            type="button"
                            onClick={() => { setAppDomainVerifyFileName(''); setAppDomainVerifyFileContent(''); }}
                            className="text-[var(--color-kb-text-muted)] hover:text-red-500 transition-colors shrink-0 ml-2 p-1"
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                    ) : (
                      <button
                        type="button"
                        onClick={() => domainVerifyInputRef.current?.click()}
                        className="w-full border border-dashed border-[var(--color-kb-panel-border)] hover:border-[#07c160] hover:bg-[#07c160]/5 rounded-lg py-3 flex flex-col items-center justify-center gap-1 transition-all"
                      >
                        <span className="text-[12px] font-bold text-[var(--color-kb-text-muted)] group-hover:text-[#07c160]">
                           {t('uploadVerifyFile')}
                        </span>
                        <span className="text-[10px] text-[var(--color-kb-text-muted)] opacity-70">
                          {t('downloadVerifyFileHint')}
                        </span>
                      </button>
                    )}
                  </div>
                </div>
              </div>
            )}

            {activeTab === 'server' && (
              <div className="space-y-5 animate-in fade-in duration-300">
                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex justify-between">
                       <span>{t('tokenLabel')}</span>
                    </label>
                    <input 
                      type="text" value={appMsgToken} onChange={(e) => setAppMsgToken(e.target.value.replace(/[^a-zA-Z0-9]/g, ''))}
                      placeholder={t('tokenPlaceholder')}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
                 </div>
                 
                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex justify-between">
                       <span>{t('encodingAESKeyLabel')}</span>
                       <button 
                         type="button"
                         onClick={() => {
                           const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
                           let key = '';
                           for (let i = 0; i < 43; i++) key += chars.charAt(Math.floor(Math.random() * chars.length));
                           setAppMsgEncodingAESKey(key);
                         }}
                         className="text-[#07c160] hover:underline text-[11px]"
                       >
                         {t('generateRandom')}
                       </button>
                    </label>
                    <input 
                      type="text" value={appMsgEncodingAESKey} onChange={(e) => setAppMsgEncodingAESKey(e.target.value.replace(/[^a-zA-Z0-9]/g, '').slice(0,43))}
                      placeholder={t('encodingAESKeyPlaceholder')}
                      maxLength={43}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
                 </div>
                 
                 <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('msgDataFormatLabel')}</label>
                  <div className="flex gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input type="radio" value="json" checked={appMsgDataFormat === 'json'} onChange={() => setAppMsgDataFormat('json')} className="accent-[#07c160]" />
                      <span className="text-[13px] text-[var(--color-kb-text-heading)]">JSON</span>
                    </label>
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input type="radio" value="xml" checked={appMsgDataFormat === 'xml'} onChange={() => setAppMsgDataFormat('xml')} className="accent-[#07c160]" />
                      <span className="text-[13px] text-[var(--color-kb-text-heading)]">XML</span>
                    </label>
                  </div>
                 </div>
                 
                 <div>
                  <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('msgEncryptModeLabel')}</label>
                  <div className="space-y-3">
                    {[
                      { val: 'plain', label: t('plainMode') },
                      { val: 'compatible', label: t('compatibleMode') },
                      { val: 'safe', label: t('safeMode') }
                    ].map(mode => (
                      <label key={mode.val} className="flex items-center gap-2 cursor-pointer">
                        <input type="radio" value={mode.val} checked={appMsgEncryptMode === mode.val} onChange={() => setAppMsgEncryptMode(mode.val as any)} className="accent-[#07c160]" />
                        <span className="text-[13px] text-[var(--color-kb-text-heading)]">{mode.label}</span>
                      </label>
                    ))}
                  </div>
                 </div>
              </div>
            )}
          </div>
        </div>

        {saveError && (
          <div role="alert" className="px-5 py-2 text-xs text-red-500 bg-red-500/5 border-t border-red-500/20">
            {saveError}
          </div>
        )}

        <div className="p-5 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] flex justify-between shrink-0">
          <button 
           onClick={onClose} 
           className="px-5 py-2.5 text-[13px] font-medium bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-heading)] rounded-lg transition-colors"
          >
           {t('cancel')}
          </button>
          <button 
           onClick={() => void handleSave()}
           disabled={isSaving}
           className="px-6 py-2.5 text-[13px] font-bold bg-[#07c160] hover:bg-[#06ad56] text-white rounded-lg transition-all shadow-sm disabled:cursor-not-allowed disabled:opacity-50"
          >
           {isSaving ? t('saving', { defaultValue: 'Saving...' }) : t('saveConfig')}
          </button>
        </div>
      </div>
    </>
  );
}
