import { requireKnowledgebaseAppSdkHttpClient } from 'sdkwork-knowledgebase-pc-core';

import { normalizeSdkWorkListPage } from './sdkWorkListPage';

/**
 * Flat note listing for the notes workspace. Reads the space's knowledge
 * documents directly (`documents.list`) instead of the drive browser tree so
 * manually created notes — which have no drive node — are always visible.
 *
 * Only authored text documents qualify as notes: file-backed documents
 * (uploads with a drive node) and binary media must never appear here, or the
 * editor would offer to "edit" a PDF/image and autosave would overwrite the
 * file's indexed content with richtext.
 */

export interface KnowledgeNoteSummary {
  id: string;
  title: string;
}

interface NoteCandidateDocument {
  id: number | string;
  title?: string | null;
  mimeType?: string | null;
  originalFileDriveNodeId?: string | null;
}

const MAX_LIST_PAGES = 5;
const PAGE_SIZE = 100;

function isAuthoredTextDocument(doc: NoteCandidateDocument): boolean {
  if (doc.originalFileDriveNodeId) {
    return false;
  }
  const mimeType = doc.mimeType?.trim().toLowerCase();
  return !mimeType || mimeType.startsWith('text/');
}

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
    const normalized = normalizeSdkWorkListPage<NoteCandidateDocument>(pageData);
    for (const doc of normalized.items) {
      if (!isAuthoredTextDocument(doc)) {
        continue;
      }
      notes.push({ id: String(doc.id), title: String(doc.title ?? '') });
    }
    if (!normalized.hasMore || !normalized.nextCursor) {
      break;
    }
    cursor = normalized.nextCursor;
  }

  return notes;
}
