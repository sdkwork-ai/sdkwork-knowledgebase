import React, { useState, useEffect, useRef } from 'react';
import { isBlank } from '@sdkwork/utils';
import { X, Settings2, Edit3, Key, Globe, Server, Plus, Trash2, Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { OfficialAccount } from '../../services/wechat';
import {
  formatFileSizeLimit,
  isFileWithinSizeLimit,
  MAX_DOMAIN_VERIFICATION_TEXT_BYTES,
} from '../../services/fileInputLimits';
import { toast } from '../ui/toast-manager';

interface Props {
  oaEditingId: string | null;
  officialAccountData: OfficialAccount | undefined;
  groups: string[];
  onClose: () => void;
  onSave: (applet: OfficialAccount) => void;
}

export function OAEditorDrawer({ oaEditingId, officialAccountData, groups, onClose, onSave }: Props) {
  const { t } = useTranslation('officialAccount');
  const [activeTab, setActiveTab] = useState<'basic' | 'developer' | 'domains' | 'server'>('basic');

  const [oaName, setOaName] = useState('');
  const [oaDescription, setOaDescription] = useState('');
  const [oaType, setOaType] = useState<'subscription' | 'service'>('subscription');
  const [oaAvatar, setOaAvatar] = useState('🤖');
  const [oaAppId, setOaAppId] = useState('');
  const [oaAppSecret, setOaAppSecret] = useState('');
  const [oaServerUrl, setOaServerUrl] = useState('');
  const [oaToken, setOaToken] = useState('');
  const [oaEncodingAesKey, setOaEncodingAesKey] = useState('');
  const [oaEncryptMode, setOaEncryptMode] = useState<'plain' | 'compatible' | 'safe'>('safe');
  const [oaJsSecureDomains, setOaJsSecureDomains] = useState<string[]>(['']);
  const [oaWebAuthDomains, setOaWebAuthDomains] = useState<string[]>(['']);
  const [oaBusinessDomains, setOaBusinessDomains] = useState<string[]>(['']);
  const [oaDomainVerifyFileName, setOaDomainVerifyFileName] = useState('');
  const [oaDomainVerifyFileContent, setOaDomainVerifyFileContent] = useState('');
  const [oaGroup, setOaGroup] = useState<string>(groups.length > 0 ? groups[0] : t('unassignedGroup'));

  const domainVerifyInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setActiveTab('basic');
    if (officialAccountData) {
      setOaName(officialAccountData.name);
      setOaDescription(officialAccountData.description || '');
      setOaType(officialAccountData.type);
      setOaAvatar(officialAccountData.avatar);
      setOaAppId(officialAccountData.appId);
      setOaAppSecret(officialAccountData.appSecret || '');
      setOaServerUrl(officialAccountData.serverUrl || '');
      setOaToken(officialAccountData.token || '');
      setOaEncodingAesKey(officialAccountData.encodingAesKey || '');
      setOaEncryptMode(officialAccountData.encryptMode || 'safe');
      setOaJsSecureDomains(officialAccountData.jsSecureDomains && officialAccountData.jsSecureDomains.length > 0 ? officialAccountData.jsSecureDomains : ['']);
      setOaWebAuthDomains(officialAccountData.webAuthDomains && officialAccountData.webAuthDomains.length > 0 ? officialAccountData.webAuthDomains : ['']);
      setOaBusinessDomains(officialAccountData.businessDomains && officialAccountData.businessDomains.length > 0 ? officialAccountData.businessDomains : ['']);
      setOaDomainVerifyFileName(officialAccountData.domainVerifyFileName || '');
      setOaDomainVerifyFileContent(officialAccountData.domainVerifyFileContent || '');
      setOaGroup(officialAccountData.group || t('unassignedGroup'));
    } else {
      setOaName('');
      setOaDescription('');
      setOaType('subscription');
      setOaAvatar('🤖');
      setOaAppId('');
      setOaAppSecret('');
      setOaServerUrl('');
      setOaToken('');
      setOaEncodingAesKey('');
      setOaEncryptMode('safe');
      setOaJsSecureDomains(['']);
      setOaWebAuthDomains(['']);
      setOaBusinessDomains(['']);
      setOaDomainVerifyFileName('');
      setOaDomainVerifyFileContent('');
      setOaGroup(groups.length > 0 ? groups[0] : t('unassignedGroup'));
    }
  }, [oaEditingId, officialAccountData, groups]);

  const handleSave = () => {
    if (isBlank(oaName) || isBlank(oaAppId)) {
      toast.error(t('errors.fillRequired'));
      return;
    }

    const domainRegex = /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?$/;
    
    const isValidDomain = (d: string) => !d || domainRegex.test(d);
    
    const jsDomains = oaJsSecureDomains.map(d => d.trim()).filter(Boolean);
    const webDomains = oaWebAuthDomains.map(d => d.trim()).filter(Boolean);
    const bizDomains = oaBusinessDomains.map(d => d.trim()).filter(Boolean);

    if (!jsDomains.every(isValidDomain) || !webDomains.every(isValidDomain) || !bizDomains.every(isValidDomain)) {
      toast.error(t('errors.invalidDomains'));
      return;
    }

    const newApplet: OfficialAccount = {
      id: oaEditingId === 'new' ? `oa-${Date.now()}` : (oaEditingId || ''),
      name: oaName.trim(),
      description: oaDescription.trim(),
      type: oaType,
      avatar: oaAvatar,
      appId: oaAppId.trim(),
      appSecret: oaAppSecret.trim(),
      serverUrl: oaServerUrl.trim(),
      token: oaToken.trim(),
      encodingAesKey: oaEncodingAesKey.trim(),
      encryptMode: oaEncryptMode,
      jsSecureDomains: jsDomains,
      webAuthDomains: webDomains,
      businessDomains: bizDomains,
      domainVerifyFileName: oaDomainVerifyFileName.trim(),
      domainVerifyFileContent: oaDomainVerifyFileContent.trim(),
      group: oaGroup
    };

    onSave(newApplet);
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
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
          setOaDomainVerifyFileContent(result);
          setOaDomainVerifyFileName(file.name);
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
        className={`fixed inset-0 z-[610] bg-black/40 backdrop-blur-sm transition-opacity duration-300 ${oaEditingId ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'}`}
        onClick={onClose}
      />

      {/* Drawer Container */}
      <div 
        className={`fixed top-0 bottom-0 left-0 w-full max-w-[640px] bg-[var(--color-kb-panel)] shadow-[10px_0_30px_rgba(0,0,0,0.15)] border-r border-[var(--color-kb-panel-border)] z-[620] flex flex-col transition-transform duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] ${
          oaEditingId ? 'translate-x-0' : '-translate-x-full'
        }`}
      >
        <div className="px-6 py-5 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3">
             <div className="w-10 h-10 rounded-xl bg-[#07c160] flex items-center justify-center text-white shadow-sm shadow-[#07c160]/20">
              <Settings2 size={20} />
            </div>
            <div>
              <h2 className="text-sm font-bold text-[var(--color-kb-text-heading)]">
                {oaEditingId === 'new' ? t('newConfigTitle') : t('editConfigTitle', { name: oaName })}
              </h2>
              <p className="text-[11px] text-[var(--color-kb-text-muted)] font-medium mt-0.5">{t('configSubtitle')}</p>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="p-2 text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] hover:bg-[var(--color-kb-panel-hover)] rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Edit Tabs */}
        <div className="flex px-6 gap-6 border-b border-[var(--color-kb-panel-border)] shrink-0 pt-2 bg-[var(--color-kb-editor)]">
          {[
            { id: 'basic', label: t('basicInfo'), icon: <Edit3 size={14}/> },
            { id: 'developer', label: t('developerConfig'), icon: <Key size={14}/> },
            { id: 'domains', label: t('domainSettings'), icon: <Globe size={14}/> },
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

        <div className="flex-1 overflow-y-auto px-8 py-6 custom-scrollbar">
          <div className="space-y-6 pb-12">

            {activeTab === 'basic' && (
              <div className="space-y-6 animate-in fade-in duration-300">
                 
                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('oaNameLabel')} <span className="text-red-500">*</span></label>
                    <input 
                      type="text" value={oaName} onChange={(e) => setOaName(e.target.value)}
                      placeholder={t('oaNamePlaceholder')}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('descriptionLabel')}</label>
                    <textarea 
                      value={oaDescription} onChange={(e) => setOaDescription(e.target.value)}
                      placeholder={t('descriptionPlaceholder')}
                      rows={2}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm resize-none"
                    />
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('accountTypeLabel')}</label>
                    <div className="flex bg-[var(--color-kb-editor)] p-1 rounded-lg border border-[var(--color-kb-panel-border)]">
                      <button
                        type="button"
                        onClick={() => setOaType('subscription')}
                        className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-[12px] font-bold rounded-md transition-all ${oaType === 'subscription' ? 'bg-[var(--color-kb-panel)] shadow-sm text-[#07c160] border border-[#07c160]/30' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] border border-transparent'}`}
                      >
                        {oaType === 'subscription' && <Check size={14} className="text-[#07c160]" />}
                        {t('subscriptionAccount')}
                      </button>
                      <button
                        type="button"
                        onClick={() => setOaType('service')}
                        className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-[12px] font-bold rounded-md transition-all ${oaType === 'service' ? 'bg-[var(--color-kb-panel)] shadow-sm text-[#07c160] border border-[#07c160]/30' : 'text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] border border-transparent'}`}
                      >
                        {oaType === 'service' && <Check size={14} className="text-[#07c160]" />}
                        {t('serviceAccount')}
                      </button>
                    </div>
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('defaultGroupLabel')}</label>
                    <select 
                      value={oaGroup} 
                      onChange={(e) => setOaGroup(e.target.value)}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-3 py-2 text-[13px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] transition-all shadow-sm h-[36px]"
                    >
                       {groups.length === 0 && <option value="未分组">{t('unassignedGroup')}</option>}
                       {groups.map(g => <option key={g} value={g}>{g}</option>)}
                    </select>
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('avatarLabel')}</label>
                    <div className="flex flex-col gap-3 p-3 bg-[var(--color-kb-editor)] rounded-lg border border-[var(--color-kb-panel-border)]">
                      <div className="grid grid-cols-7 sm:grid-cols-9 gap-2">
                        {['🤖', '💬', '📢', '📰', '🌐', '💡', '🔥', '✨', '⚡'].map((emoji) => (
                          <button
                            key={emoji}
                            type="button"
                            onClick={() => setOaAvatar(emoji)}
                            className={`w-10 h-10 flex items-center justify-center rounded-md text-xl transition-all ${
                              oaAvatar === emoji && !oaAvatar.startsWith('data:image/') 
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
              </div>
            )}

            {activeTab === 'developer' && (
              <div className="space-y-6 animate-in fade-in duration-300">
                 <div className="bg-[#07c160]/5 border border-[#07c160]/20 p-3 rounded-lg flex gap-2.5 text-[#07c160] text-[12px] font-medium">
                    <Key className="shrink-0 mt-0.5" size={14} />
                    <div className="leading-snug">{t('developerHint')}</div>
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('appIdLabel')}</label>
                    <input 
                      type="text" value={oaAppId} onChange={(e) => setOaAppId(e.target.value)}
                      placeholder={t('appIdPlaceholder')}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('appSecretLabel')}</label>
                    <input 
                      type="password" value={oaAppSecret} onChange={(e) => setOaAppSecret(e.target.value)}
                      placeholder={t('appSecretPlaceholder')}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2.5 text-[13px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
                 </div>
              </div>
            )}

            {activeTab === 'domains' && (
              <div className="space-y-6 animate-in fade-in duration-300">
                 <div className="bg-[#07c160]/5 border border-[#07c160]/20 p-3 rounded-lg flex gap-2.5 text-[#07c160] text-[12px] font-medium">
                    <Globe className="shrink-0 mt-0.5" size={14} />
                    <div className="leading-snug">{t('domainHint')}</div>
                 </div>

                 {/* JS接口安全域名 */}
                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                      <span>{t('jsSecureDomainLabel')} <span className="text-[10px] font-normal text-[var(--color-kb-text-muted)] ml-1">{t('max5')}</span></span>
                      {oaJsSecureDomains.length < 5 && (
                        <button type="button" onClick={() => setOaJsSecureDomains([...oaJsSecureDomains, ''])} className="text-[#07c160] flex items-center gap-1 hover:underline text-[11px]">
                          <Plus size={12}/> {t('add')}
                        </button>
                      )}
                    </label>
                    <div className="space-y-2">
                      {oaJsSecureDomains.map((domain, index) => {
                        const isValid = isBlank(domain) || /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?$/.test(domain.trim());
                        return (
                        <div key={index} className="flex flex-col gap-1">
                          <div className="flex items-center gap-2">
                            <input 
                              type="text" value={domain} onChange={(e) => {
                                const newDomains = [...oaJsSecureDomains];
                                newDomains[index] = e.target.value;
                                setOaJsSecureDomains(newDomains);
                              }}
                              placeholder={t('domainPlaceholder')}
                              className={`flex-1 bg-[var(--color-kb-editor)] border ${!isValid && domain ? 'border-red-500 focus:border-red-500 focus:ring-red-500/20' : 'border-[var(--color-kb-panel-border)] focus:border-[#07c160] focus:ring-[#07c160]'} rounded-lg px-3 py-2 text-[12px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:outline-none focus:ring-1 transition-all shadow-sm`}
                            />
                            {oaJsSecureDomains.length > 1 && (
                              <button type="button" onClick={() => setOaJsSecureDomains(oaJsSecureDomains.filter((_, i) => i !== index))} className="text-[var(--color-kb-text-muted)] hover:text-red-500 p-1.5 transition-colors">
                                <Trash2 size={14}/>
                              </button>
                            )}
                          </div>
                          {!isValid && domain && <span className="text-[10px] text-red-500 pl-1">{t('errors.domainFormatError')}</span>}
                        </div>
                      )})}
                    </div>
                 </div>

                 {/* 网页授权域名 */}
                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                      <span>{t('webAuthDomainLabel')} <span className="text-[10px] font-normal text-[var(--color-kb-text-muted)] ml-1">{t('max2')}</span></span>
                      {oaWebAuthDomains.length < 2 && (
                        <button type="button" onClick={() => setOaWebAuthDomains([...oaWebAuthDomains, ''])} className="text-[#07c160] flex items-center gap-1 hover:underline text-[11px]">
                          <Plus size={12}/> {t('add')}
                        </button>
                      )}
                    </label>
                    <div className="space-y-2">
                      {oaWebAuthDomains.map((domain, index) => {
                        const isValid = isBlank(domain) || /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?$/.test(domain.trim());
                        return (
                        <div key={index} className="flex flex-col gap-1">
                          <div className="flex items-center gap-2">
                            <input 
                              type="text" value={domain} onChange={(e) => {
                                const newDomains = [...oaWebAuthDomains];
                                newDomains[index] = e.target.value;
                                setOaWebAuthDomains(newDomains);
                              }}
                              placeholder={t('domainPlaceholder')}
                              className={`flex-1 bg-[var(--color-kb-editor)] border ${!isValid && domain ? 'border-red-500 focus:border-red-500 focus:ring-red-500/20' : 'border-[var(--color-kb-panel-border)] focus:border-[#07c160] focus:ring-[#07c160]'} rounded-lg px-3 py-2 text-[12px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:outline-none focus:ring-1 transition-all shadow-sm`}
                            />
                            {oaWebAuthDomains.length > 1 && (
                              <button type="button" onClick={() => setOaWebAuthDomains(oaWebAuthDomains.filter((_, i) => i !== index))} className="text-[var(--color-kb-text-muted)] hover:text-red-500 p-1.5 transition-colors">
                                <Trash2 size={14}/>
                              </button>
                            )}
                          </div>
                          {!isValid && domain && <span className="text-[10px] text-red-500 pl-1">{t('errors.domainFormatError')}</span>}
                        </div>
                      )})}
                    </div>
                 </div>

                 {/* 业务域名 */}
                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                      <span>{t('businessDomainLabel')} <span className="text-[10px] font-normal text-[var(--color-kb-text-muted)] ml-1">{t('max3')}</span></span>
                      {oaBusinessDomains.length < 3 && (
                        <button type="button" onClick={() => setOaBusinessDomains([...oaBusinessDomains, ''])} className="text-[#07c160] flex items-center gap-1 hover:underline text-[11px]">
                          <Plus size={12}/> {t('add')}
                        </button>
                      )}
                    </label>
                    <div className="space-y-2">
                      {oaBusinessDomains.map((domain, index) => {
                        const isValid = isBlank(domain) || /^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?::\d+)?$/.test(domain.trim());
                        return (
                        <div key={index} className="flex flex-col gap-1">
                          <div className="flex items-center gap-2">
                            <input 
                              type="text" value={domain} onChange={(e) => {
                                const newDomains = [...oaBusinessDomains];
                                newDomains[index] = e.target.value;
                                setOaBusinessDomains(newDomains);
                              }}
                              placeholder={t('domainPlaceholder')}
                              className={`flex-1 bg-[var(--color-kb-editor)] border ${!isValid && domain ? 'border-red-500 focus:border-red-500 focus:ring-red-500/20' : 'border-[var(--color-kb-panel-border)] focus:border-[#07c160] focus:ring-[#07c160]'} rounded-lg px-3 py-2 text-[12px] font-mono tracking-wide text-[var(--color-kb-text-heading)] focus:outline-none focus:ring-1 transition-all shadow-sm`}
                            />
                            {oaBusinessDomains.length > 1 && (
                              <button type="button" onClick={() => setOaBusinessDomains(oaBusinessDomains.filter((_, i) => i !== index))} className="text-[var(--color-kb-text-muted)] hover:text-red-500 p-1.5 transition-colors">
                                <Trash2 size={14}/>
                              </button>
                            )}
                          </div>
                          {!isValid && domain && <span className="text-[10px] text-red-500 pl-1">{t('errors.domainFormatError')}</span>}
                        </div>
                      )})}
                    </div>
                 </div>

                 <div className="bg-[var(--color-kb-editor)] p-4 rounded-xl border border-[var(--color-kb-panel-border)] mt-4">
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex items-center justify-between">
                      <span>{t('domainVerifyFileLabel')}</span>
                    </label>
                    <div className="text-[11px] text-[var(--color-kb-text-muted)] mb-3 leading-relaxed">
                      {t('downloadVerifyFileHint')}
                    </div>
                    
                    <input 
                      type="file" 
                      accept=".txt"
                      ref={domainVerifyInputRef}
                      className="hidden"
                      onChange={handleFileUpload}
                    />
                    
                    {oaDomainVerifyFileName ? (
                        <div className="flex items-center justify-between bg-white dark:bg-black/20 border border-[var(--color-kb-panel-border)] rounded-md px-3 py-2">
                          <div className="flex items-center gap-2 overflow-hidden">
                             <div className="w-5 h-5 rounded bg-blue-100 dark:bg-blue-900/40 text-blue-500 flex items-center justify-center font-bold text-[10px] shrink-0">TXT</div>
                             <span className="text-[12px] font-medium text-[var(--color-kb-text-heading)] truncate">{oaDomainVerifyFileName}</span>
                          </div>
                          <button 
                            type="button"
                            onClick={() => { setOaDomainVerifyFileName(''); setOaDomainVerifyFileContent(''); }}
                            className="text-[var(--color-kb-text-muted)] hover:text-red-500 transition-colors shrink-0 ml-2 p-1"
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                    ) : (
                      <button
                        type="button"
                        onClick={() => domainVerifyInputRef.current?.click()}
                        className="w-full border border-dashed border-[var(--color-kb-panel-border)] hover:border-[#07c160] hover:bg-[#07c160]/5 rounded-lg py-3 flex flex-col items-center justify-center gap-1 transition-all group"
                      >
                       <span className="text-[12px] font-bold text-[var(--color-kb-text-muted)] group-hover:text-[#07c160]">
                           {t('uploadVerifyFile')}
                        </span>
                      </button>
                    )}
                  </div>
              </div>
            )}

            {activeTab === 'server' && (
              <div className="space-y-5 animate-in fade-in duration-300">
                 <div className="bg-[#07c160]/5 border border-[#07c160]/20 p-3 rounded-lg flex gap-2.5 text-[#07c160] text-[12px] font-medium mb-4">
                    <Server className="shrink-0 mt-0.5" size={14} />
                    <div className="leading-snug">{t('serverHint')}</div>
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2">{t('serverUrlLabel')}</label>
                    <input 
                      type="text" value={oaServerUrl} onChange={(e) => setOaServerUrl(e.target.value)}
                      placeholder={t('serverUrlPlaceholder')}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
                 </div>

                 <div>
                    <label className="block text-[12px] font-bold text-[var(--color-kb-text-heading)] mb-2 flex justify-between">
                       <span>{t('tokenLabel')}</span>
                    </label>
                    <input 
                      type="text" value={oaToken} onChange={(e) => setOaToken(e.target.value.replace(/[^a-zA-Z0-9]/g, ''))}
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
                           setOaEncodingAesKey(key);
                         }}
                         className="text-[#07c160] hover:underline text-[11px]"
                       >
                         {t('generateRandom')}
                       </button>
                    </label>
                    <input 
                      type="text" value={oaEncodingAesKey} onChange={(e) => setOaEncodingAesKey(e.target.value.replace(/[^a-zA-Z0-9]/g, '').slice(0,43))}
                      placeholder={t('encodingAESKeyPlaceholder')}
                      maxLength={43}
                      className="w-full bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] rounded-lg px-4 py-2 text-[13px] font-mono text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[#07c160] focus:ring-1 focus:ring-[#07c160] transition-all shadow-sm"
                    />
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
                        <input type="radio" value={mode.val} checked={oaEncryptMode === mode.val} onChange={() => setOaEncryptMode(mode.val as any)} className="accent-[#07c160]" />
                        <span className="text-[13px] text-[var(--color-kb-text-heading)]">{mode.label}</span>
                      </label>
                    ))}
                  </div>
                 </div>
              </div>
            )}
          </div>
        </div>

        <div className="p-5 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] flex justify-between shrink-0">
          <button 
           onClick={onClose} 
           className="px-5 py-2.5 text-[13px] font-medium bg-[var(--color-kb-editor)] border border-[var(--color-kb-panel-border)] hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-heading)] rounded-lg transition-colors"
          >
           {t('cancel')}
          </button>
          <button 
           onClick={handleSave} 
           className="px-6 py-2.5 text-[13px] font-bold bg-[#07c160] hover:bg-[#06ad56] text-white rounded-lg transition-all shadow-sm"
          >
           {t('saveConfig')}
          </button>
        </div>
      </div>
    </>
  );
}
