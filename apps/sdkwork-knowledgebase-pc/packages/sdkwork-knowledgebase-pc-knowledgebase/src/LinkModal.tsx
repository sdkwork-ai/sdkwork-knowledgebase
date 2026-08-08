import { useRef, useEffect } from 'react';
import { isBlank } from '@sdkwork/utils';
import { createPortal } from 'react-dom';
import { X, Link as LinkIcon, Globe } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface LinkModalProps {
  isOpen: boolean;
  onClose: () => void;
  linkUrl: string;
  setLinkUrl: (url: string) => void;
  onConfirm: () => void;
}

export function LinkModal({ isOpen, onClose, linkUrl, setLinkUrl, onConfirm }: LinkModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  const inputRef = useRef<HTMLInputElement>(null);
  
  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  return createPortal(
    <div className="fixed inset-0 z-[300] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md">
      <div className="w-[480px] bg-white dark:bg-[var(--color-kb-editor)] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] animate-in zoom-in-95 duration-200 overflow-hidden">
         <div className="px-6 py-5 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] flex items-center justify-between bg-[#fafafa] dark:bg-[var(--color-kb-panel)]/30">
           <h3 className="text-[15px] font-display font-extrabold tracking-tight text-zinc-900 dark:text-[var(--color-kb-text-heading)] flex items-center gap-2">
             <div className="w-8 h-8 rounded-xl bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] flex items-center justify-center border border-transparent shadow-inner">
               <LinkIcon size={16} strokeWidth={2.5} />
             </div>
             {t('webLink')}
           </h3>
           <button onClick={onClose} className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:text-[var(--color-kb-text-muted)] dark:hover:bg-red-500/10 p-2 rounded-xl transition-all active:scale-95"><X size={16} strokeWidth={2.5} /></button>
         </div>
         <div className="p-6">
            <div className="flex bg-[var(--color-kb-panel)]/50 p-4 rounded-xl border border-[var(--color-kb-panel-border)]/50 items-start gap-4 mb-5 text-[13px] text-[var(--color-kb-text-muted)] font-medium">
              <Globe size={20} className="text-[var(--color-kb-accent)] shrink-0 mt-0.5" strokeWidth={2} />
              <span className="leading-relaxed text-[13.5px]">输入任意网页链接，我们将尝试为您提取内容正文部分，并转化为知识库内的 Markdown 文档。</span>
            </div>
            <div className="relative">
              <input 
                ref={inputRef}
                type="text" 
                value={linkUrl} 
                onChange={e => setLinkUrl(e.target.value)} 
                placeholder="https://example.com/article..." 
                className="w-full bg-[var(--color-kb-input-bg)] border border-[var(--color-kb-panel-border)]/80 rounded-xl px-4 py-3.5 pl-11 text-[13.5px] font-medium text-[var(--color-kb-text-heading)] focus:outline-none focus:border-[var(--color-kb-accent)]/50 focus:ring-4 focus:ring-[var(--color-kb-accent)]/10 focus:bg-white dark:focus:bg-[var(--color-kb-editor)] transition-all font-mono shadow-sm" 
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && linkUrl.trim()) onConfirm();
                }}
              />
              <LinkIcon size={18} className="absolute left-3.5 top-3.5 text-zinc-400 dark:text-[var(--color-kb-text-muted)]" strokeWidth={2.5} />
            </div>
         </div>
         <div className="px-6 py-4 flex justify-end gap-3 bg-[var(--color-kb-panel)]/30 border-t border-[var(--color-kb-panel-border)]/80">
           <button onClick={onClose} className="px-5 py-2.5 text-[13px] font-bold text-zinc-600 dark:text-[var(--color-kb-text-heading)] hover:bg-[#fafafa] dark:hover:bg-[var(--color-kb-panel-border)] rounded-xl transition-all active:scale-95 border border-[var(--color-kb-panel-border)] shadow-sm bg-white dark:bg-[var(--color-kb-panel)]">{t('cancel', { ns: 'common' })}</button>
           <button 
             onClick={onConfirm} 
             className="px-6 py-2.5 text-[13px] font-extrabold bg-[var(--color-kb-accent)] text-white rounded-xl shadow-md hover:bg-[var(--color-kb-accent-hover)] hover:shadow-lg transition-all disabled:opacity-40 disabled:grayscale disabled:shadow-none flex items-center gap-2 active:scale-95" 
             disabled={isBlank(linkUrl)}
           >
             提取为文档
           </button>
         </div>
      </div>
    </div>,
    document.body
  );
}
