import { throwKnowledgebaseError } from '../errors/knowledgebaseAppError';
import { KnowledgebaseErrorCodes } from '../errors/knowledgebaseErrorCodes';
import { invokeDesktopCommand, isTauriDesktopRuntime } from './tauriBridge';

const MAX_DESKTOP_EXPORT_BYTES = 64 * 1024 * 1024;
const MAX_DESKTOP_RESOURCE_BYTES = 32 * 1024 * 1024;

export interface BinaryResourcePayload {
  dataBase64: string;
  mimeType?: string | null;
  byteLength: number;
}

export interface HostAdapter {
  isNativeHost: boolean;
  windowControl(action: WindowControlAction): Promise<void>;
  openExternal(url: string): Promise<void>;
  writeTextToClipboard(text: string): Promise<void>;
  fetchBinaryResource(url: string): Promise<BinaryResourcePayload>;
  readLocalResource(path: string): Promise<BinaryResourcePayload>;
  saveBinaryResource(suggestedName: string, bytes: Uint8Array): Promise<boolean>;
  saveExportFile(options: {
    suggestedName: string;
    bytes: Uint8Array;
    mode: 'downloads' | 'saveAs';
  }): Promise<NativeSaveExportFileResult>;
  revealExportFile(path: string): Promise<void>;
  openExportFile(path: string): Promise<void>;
  locateExportFile(fileName: string): Promise<string | null>;
}

export interface NativeSaveExportFileResult {
  saved: boolean;
  cancelled: boolean;
  path?: string | null;
  mode: 'downloads' | 'saveAs';
}

export type WindowControlAction = 'minimize' | 'maximize' | 'unmaximize' | 'close' | 'show';

function assertSafeExternalUrl(url: string): void {
  const parsed = new URL(url);
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.URL_INVALID_SCHEME);
  }
}

function encodeBytesBase64(bytes: Uint8Array): string {
  if (bytes.byteLength > MAX_DESKTOP_EXPORT_BYTES) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PAYLOAD_TOO_LARGE);
  }
  let binary = '';
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    const chunk = bytes.subarray(index, index + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

export function decodeBinaryResourcePayload(payload: BinaryResourcePayload): Uint8Array {
  const maxEncodedBytes = Math.ceil(MAX_DESKTOP_RESOURCE_BYTES / 3) * 4;
  if (
    payload.byteLength < 0
    || payload.byteLength > MAX_DESKTOP_RESOURCE_BYTES
    || payload.dataBase64.length > maxEncodedBytes
  ) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PAYLOAD_TOO_LARGE);
  }
  const binary = atob(payload.dataBase64);
  if (binary.length !== payload.byteLength || binary.length > MAX_DESKTOP_RESOURCE_BYTES) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PAYLOAD_TOO_LARGE);
  }
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

export function createHostAdapter(): HostAdapter {
  return {
    get isNativeHost() {
      return isTauriDesktopRuntime();
    },
    async windowControl(action) {
      if (!isTauriDesktopRuntime()) {
        return;
      }
      await invokeDesktopCommand('window_control', { request: { action } });
    },
    async openExternal(url) {
      assertSafeExternalUrl(url);
      if (isTauriDesktopRuntime()) {
        await invokeDesktopCommand('open_external_url', { request: { url } });
        return;
      }
      globalThis.open?.(url, '_blank', 'noopener,noreferrer');
    },
    async writeTextToClipboard(text) {
      await navigator.clipboard?.writeText(text);
    },
    async fetchBinaryResource(url) {
      if (!isTauriDesktopRuntime()) {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.DESKTOP_ONLY);
      }
      return invokeDesktopCommand<BinaryResourcePayload>('fetch_binary_resource', {
        request: { url },
      });
    },
    async readLocalResource(path) {
      if (!isTauriDesktopRuntime()) {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.DESKTOP_ONLY);
      }
      return invokeDesktopCommand<BinaryResourcePayload>('read_local_resource', {
        request: { path },
      });
    },
    async saveBinaryResource(suggestedName, bytes) {
      if (!isTauriDesktopRuntime()) {
        const blob = new Blob([bytes as BlobPart], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement('a');
        anchor.href = url;
        anchor.download = suggestedName;
        anchor.click();
        URL.revokeObjectURL(url);
        return true;
      }
      return invokeDesktopCommand<boolean>('save_binary_resource', {
        request: {
          suggestedName,
          dataBase64: encodeBytesBase64(bytes),
        },
      });
    },
    async saveExportFile({ suggestedName, bytes, mode }) {
      if (!isTauriDesktopRuntime()) {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.DESKTOP_ONLY);
      }
      const response = await invokeDesktopCommand<NativeSaveExportFileResult>('save_export_file', {
        request: {
          suggestedName,
          dataBase64: encodeBytesBase64(bytes),
          mode,
        },
      });
      return response;
    },
    async revealExportFile(path) {
      if (!isTauriDesktopRuntime()) {
        return;
      }
      await invokeDesktopCommand('reveal_export_file', {
        request: { path },
      });
    },
    async openExportFile(path) {
      if (!isTauriDesktopRuntime()) {
        return;
      }
      await invokeDesktopCommand('open_export_file', {
        request: { path },
      });
    },
    async locateExportFile(fileName) {
      if (!isTauriDesktopRuntime()) {
        return null;
      }
      try {
        return await invokeDesktopCommand<string>('locate_export_file', {
          request: { fileName },
        });
      } catch {
        return null;
      }
    },
  };
}
