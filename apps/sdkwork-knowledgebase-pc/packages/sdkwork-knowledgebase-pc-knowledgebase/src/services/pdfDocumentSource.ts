import {
  KnowledgebaseErrorCodes,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';
import {
  decodeBinaryResourcePayload,
  type HostAdapter,
} from 'sdkwork-knowledgebase-pc-core';

export type PdfDocumentSource =
  | { kind: 'url'; url: string }
  | { kind: 'bytes'; data: Uint8Array };

/** Inline or navigable URL sources that react-pdf can load directly. */
export function isDirectPdfUrl(source: string): boolean {
  const trimmed = source.trim();
  return (
    trimmed.startsWith('blob:') ||
    trimmed.startsWith('data:') ||
    trimmed.startsWith('http://') ||
    trimmed.startsWith('https://') ||
    trimmed.startsWith('//') ||
    trimmed.startsWith('/')
  );
}

/** OS filesystem paths that must be read through the desktop host. */
export function isLocalFilePath(source: string): boolean {
  const trimmed = source.trim();
  if (!trimmed) return false;
  if (trimmed.startsWith('file://')) return true;
  if (/^[a-zA-Z]:[\\/]/.test(trimmed)) return true;
  if (trimmed.startsWith('\\\\')) return true;

  // Unix absolute paths outside SPA asset routes.
  if (trimmed.startsWith('/') && !isAppAssetPath(trimmed)) {
    return /^\/(?:Users|home|var|tmp|opt|mnt|private|Volumes)\//.test(trimmed);
  }

  return false;
}

function isAppAssetPath(source: string): boolean {
  return /^\/(?:samples|assets|static|app|api|backend|knowledge|admin|files|media)(?:\/|$)/i.test(
    source.trim()
  );
}

/** Final schemes a normalized PDF URL may carry; everything else is rejected. */
const ALLOWED_PDF_URL_PROTOCOLS = new Set(['http:', 'https:', 'blob:', 'data:']);

function assertAllowedPdfUrl(url: string): string {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch (error) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.URL_INVALID_SCHEME, {
      cause: error instanceof Error ? error.message : String(error),
    });
  }
  if (!ALLOWED_PDF_URL_PROTOCOLS.has(parsed.protocol)) {
    // A `javascript:`-style value must not survive normalization as a URL the
    // renderer (or a fallback fetch) will consume.
    throwKnowledgebaseError(KnowledgebaseErrorCodes.URL_INVALID_SCHEME, {
      cause: parsed.protocol,
    });
  }
  return url;
}

export function normalizePdfUrl(source: string): string {
  const trimmed = source.trim();
  if (!trimmed) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PDF_URL_REQUIRED);
  }

  if (
    trimmed.startsWith('blob:') ||
    trimmed.startsWith('data:') ||
    trimmed.startsWith('http://') ||
    trimmed.startsWith('https://')
  ) {
    return assertAllowedPdfUrl(trimmed);
  }

  if (trimmed.startsWith('//')) {
    return assertAllowedPdfUrl(`${globalThis.location?.protocol ?? 'https:'}${trimmed}`);
  }

  let resolved: string;
  try {
    resolved = new URL(trimmed, globalThis.location?.origin ?? 'http://localhost').href;
  } catch (error) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.URL_INVALID_SCHEME, {
      cause: error instanceof Error ? error.message : String(error),
    });
  }
  return assertAllowedPdfUrl(resolved);
}

export function resolveInitialPdfSource(source: string | undefined): PdfDocumentSource | null {
  const trimmed = source?.trim();
  if (!trimmed) return null;

  if (isLocalFilePath(trimmed)) {
    return null;
  }

  // Direct urls and bare relative paths such as "docs/guide.pdf" both resolve
  // through normalizePdfUrl. A rejected scheme degrades to null so the
  // react-pdf consumer shows "Unsupported PDF source." instead of throwing
  // inside the render/effect path.
  try {
    return { kind: 'url', url: normalizePdfUrl(trimmed) };
  } catch {
    return null;
  }
}

async function fetchViaNativeHost(url: string, host: HostAdapter): Promise<Uint8Array> {
  const payload = await host.fetchBinaryResource(url);
  return decodeBinaryResourcePayload(payload);
}

async function readLocalPath(path: string, host: HostAdapter): Promise<Uint8Array> {
  const payload = await host.readLocalResource(path);
  return decodeBinaryResourcePayload(payload);
}

/** Load local filesystem PDF bytes (desktop host required). */
export async function loadLocalPdfSource(
  source: string,
  host: HostAdapter
): Promise<PdfDocumentSource> {
  if (!host.isNativeHost) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DESKTOP_ONLY);
  }
  return { kind: 'bytes', data: await readLocalPath(source, host) };
}

/**
 * Fallback when direct URL rendering fails (CORS, blocked network, etc.).
 * Keeps URL as the primary path; only escalates when display fails.
 */
export async function loadPdfSourceFallback(
  source: string,
  host: HostAdapter
): Promise<PdfDocumentSource> {
  const trimmed = source.trim();
  if (!trimmed) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PDF_URL_REQUIRED);
  }

  if (isLocalFilePath(trimmed)) {
    return loadLocalPdfSource(trimmed, host);
  }

  const url = normalizePdfUrl(trimmed);

  if (host.isNativeHost) {
    try {
      return { kind: 'bytes', data: await fetchViaNativeHost(url, host) };
    } catch (nativeError) {
      console.warn('Native PDF fetch failed, retrying via WebView fetch.', nativeError);
    }
  }

  throwKnowledgebaseError(KnowledgebaseErrorCodes.DESKTOP_ONLY);
}

export function toReactPdfFile(source: PdfDocumentSource): string | Uint8Array {
  if (source.kind === 'url') {
    return source.url;
  }
  return source.data;
}
