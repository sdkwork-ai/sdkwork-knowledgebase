import { useCallback, useEffect, useRef, useState } from 'react';
import { isBlank, trim } from '@sdkwork/utils';
import { ExternalLink, Globe, RefreshCw, X } from 'lucide-react';

export interface InAppBrowserModalProps {
  url: string | null;
  title?: string;
  onClose: () => void;
}

function normalizeUrl(raw: string): string {
  const value = trim(raw);
  if (isBlank(value)) {
    return '';
  }
  const candidate = /^https?:\/\//iu.test(value) ? value : `https://${value}`;
  try {
    const parsed = new URL(candidate);
    return parsed.protocol === 'https:' || parsed.protocol === 'http:' ? parsed.toString() : '';
  } catch {
    return '';
  }
}

export function InAppBrowserModal({ url, title, onClose }: InAppBrowserModalProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [currentUrl, setCurrentUrl] = useState('');
  const [frameKey, setFrameKey] = useState(0);
  const [loadError, setLoadError] = useState(false);

  useEffect(() => {
    if (!url) return;
    const normalized = normalizeUrl(url);
    setCurrentUrl(normalized);
    setLoadError(false);
    setFrameKey((k) => k + 1);
  }, [url]);

  useEffect(() => {
    if (!url) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [url, onClose]);

  const handleReload = useCallback(() => {
    setLoadError(false);
    setFrameKey((k) => k + 1);
  }, []);

  const handleOpenExternal = useCallback(() => {
    if (currentUrl) window.open(currentUrl, '_blank', 'noopener,noreferrer');
  }, [currentUrl]);

  if (!url) return null;

  const displayTitle = title?.trim() || '网页预览';

  return (
    <div className="fixed inset-0 z-[10050] flex items-center justify-center p-4 md:p-8 animate-in fade-in duration-200">
      <div className="absolute inset-0 bg-black/45 backdrop-blur-[2px]" onClick={onClose} aria-hidden />
      <div className="relative flex flex-col w-full max-w-5xl h-[min(88vh,820px)] rounded-2xl border border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] shadow-2xl overflow-hidden animate-in zoom-in-95 duration-200">
        <div className="shrink-0 flex items-center gap-2 px-3 py-2.5 border-b border-[var(--color-kb-panel-border)] bg-[var(--color-kb-panel)]/40">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-kb-panel-active)] text-[var(--color-kb-accent)] shrink-0">
            <Globe className="w-4 h-4" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold text-[var(--color-kb-text-heading)] truncate">{displayTitle}</p>
            <p className="text-[10px] text-[var(--color-kb-text-muted)] truncate font-mono">{currentUrl}</p>
          </div>
          <button
            type="button"
            onClick={handleReload}
            className="p-2 rounded-lg text-[var(--color-kb-text-muted)] hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text)] transition-colors"
            title="刷新"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            type="button"
            onClick={handleOpenExternal}
            className="p-2 rounded-lg text-[var(--color-kb-text-muted)] hover:bg-[var(--color-kb-panel-hover)] hover:text-[var(--color-kb-text)] transition-colors"
            title="在系统浏览器打开"
          >
            <ExternalLink className="w-4 h-4" />
          </button>
          <button
            type="button"
            onClick={onClose}
            className="p-2 rounded-lg text-[var(--color-kb-text-muted)] hover:bg-red-500/10 hover:text-red-500 transition-colors"
            title="关闭"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex-1 min-h-0 relative bg-[var(--color-kb-editor)]">
          {loadError ? (
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-3 px-8 text-center">
              <p className="text-sm text-[var(--color-kb-text-muted)]">
                该页面可能不允许在内置浏览器中嵌入显示（X-Frame-Options 限制）。
              </p>
              <button
                type="button"
                onClick={handleOpenExternal}
                className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-semibold bg-[var(--color-kb-accent)] text-white hover:opacity-90 transition-opacity"
              >
                <ExternalLink className="w-4 h-4" />
                在系统浏览器打开
              </button>
            </div>
          ) : (
            <iframe
              key={frameKey}
              ref={iframeRef}
              src={currentUrl}
              title={displayTitle}
              className="w-full h-full border-0 bg-white dark:bg-[var(--color-kb-editor)]"
              referrerPolicy="no-referrer"
              sandbox="allow-scripts allow-forms allow-popups"
              onError={() => setLoadError(true)}
            />
          )}
        </div>
      </div>
    </div>
  );
}

export function InAppBrowserHost() {
  const [state, setState] = useState<{ url: string; title?: string } | null>(null);

  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<{ url: string; title?: string }>).detail;
      if (!detail?.url) return;
      setState({ url: detail.url, title: detail.title });
    };
    window.addEventListener('app-open-in-app-browser', handler);
    return () => window.removeEventListener('app-open-in-app-browser', handler);
  }, []);

  return (
    <InAppBrowserModal
      url={state?.url ?? null}
      title={state?.title}
      onClose={() => setState(null)}
    />
  );
}
