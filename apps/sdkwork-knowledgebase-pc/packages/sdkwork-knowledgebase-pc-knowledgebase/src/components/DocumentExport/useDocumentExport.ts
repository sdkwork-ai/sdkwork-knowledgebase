import { useCallback, useRef, useState } from 'react';
import { isBlank } from '@sdkwork/utils';
import { toast } from '../ui/toast-manager';
import { getDocumentExportCapabilities } from './documentExportCapabilities';
import {
  exportDocumentAsImage,
  exportDocumentAsMarkdown,
  exportDocumentAsPdf,
  exportDocumentAsWord,
} from './documentExportRender';
import {
  DEFAULT_EXPORT_TITLE,
  hasExportableContent,
  resolveExportMarkdown,
} from './exportContentUtils';
import { emitExportPostHints } from './exportPostHints';
import { dismissExportProgress, showExportProgress } from './exportProgress';
import { notifyExportCancelled, notifyExportSaveResult } from './exportSaveNotify';
import { isDesktopExportHost } from './exportRuntime';
import type {
  DocumentExportContent,
  DocumentExportContentProvider,
  DocumentExportFormat,
  DocumentExportResult,
  ExportSaveMode,
} from './types';

export interface UseDocumentExportOptions {
  getContent: DocumentExportContentProvider;
}

export interface UseDocumentExportResult {
  exportPdf: (mode?: ExportSaveMode) => Promise<void>;
  exportMarkdown: (mode?: ExportSaveMode) => Promise<void>;
  exportWord: (mode?: ExportSaveMode) => Promise<void>;
  exportImage: (mode?: ExportSaveMode) => Promise<void>;
  exportByFormat: (format: DocumentExportFormat, mode?: ExportSaveMode) => Promise<void>;
  isExporting: boolean;
}

function resolvePdfProgressMessage(content: DocumentExportContent, mode: ExportSaveMode): string {
  if (mode === 'saveAs') {
    return '请选择 PDF 保存位置...';
  }
  const capabilities = getDocumentExportCapabilities(content.sourceKind);
  switch (capabilities.pdfEngine) {
    case 'native-markdown':
      return '正在通过 Typst 生成 PDF...';
    case 'native-webview':
      return '正在通过 WebView 生成 PDF...';
    default:
      return '正在生成 PDF，请稍等...';
  }
}

function resolvePdfEngineHint(
  content: DocumentExportContent,
  result: DocumentExportResult,
): string | undefined {
  if (result.pdfEngine === 'native-markdown') {
    return 'Typst PDF 已生成。';
  }
  if (result.pdfEngine === 'native-webview') {
    return 'WebView PDF 已生成。';
  }
  if (!result.usedCanvasFallback) {
    return undefined;
  }

  const expected = getDocumentExportCapabilities(content.sourceKind).pdfEngine;
  if (expected === 'native-markdown') {
    return 'Typst 引擎不可用，已改用浏览器渲染。';
  }
  if (expected === 'native-webview') {
    return 'WebView 引擎不可用，已改用浏览器渲染。';
  }
  return undefined;
}

async function finalizeExportResult(
  result: DocumentExportResult,
  options?: { extra?: string },
) {
  if (result.save.cancelled) {
    notifyExportCancelled();
    return;
  }
  if (!result.save.saved) {
    dismissExportProgress();
    toast.error(isDesktopSaveFailure(result) ? '桌面端保存失败，请检查权限或重试' : '保存失败');
    return;
  }

  emitExportPostHints({
    imageLoadFailures: result.imageLoadFailures,
    usedTiledRender: result.usedTiledRender,
    canvasMayBeClipped: result.canvasMayBeClipped,
  });
  void notifyExportSaveResult(result.save, options?.extra);
}

function isDesktopSaveFailure(_result: DocumentExportResult): boolean {
  return isDesktopExportHost();
}

export function useDocumentExport({
  getContent,
}: UseDocumentExportOptions): UseDocumentExportResult {
  const [isExporting, setIsExporting] = useState(false);
  const exportLockRef = useRef(false);

  const requireContent = useCallback((): DocumentExportContent | null => {
    const content = getContent();
    if (!content) {
      toast.error('当前没有可导出的内容');
      return null;
    }
    const normalized: DocumentExportContent = {
      title: content.title || DEFAULT_EXPORT_TITLE,
      html: content.html,
      markdown: content.markdown,
      sourceKind: content.sourceKind,
    };
    if (!hasExportableContent(normalized)) {
      toast.error('文档内容为空，无法导出');
      return null;
    }
    return normalized;
  }, [getContent]);

  const runExport = useCallback(async (task: () => Promise<void>) => {
    if (exportLockRef.current) {
      toast.info('正在导出，请稍候...');
      return;
    }
    exportLockRef.current = true;
    setIsExporting(true);
    try {
      await task();
    } finally {
      exportLockRef.current = false;
      setIsExporting(false);
    }
  }, []);

  const exportPdf = useCallback(
    async (mode: ExportSaveMode = 'downloads') => {
      const content = requireContent();
      if (!content) return;

      await runExport(async () => {
        showExportProgress(resolvePdfProgressMessage(content, mode));
        try {
          const result = await exportDocumentAsPdf(content, mode);
          const engineHint = resolvePdfEngineHint(content, result);
          await finalizeExportResult(result, {
            extra: engineHint ? `${engineHint} ` : undefined,
          });
        } catch (error: unknown) {
          dismissExportProgress();
          console.error(error);
          const message = error instanceof Error ? error.message : String(error);
          toast.error(`PDF 生成失败: ${message}`);
        }
      });
    },
    [requireContent, runExport],
  );

  const exportMarkdown = useCallback(
    async (mode: ExportSaveMode = 'downloads') => {
      const content = requireContent();
      if (!content) return;

      await runExport(async () => {
        showExportProgress(
          mode === 'saveAs' ? '请选择 Markdown 保存位置...' : '正在导出 Markdown...',
        );
        try {
          const markdown = resolveExportMarkdown(content);
          if (isBlank(markdown)) {
            dismissExportProgress();
            toast.error('Markdown 内容为空，无法导出');
            return;
          }
          const result = await exportDocumentAsMarkdown(
            { title: content.title, markdown },
            mode,
          );
          await finalizeExportResult(result);
        } catch (error: unknown) {
          dismissExportProgress();
          console.error(error);
          const message = error instanceof Error ? error.message : String(error);
          toast.error(`Markdown 导出失败: ${message}`);
        }
      });
    },
    [requireContent, runExport],
  );

  const exportWord = useCallback(
    async (mode: ExportSaveMode = 'downloads') => {
      const content = requireContent();
      if (!content) return;

      await runExport(async () => {
        showExportProgress(mode === 'saveAs' ? '请选择 Word 保存位置...' : '正在导出 Word...');
        try {
          const result = await exportDocumentAsWord(content, mode);
          await finalizeExportResult(result);
        } catch (error: unknown) {
          dismissExportProgress();
          console.error(error);
          const message = error instanceof Error ? error.message : String(error);
          toast.error(`Word 导出失败: ${message}`);
        }
      });
    },
    [requireContent, runExport],
  );

  const exportImage = useCallback(
    async (mode: ExportSaveMode = 'downloads') => {
      const content = requireContent();
      if (!content) return;

      await runExport(async () => {
        showExportProgress(mode === 'saveAs' ? '请选择图片保存位置...' : '正在渲染图片，请稍等...');
        try {
          const result = await exportDocumentAsImage(content, mode);
          await finalizeExportResult(result);
        } catch (error: unknown) {
          dismissExportProgress();
          console.error(error);
          const message = error instanceof Error ? error.message : String(error);
          toast.error(`图片生成失败: ${message}`);
        }
      });
    },
    [requireContent, runExport],
  );

  const exportByFormat = useCallback(
    async (format: DocumentExportFormat, mode: ExportSaveMode = 'downloads') => {
      switch (format) {
        case 'pdf':
          await exportPdf(mode);
          break;
        case 'markdown':
          await exportMarkdown(mode);
          break;
        case 'word':
          await exportWord(mode);
          break;
        case 'image':
          await exportImage(mode);
          break;
        default:
          break;
      }
    },
    [exportImage, exportMarkdown, exportPdf, exportWord],
  );

  return {
    exportPdf,
    exportMarkdown,
    exportWord,
    exportImage,
    exportByFormat,
    isExporting,
  };
}
