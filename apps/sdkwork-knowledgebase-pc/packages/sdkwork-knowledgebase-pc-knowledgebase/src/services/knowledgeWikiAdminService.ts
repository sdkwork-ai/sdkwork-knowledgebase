import {
  getKnowledgebaseAppSdkClient,
  type KnowledgeWikiPublication,
  type KnowledgeWikiPublicationStatus,
  type KnowledgeWikiSourceFile,
  type KnowledgeWikiSourceFileCommandResult,
  type KnowledgeWikiVisibility,
} from 'sdkwork-knowledgebase-pc-core';

import { normalizeSdkWorkListPage } from './sdkWorkListPage';

export type {
  KnowledgeWikiPublication,
  KnowledgeWikiPublicationStatus,
  KnowledgeWikiSourceFile,
  KnowledgeWikiVisibility,
};

export interface WikiSourceFilePage {
  items: KnowledgeWikiSourceFile[];
  nextCursor: string | null;
  hasMore: boolean;
}

/** Page size for the Wiki source file management table. */
export const WIKI_SOURCE_FILES_PAGE_SIZE = 20;

/** Publication statuses from which `activate` is accepted by the backend. */
export const WIKI_ACTIVATE_ELIGIBLE_STATUSES: ReadonlySet<KnowledgeWikiPublicationStatus> =
  new Set<KnowledgeWikiPublicationStatus>(['ready', 'paused']);

/** Publication statuses from which `pause` is accepted by the backend. */
export const WIKI_PAUSE_ELIGIBLE_STATUSES: ReadonlySet<KnowledgeWikiPublicationStatus> =
  new Set<KnowledgeWikiPublicationStatus>(['active', 'degraded']);

/** Publication statuses under which source files may be published. */
export const WIKI_PUBLISH_ELIGIBLE_STATUSES: ReadonlySet<KnowledgeWikiPublicationStatus> =
  new Set<KnowledgeWikiPublicationStatus>(['ready', 'active', 'paused']);

function wikiKnowledgeApi() {
  return getKnowledgebaseAppSdkClient().client.knowledge;
}

/**
 * RFC 4122 idempotency key for Wiki command endpoints.
 * Falls back to a timestamp-random token when WebCrypto is unavailable.
 */
export function newWikiIdempotencyKey(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `kb-wiki-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`;
}

export function isWikiPublishRowEligible(file: KnowledgeWikiSourceFile): boolean {
  return (
    file.sourceState === 'ready'
    && file.canonicalRoute !== null
    && file.canonicalRoute !== undefined
    && file.indexState !== 'error'
  );
}

export async function retrieveWikiPublication(
  spaceId: string,
): Promise<KnowledgeWikiPublication> {
  return wikiKnowledgeApi().wikiPublications.retrieve(String(spaceId));
}

export async function activateWikiPublication(
  spaceId: string,
  expectedVersion: string,
): Promise<KnowledgeWikiPublication> {
  return wikiKnowledgeApi().wikiPublications.activate(
    String(spaceId),
    { expectedVersion },
    { idempotencyKey: newWikiIdempotencyKey() },
  );
}

export async function pauseWikiPublication(
  spaceId: string,
  expectedVersion: string,
): Promise<KnowledgeWikiPublication> {
  return wikiKnowledgeApi().wikiPublications.pause(
    String(spaceId),
    { expectedVersion },
    { idempotencyKey: newWikiIdempotencyKey() },
  );
}

export async function listWikiSourceFiles(
  spaceId: string,
  cursor: string | null,
  pageSize: number,
): Promise<WikiSourceFilePage> {
  const params = cursor === null ? { pageSize } : { cursor, pageSize };
  const page = normalizeSdkWorkListPage<KnowledgeWikiSourceFile>(
    await wikiKnowledgeApi().wikiSourceFiles.list(String(spaceId), params),
  );
  return {
    items: page.items,
    nextCursor: page.nextCursor,
    hasMore: page.hasMore,
  };
}

export async function publishWikiSourceFile(
  spaceId: string,
  sourceFileUuid: string,
  visibility: 'public' | 'unlisted',
  expectedPublicationVersion: string,
  expectedPageVersion: string,
): Promise<KnowledgeWikiSourceFileCommandResult> {
  return wikiKnowledgeApi().wikiSourceFiles.publish(
    String(spaceId),
    String(sourceFileUuid),
    { visibility, expectedPublicationVersion, expectedPageVersion },
    { idempotencyKey: newWikiIdempotencyKey() },
  );
}

export async function unpublishWikiSourceFile(
  spaceId: string,
  sourceFileUuid: string,
  expectedPublicationVersion: string,
  expectedPageVersion: string,
): Promise<KnowledgeWikiSourceFileCommandResult> {
  return wikiKnowledgeApi().wikiSourceFiles.unpublish(
    String(spaceId),
    String(sourceFileUuid),
    { expectedPublicationVersion, expectedPageVersion },
    { idempotencyKey: newWikiIdempotencyKey() },
  );
}

export async function changeWikiSourceFileVisibility(
  spaceId: string,
  sourceFileUuid: string,
  visibility: KnowledgeWikiVisibility,
  expectedPublicationVersion: string,
  expectedPageVersion: string,
): Promise<KnowledgeWikiSourceFileCommandResult> {
  return wikiKnowledgeApi().wikiSourceFiles.visibility.update(
    String(spaceId),
    String(sourceFileUuid),
    { visibility, expectedPublicationVersion, expectedPageVersion },
    { idempotencyKey: newWikiIdempotencyKey() },
  );
}
