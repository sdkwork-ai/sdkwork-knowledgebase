import { requireKnowledgebaseAppSdkHttpClient } from 'sdkwork-knowledgebase-pc-core';

import { normalizeSdkWorkListPage } from './sdkWorkListPage';

/**
 * Flat note listing for the notes workspace. Reads the space's knowledge
 * documents directly (`documents.list`) instead of the drive browser tree so
 * manually created notes — which have no drive node — are always visible.
 */

export interface KnowledgeNoteSummary {
  id: string;
  title: string;
}

const MAX_LIST_PAGES = 5;
const PAGE_SIZE = 100;

export async function listSpaceNotes(spaceId: string): Promise<KnowledgeNoteSummary[]> {
  const client = requireKnowledgebaseAppSdkHttpClient();
  const notes: KnowledgeNoteSummary[] = [];
  let cursor: string | null = null;

  for (let page = 0; page < MAX_LIST_PAGES; page += 1) {
    const pageData = await client.knowledge.documents.list({
      spaceId,
      cursor: cursor ?? undefined,
      pageSize: PAGE_SIZE,
    });
    const normalized = normalizeSdkWorkListPage<{ id: string; title: string }>(pageData);
    for (const doc of normalized.items) {
      notes.push({ id: String(doc.id), title: String(doc.title ?? '') });
    }
    if (!normalized.hasMore || !normalized.nextCursor) {
      break;
    }
    cursor = normalized.nextCursor;
  }

  return notes;
}
