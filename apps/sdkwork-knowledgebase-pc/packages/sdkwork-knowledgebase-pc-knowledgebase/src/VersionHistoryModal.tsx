import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DocumentService, type DocumentVersionSummary } from './services/document';

export interface VersionHistoryModalProps {
  isOpen: boolean;
  item: { id: string; title: string; author?: string } | null;
  onClose: () => void;
}

function formatVersionSize(sizeBytes: number): string {
  if (sizeBytes <= 0) {
    return '—';
  }
  if (sizeBytes < 1024) {
    return `${sizeBytes} B`;
  }
  if (sizeBytes < 1024 * 1024) {
    return `${(sizeBytes / 1024).toFixed(1)} KB`;
  }
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function VersionHistoryModal({ isOpen, item, onClose }: VersionHistoryModalProps) {
  const { t } = useTranslation(['kb', 'common']);
  const [versions, setVersions] = useState<DocumentVersionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen || !item?.id) {
      setVersions([]);
      setError(null);
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);

    DocumentService.listDocumentVersions(item.id)
      .then((items) => {
        if (cancelled) {
          return;
        }
        setVersions(items.slice().sort((left, right) => right.versionNo - left.versionNo));
        setLoading(false);
      })
      .catch((loadError) => {
        if (cancelled) {
          return;
        }
        setVersions([]);
        setLoading(false);
        setError(loadError instanceof Error ? loadError.message : 'Failed to load document versions.');
      });

    return () => {
      cancelled = true;
    };
  }, [isOpen, item?.id]);

  if (!isOpen || !item) {
    return null;
  }

  const latestVersionNo = versions[0]?.versionNo;

  return (
    <div className="fixed inset-0 bg-zinc-950/40 z-[1000] flex items-center justify-center backdrop-blur-sm select-none p-4">
      <div className="bg-white dark:bg-[var(--color-kb-editor)] w-[450px] rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] p-6 animate-in zoom-in-95 duration-200">
        <h3 className="text-[16px] font-extrabold text-zinc-900 dark:text-[var(--color-kb-text-heading)] mb-1.5 tracking-tight">{t('versionHistory')}</h3>
        <p className="text-[13px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] mb-6">
          {t('historyOf', { title: item.title, defaultValue: '{{title}} 的历史记录' })}
        </p>

        <div className="space-y-4 mb-6 max-h-[300px] overflow-y-auto pr-2 no-scrollbar">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <div className="w-7 h-7 border-3 border-indigo-500/30 border-t-indigo-500 rounded-full animate-spin" />
            </div>
          ) : error ? (
            <p className="text-sm text-rose-500 dark:text-rose-400">{error}</p>
          ) : versions.length === 0 ? (
            <p className="text-sm text-zinc-500 dark:text-[var(--color-kb-text-muted)]">
              {t('noVersionHistory', { defaultValue: '暂无版本记录。保存或导入后会自动生成版本。' })}
            </p>
          ) : (
            versions.map((version) => {
              const isCurrent = version.versionNo === latestVersionNo;
              return (
                <div
                  key={version.id}
                  className={`flex items-start gap-4 p-3 rounded-xl transition-colors relative ${
                    isCurrent
                      ? 'bg-zinc-50 dark:bg-[var(--color-kb-panel)] border border-indigo-100 dark:border-[var(--color-kb-panel-border)] shadow-sm'
                      : 'hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel-hover)] border border-transparent'
                  }`}
                >
                  <div
                    className={`absolute left-4 top-4 w-2.5 h-2.5 rounded-full ${
                      isCurrent ? 'bg-indigo-500 shadow-[0_0_0_4px_rgba(99,102,241,0.1)]' : 'bg-zinc-200 dark:bg-zinc-700'
                    }`}
                  />
                  <div className="ml-6">
                    <div className="text-[14px] text-zinc-900 dark:text-[var(--color-kb-text-heading)] font-extrabold tracking-tight">
                      {isCurrent
                        ? t('currentVersion')
                        : t('versionNumber', { number: version.versionNo, defaultValue: '版本 {{number}}' })}
                    </div>
                    <div className="text-[12px] font-medium text-zinc-500 dark:text-[var(--color-kb-text-muted)] mt-1">
                      {formatVersionSize(version.sizeBytes)}
                      {version.mimeType ? ` • ${version.mimeType}` : ''}
                      {' • '}
                      {t('indexState', { state: version.indexState, defaultValue: '索引 {{state}}' })}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>

        <div className="flex justify-end border-t border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] pt-5">
          <button
            type="button"
            onClick={onClose}
            className="px-6 py-2 text-[13px] font-bold bg-white dark:bg-[var(--color-kb-panel-hover)] border-2 border-zinc-200/80 dark:border-[var(--color-kb-panel-border)] text-zinc-700 dark:text-[var(--color-kb-text-heading)] hover:bg-zinc-50 dark:hover:bg-[var(--color-kb-panel)] rounded-xl transition-all active:scale-95 shadow-sm cursor-pointer"
          >
            {t('close', { ns: 'common', defaultValue: '关闭' })}
          </button>
        </div>
      </div>
    </div>
  );
}
