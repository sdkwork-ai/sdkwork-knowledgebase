import { useMemo } from 'react';
import { DocumentExportMenu } from './DocumentExportMenu';
import type { DocumentExportMenuProps } from './DocumentExportMenu';
import {
  createTiptapExportContentProvider,
  type TiptapEditorExportBinding,
} from './editorDocumentExport';
import { useDocumentExport, type UseDocumentExportResult } from './useDocumentExport';
import type { DocumentExportFormat } from './types';

type MenuProps = Omit<DocumentExportMenuProps, 'getContent'>;

export interface EditorDocumentExportMenuProps extends MenuProps {
  binding: TiptapEditorExportBinding | (() => TiptapEditorExportBinding | null);
}

export function EditorDocumentExportMenu({ binding, ...menuProps }: EditorDocumentExportMenuProps) {
  const getContent = useMemo(() => createTiptapExportContentProvider(binding), [binding]);
  return <DocumentExportMenu getContent={getContent} {...menuProps} />;
}

export interface UseEditorDocumentExportOptions {
  binding: TiptapEditorExportBinding | (() => TiptapEditorExportBinding | null);
}

export function useEditorDocumentExport({
  binding,
}: UseEditorDocumentExportOptions): UseDocumentExportResult {
  const getContent = useMemo(() => createTiptapExportContentProvider(binding), [binding]);
  return useDocumentExport({ getContent });
}

export type { DocumentExportFormat, TiptapEditorExportBinding };
