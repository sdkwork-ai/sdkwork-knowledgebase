import { CircleAlert, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface WechatScanModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function WechatScanModal({
  isOpen,
  onClose,
}: WechatScanModalProps) {
  const { t } = useTranslation('editor');

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-zinc-950/40 backdrop-blur-md z-[300] flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-[var(--color-kb-panel)] border border-[var(--color-kb-panel-border)] rounded-2xl max-w-sm w-full overflow-hidden shadow-2xl flex flex-col">
        <div className="p-5 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between">
          <span className="text-base font-bold text-[var(--color-kb-text-heading)]">{t('scanUploadTitle', { ns: 'editor' })}</span>
          <button
            type="button"
            onClick={onClose}
            aria-label={t('close', { ns: 'common', defaultValue: 'Close' })}
            className="text-[var(--color-kb-text-muted)] hover:text-[var(--color-kb-text)] transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        <div className="p-8 flex flex-col items-center text-center" role="alert">
          <div className="w-12 h-12 rounded-full bg-amber-500/10 text-amber-500 flex items-center justify-center mb-4 border border-amber-500/20">
            <CircleAlert size={24} />
          </div>
          <p className="text-[14px] font-bold text-[var(--color-kb-text-heading)] mb-2">
            {t('wechatScanUnavailable', {
              ns: 'editor',
              defaultValue: 'Scan upload is unavailable.',
            })}
          </p>
          <p className="text-xs text-[var(--color-kb-text-muted)] leading-5 max-w-[280px]">
            {t('wechatScanUnavailableDescription', {
              ns: 'editor',
              defaultValue: 'No scan upload service or host adapter is configured.',
            })}
          </p>
        </div>
      </div>
    </div>
  );
}
