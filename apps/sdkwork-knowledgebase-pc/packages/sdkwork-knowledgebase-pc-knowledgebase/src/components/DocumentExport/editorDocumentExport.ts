import { createEditorExportContentProvider, type EditorExportBinding } from './exportContentUtils';
import type { DocumentExportContentProvider, DocumentExportSourceKind } from './types';

export interface TiptapEditorExportBinding {
  title: string;
  mode: DocumentExportSourceKind;
  isSourceMode: boolean;
  isSplitMode: boolean;
  sourceCode: string;
  getHtml: () => string;
  getMarkdown?: () => string;
  getPlainText?: () => string;
}

export function createTiptapExportContentProvider(
  binding: TiptapEditorExportBinding | (() => TiptapEditorExportBinding | null),
): DocumentExportContentProvider {
  return createEditorExportContentProvider(() => {
    const resolved = typeof binding === 'function' ? binding() : binding;
    if (!resolved) {
      return null;
    }

    const title = resolved.title.trim() || '无标题';

    if (resolved.isSourceMode && !resolved.isSplitMode) {
      if (resolved.mode === 'markdown') {
        return {
          title,
          getHtml: () => '',
          getMarkdown: () => resolved.sourceCode,
          sourceKind: 'markdown' as const,
          isSourceMode: true,
          sourceCode: resolved.sourceCode,
        } satisfies EditorExportBinding;
      }

      return {
        title,
        getHtml: () => resolved.sourceCode,
        getMarkdown: () => resolved.sourceCode.replace(/<[^>]*>/g, '').trim(),
        sourceKind: 'richtext' as const,
        isSourceMode: true,
        sourceCode: resolved.sourceCode,
      } satisfies EditorExportBinding;
    }

    return {
      title,
      getHtml: resolved.getHtml,
      getMarkdown: resolved.getMarkdown,
      getPlainText: resolved.getPlainText,
      sourceKind: resolved.mode,
    } satisfies EditorExportBinding;
  });
}
