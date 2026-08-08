import { useState, useEffect } from 'react';
import { X, HelpCircle, User } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';
import { AppletManagerModal } from './AppletManagerModal';
import { WechatService, WechatAppletConfig } from '../services/wechat';

export interface WechatAppletModalProps {
  onClose: () => void;
  onConfirm: (data: any) => void;
}

export function WechatAppletModal({ onClose, onConfirm }: WechatAppletModalProps) {
  const { t } = useTranslation('applet');
  const [link, setLink] = useState('');
  const [displayType, setDisplayType] = useState<'text' | 'image' | 'card'>('card');
  const [textContent, setTextContent] = useState('');
  const [cardTitle, setCardTitle] = useState('');
  const [imageUrl, setImageUrl] = useState('');
  const [isAppletManagerOpen, setIsAppletManagerOpen] = useState(false);
  const [applets, setApplets] = useState<WechatAppletConfig[]>([]);
  const [appletGroups, setAppletGroups] = useState<string[]>([]);
  const [appletsLoading, setAppletsLoading] = useState(true);
  const [appletError, setAppletError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setAppletsLoading(true);
    setAppletError(null);

    void WechatService.getApplets()
      .then(data => {
        if (!active) return;
        setApplets(data);
        setAppletGroups(
          Array.from(new Set(data.map(item => item.group).filter((group): group is string => Boolean(group)))),
        );
      })
      .catch(error => {
        if (!active) return;
        setAppletError(
          resolveUserFacingErrorMessage(error, t as unknown as ErrorTranslateFn),
        );
      })
      .finally(() => {
        if (active) setAppletsLoading(false);
      });

    return () => {
      active = false;
    };
  }, [t]);

  const handleConfirm = () => {
    onConfirm({ 
      link: link || '', 
      displayType, 
      textContent,
      cardTitle,
      imageUrl: imageUrl || ''
    });
    onClose();
  };

  const handleAppletSelect = (applet: WechatAppletConfig) => {
    setLink(applet.path || applet.appId);
    setCardTitle(applet.name);
    setIsAppletManagerOpen(false);
  };

  const handleSaveApplets = async (newApplets: WechatAppletConfig[], newGroups: string[]) => {
    setAppletError(null);
    try {
      await WechatService.saveApplets(newApplets);
      setApplets(newApplets);
      setAppletGroups(newGroups);
    } catch (error) {
      setAppletError(
        resolveUserFacingErrorMessage(error, t as unknown as ErrorTranslateFn),
      );
      throw error;
    }
  };

  return (
    <div className="fixed inset-0 z-[500] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md">
      <div className="bg-[var(--color-kb-editor)] w-[800px] max-w-[90vw] rounded-2xl shadow-2xl flex flex-col animate-in fade-in zoom-in-95 duration-200 text-[var(--color-kb-text)] border border-[var(--color-kb-panel-border)] overflow-y-auto max-h-[90vh]">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-5 border-b border-[var(--color-kb-panel-border)]">
          <h3 className="text-lg text-[var(--color-kb-text-heading)] font-normal">{t('insertApplet')}</h3>
          <button 
            onClick={onClose} 
            className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] transition-colors p-1"
          >
            <X size={20} className="font-light" />
          </button>
        </div>

        {/* Body */}
        <div className="p-10 flex flex-col gap-8 min-h-[400px]">
          {/* Link Input */}
          <div className="flex items-start">
            <div className="w-[120px] flex-shrink-0 flex items-center pt-2 text-sm text-[var(--color-kb-text-muted)]">
              {t('appletLinkTitle')} 
              <HelpCircle size={14} className="ml-1 opacity-70" />
            </div>
            <div className="flex-1 max-w-[500px]">
              <div className="flex gap-2">
                <input 
                  type="text" 
                  value={link}
                  onChange={e => setLink(e.target.value)}
                  placeholder={t('copyLinkHint')}
                  className="flex-1 w-full border border-[var(--color-kb-panel-border)] rounded bg-transparent px-3 py-2 text-sm text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] transition-colors"
                  style={{ borderColor: link ? 'var(--color-kb-panel-border)' : '' }}
                />
                <button 
                  onClick={() => setIsAppletManagerOpen(true)}
                  disabled={appletsLoading || Boolean(appletError)}
                  className="px-4 py-2 bg-[var(--color-kb-panel)] hover:bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text)] text-sm rounded shadow-sm transition-colors whitespace-nowrap disabled:cursor-not-allowed disabled:opacity-50"
                >
                  {t('selectApplet')}
                </button>
              </div>
              <div className="text-[13px] text-[var(--color-kb-text-muted)] mt-3 leading-relaxed">
                {t('cannotCopyHint')}
              </div>
              {appletsLoading && (
                <p role="status" className="text-[12px] text-[var(--color-kb-text-muted)] mt-2">
                  {t('loadingApplets', { defaultValue: 'Loading applets...' })}
                </p>
              )}
              {appletError && (
                <p role="alert" className="text-[12px] text-red-500 mt-2">
                  {appletError}
                </p>
              )}
            </div>
          </div>

          {/* Display Format */}
          <div className="flex items-start mt-2">
            <div className="w-[120px] flex-shrink-0 flex items-center pt-1 text-sm text-[var(--color-kb-text-muted)]">
              {t('displayFormat')}
              <HelpCircle size={14} className="ml-1 opacity-70" />
            </div>
            <div className="flex-1 max-w-[500px] flex items-center gap-6">
              <label className="flex items-center cursor-pointer group">
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center mr-2 transition-colors ${displayType === 'text' ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]' : 'border-[var(--color-kb-text-muted)] group-hover:border-[var(--color-kb-accent)]'}`}>
                  {displayType === 'text' && <div className="w-1.5 h-1.5 bg-white rounded-full"></div>}
                </div>
                <input type="radio" className="hidden" checked={displayType === 'text'} onChange={() => setDisplayType('text')} />
                <span className="text-sm text-[var(--color-kb-text)]">{t('textType')}</span>
              </label>

              <label className="flex items-center cursor-pointer group">
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center mr-2 transition-colors ${displayType === 'image' ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]' : 'border-[var(--color-kb-text-muted)] group-hover:border-[var(--color-kb-accent)]'}`}>
                  {displayType === 'image' && <div className="w-1.5 h-1.5 bg-white rounded-full"></div>}
                </div>
                <input type="radio" className="hidden" checked={displayType === 'image'} onChange={() => setDisplayType('image')} />
                <span className="text-sm text-[var(--color-kb-text)]">{t('imageType')}</span>
              </label>

              <label className="flex items-center cursor-pointer group">
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center mr-2 transition-colors ${displayType === 'card' ? 'border-[var(--color-kb-accent)] bg-[var(--color-kb-accent)]' : 'border-[var(--color-kb-text-muted)] group-hover:border-[var(--color-kb-accent)]'}`}>
                  {displayType === 'card' && <div className="w-1.5 h-1.5 bg-white rounded-full"></div>}
                </div>
                <input type="radio" className="hidden" checked={displayType === 'card'} onChange={() => setDisplayType('card')} />
                <span className="text-sm text-[var(--color-kb-text)]">{t('cardType')}</span>
              </label>
            </div>
          </div>

          {/* Text Content Fields based on type */}
          {displayType === 'text' && (
            <div className="flex items-start mt-2 border-t border-[var(--color-kb-panel-border)] pt-8">
              <div className="w-[120px] flex-shrink-0 flex items-center pt-2 text-sm text-[var(--color-kb-text-muted)]">
                {t('textContentLabel')}
              </div>
              <div className="flex-1 max-w-[500px]">
                <input 
                  type="text" 
                  value={textContent}
                  onChange={e => setTextContent(e.target.value)}
                  className="w-full border border-[var(--color-kb-panel-border)] rounded bg-transparent px-3 py-2 text-sm text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] transition-colors"
                />
                <div className="text-[13px] text-[var(--color-kb-text-muted)] mt-3">
                  {t('textTypeHint')}
                </div>
              </div>
            </div>
          )}

          {displayType === 'image' && (
            <div className="flex items-start mt-2 border-t border-[var(--color-kb-panel-border)] pt-8">
              <div className="w-[120px] flex-shrink-0 flex items-center text-sm text-[var(--color-kb-text-muted)]">
                {t('imageType')}
              </div>
              <div className="flex-1 max-w-[500px]">
                <div className="text-[13px] text-[var(--color-kb-text-muted)] mb-3">
                  {t('imageTypeHint')}
                </div>
                <input
                  type="url"
                  value={imageUrl}
                  onChange={event => setImageUrl(event.target.value)}
                  placeholder={t('imageUrlPlaceholder', { defaultValue: 'https://example.com/image.png' })}
                  className="w-full border border-[var(--color-kb-panel-border)] rounded bg-transparent px-3 py-2 text-sm text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] transition-colors"
                />
                <p className="text-[12px] text-[var(--color-kb-text-muted)] mt-2">
                  {t('imagePickerUnavailable', {
                    defaultValue: 'Upload and asset library integrations are not configured. Enter an image URL.',
                  })}
                </p>
              </div>
            </div>
          )}

          {displayType === 'card' && (
            <>
              <div className="flex items-start mt-2 border-t border-[var(--color-kb-panel-border)] pt-8">
                <div className="w-[120px] flex-shrink-0 flex items-center pt-2 text-sm text-[var(--color-kb-text-muted)]">
                  {t('cardTitleLabel')}
                </div>
                <div className="flex-1 max-w-[500px]">
                  <div className="relative">
                    <input 
                      type="text" 
                      value={cardTitle}
                      onChange={e => setCardTitle(e.target.value)}
                      maxLength={20}
                      className="w-full border border-[var(--color-kb-panel-border)] rounded bg-transparent px-3 py-2 text-sm text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] transition-colors pr-12"
                    />
                    <span className="absolute right-3 top-2.5 text-xs text-gray-400">{cardTitle.length}/20</span>
                  </div>
                  <div className="text-[13px] text-[var(--color-kb-text-muted)] mt-3">
                    {t('cardTypeHint')}
                  </div>
                </div>
              </div>

              <div className="flex items-start mt-2">
                <div className="w-[120px] flex-shrink-0 flex items-start pt-2 text-sm text-[var(--color-kb-text-muted)]">
                  {t('cardStyleLabel')}
                </div>
                <div className="flex-1 max-w-[500px]">
                  <div className="w-[280px] bg-[#f8f8f8] border border-[#e3e3e3] rounded flex flex-col items-center justify-center p-6 relative">
                    <div className="absolute top-3 left-3 bg-gray-200 rounded-full w-5 h-5 flex items-center justify-center">
                      <User size={12} className="text-white" />
                    </div>
                    {imageUrl ? (
                       <img src={imageUrl} alt="preview" className="w-[240px] h-[192px] object-cover rounded mt-4" />
                    ) : (
                      <div className="w-[240px] h-[192px] border border-dashed border-[#ccc] bg-white mt-4 flex flex-col items-center justify-center p-4">
                        <div className="text-[13px] text-[#999] text-center mb-4 leading-relaxed">
                          {t('cardImageHint')}
                        </div>
                        <p className="text-[12px] text-[#999] text-center leading-relaxed">
                          {t('imagePickerUnavailable', {
                            defaultValue: 'Upload and asset library integrations are not configured.',
                          })}
                        </p>
                      </div>
                    )}
                  </div>
                  <input
                    type="url"
                    value={imageUrl}
                    onChange={event => setImageUrl(event.target.value)}
                    placeholder={t('imageUrlPlaceholder', { defaultValue: 'https://example.com/image.png' })}
                    className="w-[280px] mt-3 border border-[var(--color-kb-panel-border)] rounded bg-transparent px-3 py-2 text-sm text-[var(--color-kb-text)] focus:outline-none focus:border-[var(--color-kb-accent)] transition-colors"
                  />
                  <div className="text-[14px] text-[#333] mt-3 flex items-center">
                    <span className="w-3 h-3 rounded-full border border-[var(--color-kb-accent)] flex items-center justify-center mr-2">
                      <span className="w-1.5 h-1.5 bg-[var(--color-kb-accent)] rounded-full"></span>
                    </span>
                    {t('appletLabel')}
                  </div>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-center py-6 border-t border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] rounded-b-xl gap-4">
          <button 
            onClick={onClose}
            className="px-8 py-3 bg-[var(--color-kb-editor)] hover:bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text-heading)] text-[15px] font-semibold rounded-xl transition-colors min-w-[120px]"
          >
            {t('cancel')}
          </button>
          <button 
            onClick={handleConfirm}
            className="px-12 py-3 bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white text-[15px] font-semibold rounded-xl shadow-[0_4px_12px_rgba(37,99,235,0.2)] hover:shadow-[0_6px_16px_rgba(37,99,235,0.3)] transition-all min-w-[140px]"
          >
            {t('confirm')}
          </button>
        </div>
      </div>

      <AppletManagerModal 
        isOpen={isAppletManagerOpen}
        onClose={() => setIsAppletManagerOpen(false)}
        onSelect={handleAppletSelect}
        initialApplets={applets}
        initialGroups={appletGroups}
        onSaveApplets={handleSaveApplets}
      />
    </div>
  );
}
