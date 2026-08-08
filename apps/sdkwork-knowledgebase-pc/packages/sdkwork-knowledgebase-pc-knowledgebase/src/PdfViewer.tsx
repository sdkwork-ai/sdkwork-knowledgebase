import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Document, Page, pdfjs } from 'react-pdf';
import pdfWorkerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
import 'react-pdf/dist/Page/AnnotationLayer.css';
import 'react-pdf/dist/Page/TextLayer.css';
import { ChevronLeft, ChevronRight, Loader2, ZoomIn, ZoomOut } from 'lucide-react';
import { useKnowledgebaseHostAdapter } from 'sdkwork-knowledgebase-pc-core';
import { DocumentMeta } from './services/document';
import {
  isLocalFilePath,
  loadLocalPdfSource,
  loadPdfSourceFallback,
  resolveInitialPdfSource,
  toReactPdfFile,
  type PdfDocumentSource,
} from './services/pdfDocumentSource';
import { ExportSaveButton } from './components/DocumentExport/ExportSaveButton';
import { useTranslation } from 'react-i18next';

pdfjs.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

export interface PdfViewerProps {
  activeDoc: DocumentMeta;
}

export function PdfViewer({ activeDoc }: PdfViewerProps) {
  const { t } = useTranslation('editor');
  const host = useKnowledgebaseHostAdapter();
  const [numPages, setNumPages] = useState<number>();
  const [pageNumber, setPageNumber] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [fitMode, setFitMode] = useState<'page' | 'width' | 'manual'>('page');
  const [error, setError] = useState<string | null>(null);
  const [isResolvingSource, setIsResolvingSource] = useState(false);
  const [pdfSource, setPdfSource] = useState<PdfDocumentSource | null>(null);
  const [loadedBytes, setLoadedBytes] = useState<Uint8Array | null>(null);
  const fallbackAttemptedRef = useRef(false);
  const prefetchAttemptedRef = useRef(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });
  const [pageDimensions, setPageDimensions] = useState<{ width: number; height: number } | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    setContainerSize({
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight,
    });

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        setContainerSize({ width, height });
      }
    });

    resizeObserver.observe(containerRef.current);
    return () => resizeObserver.disconnect();
  }, []);

  useEffect(() => {
    setPageDimensions(null);
    setPageNumber(1);
    setNumPages(undefined);
    setError(null);
    setLoadedBytes(null);
    fallbackAttemptedRef.current = false;
    prefetchAttemptedRef.current = false;

    const source = activeDoc.url?.trim();
    if (!source) {
      setPdfSource(null);
      return;
    }

    const initialSource = resolveInitialPdfSource(source);
    if (initialSource) {
      setPdfSource(initialSource);
      setIsResolvingSource(false);
      return;
    }

    if (!isLocalFilePath(source)) {
      setPdfSource(null);
      setError('Unsupported PDF source.');
      return;
    }

    let cancelled = false;
    setIsResolvingSource(true);
    void loadLocalPdfSource(source, host)
      .then((resolved) => {
        if (cancelled) return;
        setPdfSource(resolved);
        if (resolved.kind === 'bytes') {
          setLoadedBytes(resolved.data);
        }
      })
      .catch((loadError: unknown) => {
        if (cancelled) return;
        const message = loadError instanceof Error ? loadError.message : 'Failed to load PDF';
        setError(message);
      })
      .finally(() => {
        if (!cancelled) setIsResolvingSource(false);
      });

    return () => {
      cancelled = true;
    };
  }, [activeDoc.id, activeDoc.url, host]);

  const handleDocumentLoadError = useCallback(
    (loadError: Error) => {
      const source = activeDoc.url?.trim();
      if (!source || fallbackAttemptedRef.current) {
        console.error('PDF render error:', loadError);
        setError(loadError.message);
        return;
      }

      fallbackAttemptedRef.current = true;
      setError(null);
      setIsResolvingSource(true);

      void loadPdfSourceFallback(source, host)
        .then((resolved) => {
          setPdfSource(resolved);
          if (resolved.kind === 'bytes') {
            setLoadedBytes(resolved.data);
          }
        })
        .catch((fallbackError: unknown) => {
          const message =
            fallbackError instanceof Error ? fallbackError.message : loadError.message;
          console.error('PDF fallback load error:', fallbackError);
          setError(message);
        })
        .finally(() => {
          setIsResolvingSource(false);
        });
    },
    [activeDoc.url, host]
  );

  const onDocumentLoadSuccess = useCallback(
    ({ numPages: nextNumPages }: { numPages: number }) => {
      setNumPages(nextNumPages);
      setPageNumber(1);
      setError(null);

      const source = activeDoc.url?.trim();
      if (!source || pdfSource?.kind !== 'url' || prefetchAttemptedRef.current) {
        return;
      }

      prefetchAttemptedRef.current = true;
      void loadPdfSourceFallback(source, host)
        .then((resolved) => {
          if (resolved.kind === 'bytes') {
            setLoadedBytes(resolved.data);
          }
        })
        .catch(() => {
          // URL-only sources can still be opened externally.
        });
    },
    [activeDoc.url, host, pdfSource?.kind],
  );

  function onPageLoadSuccess(page: {
    getViewport: (options: { scale: number }) => { width: number; height: number };
  }) {
    const { width, height } = page.getViewport({ scale: 1 });
    setPageDimensions({ width, height });
  }

  const calculatedScale = useMemo(() => {
    if (fitMode === 'manual') {
      return scale;
    }
    if (!pageDimensions || containerSize.width === 0 || containerSize.height === 0) {
      return 1.0;
    }

    const padding = 32;
    const availWidth = Math.max(containerSize.width - padding, 100);
    const availHeight = Math.max(containerSize.height - padding, 100);
    const scaleWidth = availWidth / pageDimensions.width;
    const scaleHeight = availHeight / pageDimensions.height;

    if (fitMode === 'page') {
      return Math.min(scaleWidth, scaleHeight);
    }
    return scaleWidth;
  }, [fitMode, scale, pageDimensions, containerSize]);

  async function handleOpenExternal() {
    if (activeDoc.url) {
      await host.openExternal(activeDoc.url.startsWith('http') ? activeDoc.url : normalizeDownloadUrl(activeDoc.url));
    }
  }

  const downloadableBytes =
    pdfSource?.kind === 'bytes' ? pdfSource.data : loadedBytes;

  const pdfFileName = activeDoc.title.endsWith('.pdf')
    ? activeDoc.title.replace(/\.pdf$/i, '')
    : activeDoc.title;

  const documentFile = pdfSource ? toReactPdfFile(pdfSource) : null;
  const isScrollableVertically =
    pageDimensions && pageDimensions.height * calculatedScale > containerSize.height;

  if (!activeDoc.url) {
    return <div className="flex-1 flex items-center justify-center">No PDF URL provided.</div>;
  }

  return (
    <div className="w-full flex-1 flex flex-col min-h-0 bg-zinc-50 dark:bg-[var(--color-kb-panel)]/30">
      <div className="flex items-center justify-between px-4 py-1 h-[40px] bg-white dark:bg-[var(--color-kb-panel)] border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/80 flex-shrink-0 shadow-sm z-10">
        <div className="flex items-center space-x-2">
          <div className="flex items-center space-x-1">
            <button
              onClick={() => {
                setFitMode('manual');
                setScale((prev) => {
                  const current = fitMode !== 'manual' ? calculatedScale : prev;
                  return Math.max(current - 0.1, 0.3);
                });
              }}
              className="p-1 rounded-lg hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-border)] text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-800 dark:hover:text-[var(--color-kb-text-heading)] transition-all active:scale-95"
              title="缩小"
              type="button"
            >
              <ZoomOut size={14} strokeWidth={2.5} />
            </button>
            <span className="text-[11px] font-bold text-zinc-500 dark:text-[var(--color-kb-text-muted)] min-w-[2.5rem] text-center font-mono select-none">
              {Math.round(calculatedScale * 100)}%
            </span>
            <button
              onClick={() => {
                setFitMode('manual');
                setScale((prev) => {
                  const current = fitMode !== 'manual' ? calculatedScale : prev;
                  return Math.min(current + 0.1, 3.0);
                });
              }}
              className="p-1 rounded-lg hover:bg-zinc-100 dark:hover:bg-[var(--color-kb-panel-border)] text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-800 dark:hover:text-[var(--color-kb-text-heading)] transition-all active:scale-95"
              title="放大"
              type="button"
            >
              <ZoomIn size={14} strokeWidth={2.5} />
            </button>
          </div>

          <div className="flex items-center bg-[#fafafa] dark:bg-black/10 border border-zinc-200/80 dark:border-transparent p-0.5 rounded-lg shadow-inner text-[11px] font-bold">
            <button
              type="button"
              onClick={() => setFitMode('page')}
              className={`px-2 py-0.5 rounded-md transition-all ${
                fitMode === 'page'
                  ? 'bg-white dark:bg-[var(--color-kb-editor)] text-zinc-800 dark:text-white shadow-sm ring-1 ring-black/5 font-semibold'
                  : 'text-zinc-400 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-300'
              }`}
            >
              {t('fitPage', { defaultValue: '自适应' })}
            </button>
            <button
              type="button"
              onClick={() => setFitMode('width')}
              className={`px-2 py-0.5 rounded-md transition-all ${
                fitMode === 'width'
                  ? 'bg-white dark:bg-[var(--color-kb-editor)] text-zinc-800 dark:text-white shadow-sm ring-1 ring-black/5 font-semibold'
                  : 'text-zinc-400 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-300'
              }`}
            >
              {t('fitWidth', { defaultValue: '合宽' })}
            </button>
          </div>
        </div>

        <div className="flex items-center space-x-2 bg-[#fafafa] dark:bg-black/10 border border-zinc-200/80 dark:border-transparent px-2 py-0.5 rounded-lg shadow-inner">
          <button
            type="button"
            disabled={pageNumber <= 1}
            onClick={() => setPageNumber((value) => value - 1)}
            className="p-1 rounded-md hover:bg-zinc-200/60 dark:hover:bg-[var(--color-kb-panel-border)] disabled:opacity-30 disabled:cursor-not-allowed text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-900 dark:hover:text-[var(--color-kb-text-heading)] transition-all active:scale-95"
          >
            <ChevronLeft size={14} strokeWidth={2.5} />
          </button>
          <span className="text-[11px] font-bold text-zinc-600 dark:text-[var(--color-kb-text-muted)] min-w-[3.5rem] text-center font-mono select-none">
            {pageNumber || (numPages ? 1 : '--')} / {numPages || '--'}
          </span>
          <button
            type="button"
            disabled={pageNumber >= (numPages || -1)}
            onClick={() => setPageNumber((value) => value + 1)}
            className="p-1 rounded-md hover:bg-zinc-200/60 dark:hover:bg-[var(--color-kb-panel-border)] disabled:opacity-30 disabled:cursor-not-allowed text-zinc-500 dark:text-[var(--color-kb-text-muted)] hover:text-zinc-900 dark:hover:text-[var(--color-kb-text-heading)] transition-all active:scale-95"
          >
            <ChevronRight size={14} strokeWidth={2.5} />
          </button>
        </div>

        <div className="flex items-center">
          {downloadableBytes ? (
            <ExportSaveButton
              title={pdfFileName}
              extension="pdf"
              getBytes={() => downloadableBytes}
              disabled={!downloadableBytes}
              primaryLabel={t('download')}
              dropdownClassName="relative export-save-dropdown"
            />
          ) : (
            <button
              type="button"
              onClick={() => void handleOpenExternal()}
              className="text-[11px] px-2.5 py-1 bg-indigo-50 dark:bg-[var(--color-kb-accent)]/10 text-indigo-600 dark:text-[var(--color-kb-accent)] rounded-lg hover:bg-indigo-100 dark:hover:bg-[var(--color-kb-accent)]/20 transition-all font-bold border border-indigo-200 dark:border-[var(--color-kb-accent)]/20 active:scale-95"
            >
              {t('download')}
            </button>
          )}
        </div>
      </div>

      <div
        ref={containerRef}
        className={`flex-1 overflow-auto w-full flex justify-center py-4 px-4 relative no-scrollbar ${
          isScrollableVertically ? 'items-start' : 'items-center'
        }`}
      >
        {isResolvingSource ? (
          <div className="flex items-center gap-2 text-sm text-zinc-500 dark:text-[var(--color-kb-text-muted)] mt-10">
            <Loader2 className="w-4 h-4 animate-spin" />
            {t('loadingPdf', { defaultValue: '正在加载 PDF…' })}
          </div>
        ) : error ? (
          <div className="text-red-500 font-medium text-sm mt-10 max-w-md text-center">
            {t('pdfLoadFailed', { defaultValue: 'PDF 加载失败' })}: {error}
          </div>
        ) : documentFile ? (
          <Document
            key={pdfSource?.kind === 'bytes' ? `bytes-${activeDoc.id}` : `url-${activeDoc.url}`}
            file={documentFile as unknown as File}
            onLoadSuccess={onDocumentLoadSuccess}
            onLoadError={handleDocumentLoadError}
            loading={
              <div className="animate-pulse space-y-4 pt-10">
                <div className="h-[800px] w-[600px] bg-white dark:bg-[var(--color-kb-panel-border)] rounded-xl shadow-sm opacity-50 border border-zinc-200 dark:border-transparent" />
              </div>
            }
            className="pdf-document drop-shadow-xl"
          >
            <div className="shadow-2xl ring-1 ring-zinc-200/80 dark:ring-white/10 rounded-xl overflow-hidden bg-white">
              <Page
                pageNumber={pageNumber}
                scale={calculatedScale}
                onLoadSuccess={onPageLoadSuccess}
                renderTextLayer={true}
                renderAnnotationLayer={true}
                loading={
                  <div
                    style={{
                      width: pageDimensions ? pageDimensions.width * calculatedScale : '600px',
                      height: pageDimensions ? pageDimensions.height * calculatedScale : '800px',
                    }}
                    className="bg-white"
                  />
                }
              />
            </div>
          </Document>
        ) : null}
      </div>
    </div>
  );
}

function normalizeDownloadUrl(source: string): string {
  return new URL(source, globalThis.location?.origin ?? 'http://localhost').href;
}
