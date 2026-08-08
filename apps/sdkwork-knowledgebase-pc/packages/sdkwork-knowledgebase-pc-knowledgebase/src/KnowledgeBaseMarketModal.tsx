import { X } from 'lucide-react';
import { KnowledgeBaseMarketView } from './KnowledgeBaseMarketView';
import { useTranslation } from 'react-i18next';

interface KnowledgeBaseMarketModalProps {
  onClose: () => void;
  onSubscribedChange: () => void;
}

export function KnowledgeBaseMarketModal({ onClose, onSubscribedChange }: KnowledgeBaseMarketModalProps) {
  const { t } = useTranslation('kb');

  return (
    <div className="fixed inset-0 bg-zinc-950/40 z-[320] flex items-center justify-center backdrop-blur-md animate-fade-in">
      <div className="bg-[var(--color-kb-editor)] w-[95vw] h-[95vh] rounded-3xl shadow-[0_24px_80px_rgba(0,0,0,0.35)] border border-[var(--color-kb-panel-border)] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        
        {/* Simple Header for closing */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]">
          <h3 className="font-display font-bold tracking-tight text-[16px] text-[var(--color-kb-text-heading)]">{t('subscribeSharedKb')}</h3>
          <button onClick={onClose} className="text-[var(--color-kb-text-muted)] hover:text-red-500 p-1.5 rounded-xl hover:bg-[var(--color-kb-panel-hover)] transition-all">
            <X size={20} />
          </button>
        </div>

        {/* Reusing the exact same View component */}
        <div className="flex-1 overflow-hidden relative">
          <KnowledgeBaseMarketView onSubscribedChange={onSubscribedChange} />
        </div>
      </div>
    </div>
  );
}

