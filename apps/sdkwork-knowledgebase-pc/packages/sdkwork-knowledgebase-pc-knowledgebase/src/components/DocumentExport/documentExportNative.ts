import { getDocumentExportCapabilities } from './documentExportCapabilities';
import { isBlank } from '@sdkwork/utils';
import { prepareExportHtml } from './exportContentUtils';
import { getTauriInvoke, invokeTauriCommand } from './exportRuntime';
import type { DocumentExportContent, DocumentPdfExportEngine } from './types';

interface BinaryResourcePayload {
  dataBase64: string;
  mimeType?: string | null;
  byteLength?: number;
}

function decodeBinaryResourcePayload(payload: BinaryResourcePayload): Uint8Array {
  const binary = atob(payload.dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

async function decodeExportPayload(payload: BinaryResourcePayload): Promise<Uint8Array> {
  try {
    const { decodeBinaryResourcePayload: decodeFromCore } = await import('sdkwork-knowledgebase-pc-core');
    return decodeFromCore({
      dataBase64: payload.dataBase64,
      mimeType: payload.mimeType ?? undefined,
      byteLength: payload.byteLength ?? 0,
    });
  } catch {
    return decodeBinaryResourcePayload(payload);
  }
}

export interface NativeDocumentPdfExportResult {
  bytes: Uint8Array;
  engine: DocumentPdfExportEngine;
}

export async function tryNativeDocumentPdfExport(
  content: DocumentExportContent,
): Promise<NativeDocumentPdfExportResult | null> {
  if (!getTauriInvoke()) {
    return null;
  }

  const capabilities = getDocumentExportCapabilities(content.sourceKind);
  if (capabilities.pdfEngine === 'canvas') {
    return null;
  }

  if (isBlank(content.html) && isBlank(content.markdown)) {
    return null;
  }

  const preparedHtml = await prepareExportHtml(content.html);
  const request = {
    title: content.title,
    html: preparedHtml,
    markdown: content.markdown ?? null,
    sourceKind: content.sourceKind ?? null,
  };

  try {
    let payload = await invokeTauriCommand<BinaryResourcePayload>('export_document_pdf', request);
    if (!payload) {
      return null;
    }

    const bytes = await decodeExportPayload(payload);
    if (bytes.length === 0) {
      return null;
    }

    return {
      bytes,
      engine: capabilities.pdfEngine,
    };
  } catch (error) {
    console.warn('[DocumentExport] native PDF export unavailable, falling back to canvas.', error);
    return null;
  }
}

export function isNativeDocumentPdfAvailable(sourceKind?: DocumentExportContent['sourceKind']): boolean {
  if (!getTauriInvoke()) {
    return false;
  }
  return getDocumentExportCapabilities(sourceKind).pdfEngine !== 'canvas';
}
