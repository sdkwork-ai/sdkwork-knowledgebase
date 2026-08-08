import { useState } from 'react';
import { isBlank } from '@sdkwork/utils';
import { X, Clock, FileText, ArrowUp, ChevronDown, ChevronUp } from 'lucide-react';
import {
  isKnowledgebaseApiAvailable,
  resolveUserFacingErrorMessage,
  type ErrorTranslateFn,
} from 'sdkwork-knowledgebase-pc-core';
import { AIService } from '../services/ai';
import { useTranslation } from 'react-i18next';

export interface WechatAiImageModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (imageUrl: string) => void;
  kbId?: string | null;
}

interface Message {
  role: 'user' | 'ai';
  type: 'text' | 'image_result';
  content: string;
  imageDetails?: {
    url: string;
    resolution: string;
    suggestions: string[];
    similars: string[];
  };
}

function appendImageGenerationError(messages: Message[], detail: string): Message[] {
  return [
    ...messages,
    {
      role: 'ai',
      type: 'text',
      content: detail,
    },
  ];
}

export function WechatAiImageModal({ isOpen, onClose, onConfirm, kbId }: WechatAiImageModalProps) {
  const { t } = useTranslation(['editor', 'common', 'officialAccount']);
  const [prompt, setPrompt] = useState('');
  const [messages, setMessages] = useState<Message[]>([]);
  const [aspectMode, setAspectMode] = useState<'1:1' | '16:9' | '9:16'>('1:1');
  const [styleMode, setStyleMode] = useState<
    'illustration' | 'photography' | 'minimalist' | 'abstract'
  >('illustration');
  const [showSimilars, setShowSimilars] = useState(true);
  const imageGenerationAvailable = isKnowledgebaseApiAvailable();

  const handleGenerateImage = async () => {
    if (!imageGenerationAvailable || isBlank(prompt)) return;

    const newMsg: Message = { role: 'user', type: 'text', content: prompt.trim() };
    setMessages(prev => [...prev, newMsg]);
    setPrompt('');

    try {
      const result = await AIService.generateImage(
        newMsg.content,
        aspectMode,
        styleMode,
        kbId ? { spaceId: kbId } : undefined,
      );
      setMessages(prev => [...prev, {
        role: 'ai',
        type: 'image_result',
        content: t('ai_generated_desc_suggestions', {
          aspect: aspectMode,
          style: styleMode,
          defaultValue: `已为您生成图片 (${aspectMode} - ${styleMode})，包含相关的调整建议。`,
        }),
        imageDetails: result,
      }]);
    } catch (error) {
      const detail = resolveUserFacingErrorMessage(
        error,
        t as unknown as ErrorTranslateFn,
      );
      setMessages(prev => appendImageGenerationError(prev, detail));
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[400] flex items-center justify-center bg-zinc-950/40 backdrop-blur-md p-4 sm:p-6 lg:p-8 animate-in fade-in duration-200">
      <div className="bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] w-full max-w-4xl h-full max-h-[90vh] rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 flex-shrink-0 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/50">
          <div className="flex items-center gap-4">
            <h2 className="text-[15px] font-bold tracking-tight text-[var(--color-kb-text-heading)]">
              {t('ai_image_helper', { defaultValue: 'AI 配图助手' })}
            </h2>
            <div className="flex items-center gap-3 text-[var(--color-kb-text-muted)]">
              <button className="hover:text-[var(--color-kb-text-heading)] transition-colors"><Clock size={16} /></button>
              <button className="hover:text-[var(--color-kb-text-heading)] transition-colors"><FileText size={16} /></button>
            </div>
          </div>
          <button 
            onClick={onClose} 
            className="text-[var(--color-kb-text-muted)] hover:text-red-500 hover:bg-red-500/10 p-1.5 rounded-lg transition-all"
          >
            <X size={18} strokeWidth={2} />
          </button>
        </div>

        {/* Chat Area */}
        <div className="flex-1 overflow-y-auto px-6 py-6 flex flex-col gap-8 bg-[var(--color-kb-editor)]">
          {messages.length === 0 && (
            <div
              role={imageGenerationAvailable ? 'status' : 'alert'}
              className="max-w-[80%] text-[14px] leading-relaxed bg-[var(--color-kb-panel-hover)]/30 border border-[var(--color-kb-panel-border)]/50 rounded-xl p-4 text-[var(--color-kb-text)]"
            >
              {imageGenerationAvailable
                ? t('wechatAiImageReady', {
                    defaultValue: 'Describe the image you want to generate.',
                  })
                : t('wechatAiImageUnavailable', {
                    defaultValue: 'AI image generation is unavailable until the Knowledgebase API is connected.',
                  })}
            </div>
          )}
          {messages.map((msg, idx) => (
            <div key={idx} className={`flex flex-col ${msg.role === 'user' ? 'items-end' : 'items-start'}`}>
              {msg.role === 'user' ? (
                <div className="bg-[var(--color-kb-panel-active)] border border-[var(--color-kb-editor-border)] text-[var(--color-kb-panel-text)] px-4.5 py-3 rounded-2xl rounded-tr-sm max-w-[80%] text-[14px] font-medium shadow-sm">
                  {msg.content}
                </div>
              ) : (
                <div className="flex flex-col gap-3.5 max-w-[80%]">
                  <div className="text-[var(--color-kb-text)] text-[14.5px] leading-relaxed bg-[var(--color-kb-panel-hover)]/30 border border-[var(--color-kb-panel-border)]/50 rounded-2xl rounded-tl-none p-4 shadow-xs">{msg.content}</div>
                  
                  {msg.type === 'image_result' && msg.imageDetails && (
                    <div className="flex flex-col gap-4">
                      {/* Main Image */}
                      <div className="relative group cursor-pointer w-[320px] h-[320px] overflow-hidden rounded-2xl border border-[var(--color-kb-panel-border)] shadow-md" onClick={() => onConfirm(msg.imageDetails!.url)}>
                        <img 
                          referrerPolicy="no-referrer"
                          src={msg.imageDetails.url} 
                          alt="AI generated" 
                          className="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
                        />
                        <div className="absolute bottom-3 right-3 bg-black/50 backdrop-blur-md text-white text-[11px] px-2.5 py-1 rounded-xl flex items-center gap-1.5 font-medium opacity-90">
                           <SparklesIcon /> {t('ai_illustration', { defaultValue: 'AI 配图' })}
                        </div>
                        <div className="absolute inset-0 bg-black/0 group-hover:bg-zinc-950/25 transition-colors flex items-center justify-center">
                           <div className="opacity-0 group-hover:opacity-100 bg-[var(--color-kb-panel)] text-[var(--color-kb-text-heading)] border border-[var(--color-kb-panel-border)] text-xs font-bold px-4.5 py-2.5 rounded-full shadow-lg transition-opacity transform translate-y-2 group-hover:translate-y-0">
                             {t('use_this_image', { defaultValue: '使用此图片' })}
                           </div>
                        </div>
                      </div>

                      {/* Suggestions */}
                      <div className="flex flex-wrap gap-2">
                        {msg.imageDetails.suggestions.map((sug, i) => (
                          <button key={i} className="px-3.5 py-1.5 bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] hover:border-emerald-500/40 text-[var(--color-kb-text)] rounded-full text-[12.5px] hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-accent)] transition-colors font-medium">
                            {sug}
                          </button>
                        ))}
                      </div>

                      {/* Similar Images */}
                      <div className="flex flex-col gap-3 mt-2">
                        <button 
                          className="flex items-center gap-1 text-[13px] text-[var(--color-kb-accent)] hover:text-[var(--color-kb-accent-hover)] transition-colors w-fit font-bold"
                          onClick={() => setShowSimilars(!showSimilars)}
                        >
                          {t('similar_images', { defaultValue: '类似图片' })} {showSimilars ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                        </button>
                        
                        {showSimilars && (
                          <div className="flex items-center gap-2.5">
                            {msg.imageDetails.similars.map((simUrl, i) => (
                              <img key={i} referrerPolicy="no-referrer" src={simUrl} className="w-[64px] h-[64px] rounded-xl object-cover border border-[var(--color-kb-panel-border)] cursor-pointer hover:border-[var(--color-kb-accent)] transition-colors" />
                            ))}
                            <button className="h-[64px] px-4.5 bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text-heading)] rounded-xl text-[12px] flex items-center justify-center font-bold transition-all">
                              {t('more_arrow', { defaultValue: '更多 >' })}
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Input Area */}
        <div className="px-6 pb-6 pt-3 bg-[var(--color-kb-panel)]/50 border-t border-[var(--color-kb-panel-border)] flex-shrink-0 flex flex-col gap-4">
          <div className="border border-[var(--color-kb-panel-border)] rounded-2xl focus-within:border-[var(--color-kb-accent)] focus-within:ring-2 focus-within:ring-[var(--color-kb-accent)]/20 transition-all bg-[var(--color-kb-editor)] overflow-hidden flex flex-col shadow-xs">
            <textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              disabled={!imageGenerationAvailable}
              placeholder={imageGenerationAvailable
                ? t('ai_illustration_placeholder', { defaultValue: 'Describe the image you want to create.' })
                : t('wechatAiImageUnavailable', { defaultValue: 'AI image generation is unavailable.' })}
              className="w-full min-h-[80px] max-h-[160px] resize-none outline-none text-[14px] font-medium p-4 text-[var(--color-kb-text-heading)] placeholder-[var(--color-kb-text-muted)] bg-transparent disabled:cursor-not-allowed disabled:opacity-60"
              onKeyDown={async (e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  await handleGenerateImage();
                }
              }}
            />
            
            <div className="flex items-center justify-between px-4 pb-4">
              <div className="flex items-center gap-2">
                <select
                  aria-label={t('aspectRatio', { defaultValue: 'Aspect ratio' })}
                  value={aspectMode}
                  onChange={(event) => setAspectMode(event.target.value as typeof aspectMode)}
                  className="h-8 rounded-lg border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] px-3 text-[12.5px] font-semibold text-[var(--color-kb-text)] outline-none transition-colors hover:bg-[var(--color-kb-panel-hover)] focus:border-[var(--color-kb-accent)]"
                >
                  <option value="1:1">1:1</option>
                  <option value="16:9">16:9</option>
                  <option value="9:16">9:16</option>
                </select>
                <select
                  aria-label={t('imageStyle', { defaultValue: 'Image style' })}
                  value={styleMode}
                  onChange={(event) => setStyleMode(event.target.value as typeof styleMode)}
                  className="h-8 rounded-lg border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)] px-3 text-[12.5px] font-semibold text-[var(--color-kb-text)] outline-none transition-colors hover:bg-[var(--color-kb-panel-hover)] focus:border-[var(--color-kb-accent)]"
                >
                  <option value="illustration">
                    {t('aiStyleIllustration', { defaultValue: 'Illustration' })}
                  </option>
                  <option value="photography">
                    {t('aiStylePhotography', { defaultValue: 'Photography' })}
                  </option>
                  <option value="minimalist">
                    {t('aiStyleMinimalist', { defaultValue: 'Minimalist' })}
                  </option>
                  <option value="abstract">
                    {t('aiStyleAbstract', { defaultValue: 'Abstract' })}
                  </option>
                </select>
              </div>
              
              <button 
                type="button"
                onClick={handleGenerateImage}
                disabled={!imageGenerationAvailable || isBlank(prompt)}
                aria-label={t('generateImage', { defaultValue: 'Generate image' })}
                className={`w-8 h-8 rounded-full flex items-center justify-center transition-all ${
                  imageGenerationAvailable && !isBlank(prompt) ? 'bg-[var(--color-kb-accent)] text-white hover:opacity-90 shadow-sm' : 'bg-[var(--color-kb-panel-hover)] border border-[var(--color-kb-panel-border)] text-[var(--color-kb-text-muted)] cursor-not-allowed'
                }`}
              >
                <ArrowUp size={16} strokeWidth={2.5} />
              </button>
            </div>
          </div>
          
          <div className="text-center text-[11px] text-[var(--color-kb-text-muted)] font-medium">
            {t('agree_terms_first', { defaultValue: '已阅读并同意遵守' })} <a href="#" className="hover:text-[var(--color-kb-text-heading)] underline">《{t('wechat_ai_terms', { defaultValue: '微信公众平台AI配图功能使用条款' })}》</a>{t('and', { defaultValue: '及' })}<a href="#" className="hover:text-[var(--color-kb-text-heading)] underline">《{t('wechat_privacy_hint', { defaultValue: '微信公众平台个人信息保护指引' })}》</a>。
          </div>
        </div>
      </div>
    </div>
  );
}

function SparklesIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"/>
      <path d="M5 3v4"/>
      <path d="M19 17v4"/>
      <path d="M3 5h4"/>
      <path d="M17 19h4"/>
    </svg>
  );
}
