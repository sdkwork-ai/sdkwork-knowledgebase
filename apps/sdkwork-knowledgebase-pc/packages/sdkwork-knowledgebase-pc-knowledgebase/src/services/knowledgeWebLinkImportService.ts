import {
  KnowledgebaseErrorCodes,
  requireHttpUrl,
  requireKnowledgebaseAppSdkHttpClient,
  requireRegisteredSpaceId,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import type { DocumentMeta } from './document';
import { invalidateKnowledgeBrowserNodeCacheForKbIds } from './knowledgeBrowserListService';
import { resolveIngestedDocument, waitForIngestJob } from './knowledgeIngestService';
import { placeDocumentInParentFolder } from './knowledgebaseDocumentApiBridge';

export function validateWebLinkUrl(raw: string): URL {
  return requireHttpUrl(raw);
}

function buildIdempotencyKey(spaceId: string, url: URL): string {
  // Per-attempt uniqueness: the ingest create-or-get deduplicates by key, and
  // a key without the query string (or a repeated import of the same URL)
  // would otherwise collide into the first job.
  const randomSuffix = Math.random().toString(36).slice(2, 10);
  return `pc-weblink-${spaceId}-${url.hostname}-${url.pathname}-${Date.now()}-${randomSuffix}`.slice(0, 128);
}

export async function importWebLinkToKnowledgeBase(params: {
  kbId: string;
  parentId?: string | null;
  url: string;
  title?: string;
}): Promise<DocumentMeta> {
  const validatedUrl = validateWebLinkUrl(params.url);
  const spaceId = requireRegisteredSpaceId(params.kbId);
  const title = params.title?.trim() || validatedUrl.hostname;

  const client = requireKnowledgebaseAppSdkHttpClient();
  const job = await client.knowledge.ingests.create({
    spaceId,
    title,
    sourceUrl: validatedUrl.toString(),
    idempotencyKey: buildIdempotencyKey(spaceId, validatedUrl),
  });

  const finalJob = job.state === 'succeeded' ? job : await waitForIngestJob(job.id);
  if (finalJob.state !== 'succeeded') {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.INGEST_FAILED);
  }

  const document = await resolveIngestedDocument(spaceId, title);
  const meta: DocumentMeta = {
    id: String(document.id),
    title: document.title,
    type: 'markdown',
    kbId: params.kbId,
    parentId: params.parentId ?? null,
    updatedAt: new Date().toISOString(),
    author: 'Knowledgebase',
    url: validatedUrl.toString(),
  };

  if (params.parentId?.trim()) {
    await placeDocumentInParentFolder(meta.id, params.kbId, params.parentId);
  }

  invalidateKnowledgeBrowserNodeCacheForKbIds(params.kbId);
  return meta;
}
