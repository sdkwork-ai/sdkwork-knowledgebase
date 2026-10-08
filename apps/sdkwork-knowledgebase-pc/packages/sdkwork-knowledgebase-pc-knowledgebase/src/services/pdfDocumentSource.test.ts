import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  KnowledgebaseErrorCodes,
  type HostAdapter,
} from 'sdkwork-knowledgebase-pc-core';
import {
  loadPdfSourceFallback,
  normalizePdfUrl,
  resolveInitialPdfSource,
} from './pdfDocumentSource';

function createHostAdapter(overrides: Partial<HostAdapter> = {}): HostAdapter {
  return {
    isNativeHost: false,
    windowControl: vi.fn(),
    openExternal: vi.fn(),
    writeTextToClipboard: vi.fn(),
    fetchBinaryResource: vi.fn(),
    readLocalResource: vi.fn(),
    saveBinaryResource: vi.fn(),
    saveExportFile: vi.fn(),
    revealExportFile: vi.fn(),
    openExportFile: vi.fn(),
    locateExportFile: vi.fn(),
    ...overrides,
  };
}

describe('loadPdfSourceFallback', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('does not use browser raw fetch when no native binary-resource port is available', async () => {
    const fetchSpy = vi.fn(async () => {
      throw new Error('raw fetch must not be called');
    });
    vi.stubGlobal('fetch', fetchSpy);
    vi.stubGlobal('window', { location: { origin: 'https://app.local' } });

    await expect(
      loadPdfSourceFallback('https://example.com/guide.pdf', createHostAdapter()),
    ).rejects.toMatchObject({
      code: KnowledgebaseErrorCodes.DESKTOP_ONLY,
    });
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('loads remote PDF bytes through the native host binary-resource port', async () => {
    const host = createHostAdapter({
      isNativeHost: true,
      fetchBinaryResource: vi.fn(async () => ({
        dataBase64: 'JVBERi0x',
        mimeType: 'application/pdf',
        byteLength: 6,
      })),
    });

    const source = await loadPdfSourceFallback('https://example.com/guide.pdf', host);

    expect(source.kind).toBe('bytes');
    expect(Array.from(source.kind === 'bytes' ? source.data : new Uint8Array())).toEqual([
      37,
      80,
      68,
      70,
      45,
      49,
    ]);
    expect(host.fetchBinaryResource).toHaveBeenCalledWith('https://example.com/guide.pdf');
  });
});

describe('normalizePdfUrl', () => {
  it('keeps direct http, blob, and data URLs untouched', () => {
    expect(normalizePdfUrl('https://example.com/guide.pdf')).toBe('https://example.com/guide.pdf');
    expect(normalizePdfUrl('blob:https://app.local/1f2e')).toBe('blob:https://app.local/1f2e');
    expect(
      normalizePdfUrl('data:application/pdf;base64,JVBERi0x'),
    ).toBe('data:application/pdf;base64,JVBERi0x');
  });

  it('resolves relative and protocol-relative paths against the app origin', () => {
    expect(normalizePdfUrl('docs/guide.pdf')).toBe('http://localhost/docs/guide.pdf');
    expect(normalizePdfUrl('//cdn.example.com/guide.pdf')).toBe('https://cdn.example.com/guide.pdf');
  });

  it('rejects javascript-style schemes instead of returning them as absolute URLs', () => {
    try {
      normalizePdfUrl('javascript:alert(1)');
      expect.unreachable('normalizePdfUrl must reject javascript: URLs');
    } catch (error) {
      expect((error as { code?: string }).code).toBe(KnowledgebaseErrorCodes.URL_INVALID_SCHEME);
    }
    try {
      normalizePdfUrl('vbscript:msgbox(1)');
      expect.unreachable('normalizePdfUrl must reject vbscript: URLs');
    } catch (error) {
      expect((error as { code?: string }).code).toBe(KnowledgebaseErrorCodes.URL_INVALID_SCHEME);
    }
  });
});

describe('resolveInitialPdfSource', () => {
  it('returns null instead of throwing for rejected schemes so the viewer degrades gracefully', () => {
    expect(resolveInitialPdfSource('javascript:alert(1)')).toBeNull();
    expect(resolveInitialPdfSource('file:///etc/passwd.pdf')).toBeNull();
    expect(resolveInitialPdfSource(undefined)).toBeNull();
  });

  it('resolves safe URLs into a url source', () => {
    expect(resolveInitialPdfSource('https://example.com/guide.pdf')).toEqual({
      kind: 'url',
      url: 'https://example.com/guide.pdf',
    });
  });
});
