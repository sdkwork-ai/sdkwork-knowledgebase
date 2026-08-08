import { sanitizeEditorHtml } from '@sdkwork/sdkwork-knowledgebase-pc-commons/htmlSanitizer';
import { getDocumentExportCapabilities } from './documentExportCapabilities';
import { KnowledgebaseErrorCodes, throwKnowledgebaseError } from 'sdkwork-knowledgebase-pc-core';
import { tryNativeDocumentPdfExport } from './documentExportNative';
import { prepareExportHtml, prepareExportImages, stripHtmlText } from './exportContentUtils';
import { showExportProgress } from './exportProgress';
import { detectBrowserFamily, resolveExportCanvasScale } from './exportRuntime';
import {
  buildExportFileName,
  EXPORT_MIME_TYPES,
  persistExportFile,
} from './documentExportSave';
import type {
  DocumentExportContent,
  DocumentExportResult,
  ExportSaveMode,
} from './types';

export type { DocumentExportResult };

const EXPORT_PROSE_STYLES = `
  .prose h1, .prose h2, .prose h3 { color: #09090b; font-weight: 700; margin-top: 1.6em; margin-bottom: 0.6em; }
  .prose h1 { font-size: 1.5em; letter-spacing: -0.025em; }
  .prose h2 { font-size: 1.25em; border-bottom: 1px solid #f4f4f5; padding-bottom: 0.3em; letter-spacing: -0.015em; }
  .prose h3 { font-size: 1.1em; }
  .prose p { margin-bottom: 1.1em; margin-top: 0; color: #27272a; line-height: 1.65; }
  .prose blockquote { border-left: 4.5px solid #4f46e5; padding-left: 1.1rem; color: #71717a; font-style: italic; margin-left: 0; margin-bottom: 1.1em; }
  .prose pre { background: #f8f8f9; padding: 1rem; border-radius: 8px; overflow-x: auto; font-family: monospace; font-size: 0.85em; border: 1px solid #e4e4e7; margin-bottom: 1.1em; white-space: pre-wrap; word-break: break-word; }
  .prose code { background: #f4f4f5; padding: 0.15rem 0.3rem; border-radius: 4px; font-family: monospace; font-size: 0.9em; color: #09090b; }
  .prose ul { list-style-type: disc; padding-left: 1.5rem; margin-bottom: 1.1em; }
  .prose ol { list-style-type: decimal; padding-left: 1.5rem; margin-bottom: 1.1em; }
  .prose li { margin-bottom: 0.5em; color: #27272a; }
  .prose img { max-width: 100%; border-radius: 8px; margin: 1em 0; border: 1px solid #f4f4f5; }
  .prose table { width: 100%; border-collapse: collapse; margin-bottom: 1.1em; }
  .prose th, .prose td { border: 1px solid #e4e4e7; padding: 0.5rem 0.75rem; text-align: left; }
  .prose th { background: #f8f8f9; font-weight: 600; }
`;

export interface BuildExportContainerOptions {
  htmlContent: string;
  /** Flat layout without card chrome — better for PDF pagination. */
  flat?: boolean;
}

export function buildExportContainer({
  htmlContent,
  flat = false,
}: BuildExportContainerOptions): HTMLDivElement {
  const container = document.createElement('div');
  container.style.position = 'absolute';
  container.style.top = '-9999px';
  container.style.left = '-9999px';
  container.style.width = '760px';
  container.className = flat
    ? 'bg-white text-zinc-800 p-10 font-sans'
    : 'bg-white text-zinc-800 p-10 font-sans rounded-xl border border-zinc-100 shadow-lg';

  const style = document.createElement('style');
  style.innerHTML = EXPORT_PROSE_STYLES;
  container.appendChild(style);

  const content = document.createElement('div');
  content.className = 'prose max-w-none text-[14.5px] leading-relaxed text-zinc-700 space-y-4';
  content.innerHTML = sanitizeEditorHtml(htmlContent);
  container.appendChild(content);

  return container;
}

async function loadHtml2Canvas(): Promise<
  (element: HTMLElement, options?: Record<string, unknown>) => Promise<HTMLCanvasElement>
> {
  try {
    return (await import('html2canvas-pro')).default;
  } catch {
    return (await import('html2canvas')).default;
  }
}

const MAX_CANVAS_HEIGHT_PX = 16384;
const SLICE_CSS_HEIGHT = 3600;

export { MAX_CANVAS_HEIGHT_PX };

function buildHtml2CanvasOptions(
  scale: number,
  slice?: { y: number; height: number; windowHeight: number },
): Record<string, unknown> {
  const browser = detectBrowserFamily();
  const options: Record<string, unknown> = {
    useCORS: true,
    allowTaint: true,
    backgroundColor: '#ffffff',
    scale,
    logging: false,
  };

  // Safari / Firefox often render more reliably without foreignObjectRendering.
  if (browser === 'safari' || browser === 'firefox') {
    options.foreignObjectRendering = false;
  }

  if (slice) {
    options.y = slice.y;
    options.height = slice.height;
    options.windowHeight = slice.windowHeight;
  }

  return options;
}

function stitchCanvasSlices(slices: HTMLCanvasElement[]): HTMLCanvasElement {
  if (slices.length === 1) {
    return slices[0];
  }

  const width = slices[0]?.width ?? 0;
  const height = slices.reduce((total, slice) => total + slice.height, 0);
  const stitched = document.createElement('canvas');
  stitched.width = width;
  stitched.height = height;

  const context = stitched.getContext('2d');
  if (!context) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.EXPORT_RENDER_FAILED);
  }

  context.fillStyle = '#ffffff';
  context.fillRect(0, 0, width, height);

  let offsetY = 0;
  for (const slice of slices) {
    context.drawImage(slice, 0, offsetY);
    offsetY += slice.height;
  }

  return stitched;
}

async function renderCanvasSlices(
  html2canvas: (element: HTMLElement, options?: Record<string, unknown>) => Promise<HTMLCanvasElement>,
  container: HTMLDivElement,
  scale: number,
): Promise<{ canvas: HTMLCanvasElement; usedTiledRender: boolean }> {
  const totalCssHeight = container.scrollHeight;
  const projectedHeight = totalCssHeight * scale;

  if (projectedHeight <= MAX_CANVAS_HEIGHT_PX) {
    const canvas = await html2canvas(container, buildHtml2CanvasOptions(scale));
    return { canvas, usedTiledRender: false };
  }

  const slices: HTMLCanvasElement[] = [];
  let offsetY = 0;
  const sliceCount = Math.ceil(totalCssHeight / SLICE_CSS_HEIGHT);
  let sliceIndex = 0;
  while (offsetY < totalCssHeight) {
    sliceIndex += 1;
    if (sliceCount > 1) {
      showExportProgress(`正在分段渲染 (${sliceIndex}/${sliceCount})...`);
    }
    const sliceHeight = Math.min(SLICE_CSS_HEIGHT, totalCssHeight - offsetY);
    const slice = await html2canvas(
      container,
      buildHtml2CanvasOptions(scale, {
        y: offsetY,
        height: sliceHeight,
        windowHeight: sliceHeight,
      }),
    );
    slices.push(slice);
    offsetY += sliceHeight;
  }

  const canvas = stitchCanvasSlices(slices);
  return { canvas, usedTiledRender: true };
}

export async function renderExportCanvas(
  container: HTMLDivElement,
  scale = resolveExportCanvasScale(),
): Promise<{ canvas: HTMLCanvasElement; imageLoadFailures: number; usedTiledRender: boolean }> {
  document.body.appendChild(container);
  const imageLoadFailures = await prepareExportImages(container);
  await new Promise((resolve) => setTimeout(resolve, 300));

  try {
    const html2canvas = await loadHtml2Canvas();
    return {
      ...(await renderCanvasSlices(html2canvas, container, scale)),
      imageLoadFailures,
    };
  } finally {
    document.body.removeChild(container);
  }
}

export async function canvasToPdfBytes(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  const { jsPDF } = await import('jspdf');
  const pdf = new jsPDF({ orientation: 'portrait', unit: 'mm', format: 'a4' });
  const pageWidth = pdf.internal.pageSize.getWidth();
  const pageHeight = pdf.internal.pageSize.getHeight();
  const pxPerMm = canvas.width / pageWidth;
  const pageHeightPx = pageHeight * pxPerMm;

  let renderedHeight = 0;
  let pageIndex = 0;

  while (renderedHeight < canvas.height) {
    const sliceHeight = Math.min(pageHeightPx, canvas.height - renderedHeight);
    const pageCanvas = document.createElement('canvas');
    pageCanvas.width = canvas.width;
    pageCanvas.height = sliceHeight;

    const ctx = pageCanvas.getContext('2d');
    if (!ctx) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.EXPORT_RENDER_FAILED);
    }

    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, pageCanvas.width, pageCanvas.height);
    ctx.drawImage(
      canvas,
      0,
      renderedHeight,
      canvas.width,
      sliceHeight,
      0,
      0,
      canvas.width,
      sliceHeight,
    );

    const imgData = pageCanvas.toDataURL('image/jpeg', 0.92);
    const sliceImgHeight = (sliceHeight * pageWidth) / canvas.width;

    if (pageIndex > 0) {
      pdf.addPage();
    }
    pdf.addImage(imgData, 'JPEG', 0, 0, pageWidth, sliceImgHeight);

    renderedHeight += sliceHeight;
    pageIndex += 1;
  }

  const buffer = pdf.output('arraybuffer');
  return new Uint8Array(buffer);
}

export async function exportDocumentAsPdf(
  content: DocumentExportContent,
  mode: ExportSaveMode,
): Promise<DocumentExportResult> {
  const fileName = buildExportFileName(content.title || 'document', 'pdf');
  const expectedEngine = getDocumentExportCapabilities(content.sourceKind).pdfEngine;
  const nativeResult = await tryNativeDocumentPdfExport(content);
  if (nativeResult) {
    showExportProgress(mode === 'saveAs' ? '请选择 PDF 保存位置...' : '正在保存 PDF...');
    const save = await persistExportFile({
      bytes: nativeResult.bytes,
      suggestedName: fileName,
      mode,
      mimeType: EXPORT_MIME_TYPES.pdf,
    });
    return {
      save,
      pdfEngine: nativeResult.engine,
      usedCanvasFallback: false,
    };
  }

  showExportProgress('正在渲染 PDF 内容...');
  const preparedHtml = await prepareExportHtml(content.html);
  const container = buildExportContainer({
    htmlContent: preparedHtml,
    flat: true,
  });
  const { canvas, imageLoadFailures, usedTiledRender } = await renderExportCanvas(container);
  showExportProgress(mode === 'saveAs' ? '请选择 PDF 保存位置...' : '正在保存 PDF...');
  const pdfBytes = await canvasToPdfBytes(canvas);
  const save = await persistExportFile({
    bytes: pdfBytes,
    suggestedName: fileName,
    mode,
    mimeType: EXPORT_MIME_TYPES.pdf,
  });
  return {
    save,
    pdfEngine: 'canvas',
    usedCanvasFallback: expectedEngine !== 'canvas',
    imageLoadFailures,
    usedTiledRender,
    canvasMayBeClipped: canvas.height > MAX_CANVAS_HEIGHT_PX,
  };
}

export async function exportDocumentAsWord(
  content: { title: string; html: string },
  mode: ExportSaveMode,
): Promise<DocumentExportResult> {
  showExportProgress(mode === 'saveAs' ? '请选择 Word 保存位置...' : '正在保存 Word...');
  const preparedHtml = await prepareExportHtml(content.html);
  if (!stripHtmlText(preparedHtml)) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.EXPORT_CONTENT_EMPTY);
  }
  const header =
    "<html xmlns:o='urn:schemas-microsoft-com:office:office' xmlns:w='urn:schemas-microsoft-com:office:word' xmlns='http://www.w3.org/TR/REC-html40'><head><meta charset='utf-8'><title>Export HTML to Word</title></head><body>";
  const footer = '</body></html>';
  const html = header + preparedHtml + footer;
  const bytes = new TextEncoder().encode(`\ufeff${html}`);
  const save = await persistExportFile({
    bytes,
    suggestedName: buildExportFileName(content.title || 'document', 'doc'),
    mode,
    mimeType: EXPORT_MIME_TYPES.word,
  });
  return { save };
}

export async function exportDocumentAsMarkdown(
  content: { title: string; markdown: string },
  mode: ExportSaveMode,
): Promise<DocumentExportResult> {
  showExportProgress(mode === 'saveAs' ? '请选择 Markdown 保存位置...' : '正在保存 Markdown...');
  const markdown = content.markdown.trim();
  if (!markdown) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.EXPORT_CONTENT_EMPTY);
  }
  const bytes = new TextEncoder().encode(`\ufeff${markdown}`);
  const save = await persistExportFile({
    bytes,
    suggestedName: buildExportFileName(content.title || 'document', 'md'),
    mode,
    mimeType: EXPORT_MIME_TYPES.markdown,
  });
  return { save };
}

export async function exportDocumentAsImage(
  content: { title: string; html: string },
  mode: ExportSaveMode,
): Promise<DocumentExportResult> {
  showExportProgress('正在渲染图片，请稍等...');
  const preparedHtml = await prepareExportHtml(content.html);
  const container = buildExportContainer({
    htmlContent: preparedHtml,
  });
  const { canvas, imageLoadFailures, usedTiledRender } = await renderExportCanvas(container);
  const canvasMayBeClipped = canvas.height > MAX_CANVAS_HEIGHT_PX;
  if (canvasMayBeClipped) {
    console.warn(
      `[DocumentExport] export canvas height ${canvas.height}px exceeds ${MAX_CANVAS_HEIGHT_PX}px; output may be clipped.`,
    );
  }
  showExportProgress(mode === 'saveAs' ? '请选择图片保存位置...' : '正在保存图片...');
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((value) => {
      if (!value) {
        reject(new Error('Failed to render export image.'));
        return;
      }
      resolve(value);
    }, 'image/png');
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const save = await persistExportFile({
    bytes,
    suggestedName: buildExportFileName(content.title || 'document', 'png'),
    mode,
    mimeType: EXPORT_MIME_TYPES.image,
  });
  return { save, imageLoadFailures, usedTiledRender, canvasMayBeClipped };
}
