import type { DriveNode, KnowledgeBrowserNode } from 'sdkwork-knowledgebase-pc-core';
import { isBlank } from '@sdkwork/utils';
import {
  KnowledgebaseErrorCodes,
  parseKnowledgeSpaceId,
  requireDriveApiClient,
  requireKnowledgebaseAppSdkHttpClient,
  requireRegisteredSpaceId,
  throwKnowledgebaseError} from 'sdkwork-knowledgebase-pc-core';

import { invalidateKnowledgeBrowserNodeCacheForKbIds } from './knowledgeBrowserListService';
import { placeDocumentInParentFolder } from './knowledgebaseDocumentApiBridge';
import { normalizeDriveNodePage } from './knowledgeDriveSdkResponse';
import { normalizeSdkWorkListPage } from './sdkWorkListPage';

export interface CloudDriveBrowserItem {
  id: string;
  name: string;
  type: 'folder' | 'file';
  size?: string;
  updatedAt: string | null;
  mimeType?: string | null;
  driveSpaceId?: string | null;
  driveNodeId?: string | null;
  driveStorageProviderId?: string | null;
  driveBucket?: string | null;
  driveObjectKey?: string | null;
  documentId?: string | null;
}

export interface CloudDriveImportResultItem {
  title: string;
  type: string;
  documentId?: string;
  content?: string;
}

export interface CloudDriveImportFailure {
  title: string;
  message: string;
}

export interface CloudDriveBrowserItemsPage {
  items: CloudDriveBrowserItem[];
  nextCursor: string | null;
  hasMore: boolean;
}

function requireSdkClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function spaceIdFromKbId(kbId: string): string {
  return parseKnowledgeSpaceId(kbId);
}

function formatBytes(bytes: string | number | null | undefined): string | undefined {
  const value = typeof bytes === 'number' ? bytes : Number(bytes);
  if (!Number.isFinite(value) || value <= 0) {
    return undefined;
  }
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

function requireDriveClient() {
  return requireDriveApiClient();
}

async function resolveDriveSpaceId(kbId: string): Promise<string> {
  const client = requireSdkClient();
  const space = await client.knowledge.spaces.retrieve(spaceIdFromKbId(kbId));
  const driveSpaceId = space.driveSpaceId?.trim();
  if (!driveSpaceId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DRIVE_SPACE_MISSING);
  }
  return driveSpaceId;
}

function mapDriveNode(node: DriveNode): CloudDriveBrowserItem {
  const isFolder = node.nodeType === 'folder';
  const contentLength = node.contentLength ? Number(node.contentLength) : undefined;
  return {
    id: node.id,
    name: node.nodeName,
    type: isFolder ? 'folder' : 'file',
    size: formatBytes(Number.isFinite(contentLength) ? contentLength : undefined),
    updatedAt: null,
    mimeType: node.contentType ?? null,
    driveSpaceId: node.spaceId,
    driveNodeId: node.id,
    driveStorageProviderId: null,
    driveBucket: null,
    driveObjectKey: null,
    documentId: null};
}

const DRIVE_COLLECTION_PAGE_SIZE = 100;

async function listDriveCollectionPage(
  kbId: string,
  cursor: string | null | undefined,
  listPage: (
    driveSpaceId: string,
    cursor?: string,
  ) => Promise<unknown>,
): Promise<CloudDriveBrowserItemsPage> {
  const driveSpaceId = await resolveDriveSpaceId(kbId);
  const page = normalizeDriveNodePage(await listPage(driveSpaceId, cursor ?? undefined));
  return {
    items: page.items.map(mapDriveNode),
    nextCursor: page.nextCursor,
    hasMore: page.hasMore};
}

function mapBrowserNode(node: KnowledgeBrowserNode): CloudDriveBrowserItem {
  const isFolder = node.nodeType === 'folder' || node.nodeType === 'virtual_folder';
  return {
    id: node.id,
    name: node.name,
    type: isFolder ? 'folder' : 'file',
    size: formatBytes(node.sizeBytes ?? undefined),
    updatedAt: node.updatedAt,
    mimeType: node.mimeType,
    driveSpaceId: node.driveSpaceId,
    driveNodeId: node.driveNodeId,
    driveStorageProviderId: node.driveStorageProviderId,
    driveBucket: node.driveBucket,
    driveObjectKey: node.driveObjectKey,
    documentId: node.documentId ? String(node.documentId) : null};
}

function mapMimeToLegacyType(name: string, mimeType?: string | null): string {
  const lowerName = name.toLowerCase();
  const mime = mimeType ?? '';
  if (mime.startsWith('image/')) {
    return 'image';
  }
  if (mime.startsWith('video/')) {
    return 'video';
  }
  if (mime.startsWith('audio/')) {
    return 'audio';
  }
  if (mime.includes('pdf') || lowerName.endsWith('.pdf')) {
    return 'pdf';
  }
  if (lowerName.endsWith('.md') || lowerName.endsWith('.markdown') || mime.includes('markdown')) {
    return 'markdown';
  }
  if (mime.includes('html') || lowerName.endsWith('.html') || lowerName.endsWith('.htm')) {
    return 'richtext';
  }
  if (/\.(ts|tsx|js|jsx|html|htm|css|json|xml|py|java|go|rs)$/i.test(lowerName)) {
    return 'code';
  }
  return 'file';
}

export async function listCloudDriveBrowserItemsPage(
  spaceId: string,
  parentId?: string | null,
  cursor?: string | null,
): Promise<CloudDriveBrowserItemsPage> {
  const client = requireSdkClient();
  const numericSpaceId = spaceIdFromKbId(spaceId);
  const page = normalizeSdkWorkListPage<KnowledgeBrowserNode>(
    await client.knowledge.spaces.browser.list(numericSpaceId, {
      view: 'files',
      parentId: parentId ?? null,
      cursor,
      pageSize: 100}),
  );
  return {
    items: page.items.map(mapBrowserNode),
    nextCursor: page.nextCursor,
    hasMore: page.hasMore};
}

export async function listStarredCloudDriveItemsPage(
  kbId: string,
  cursor?: string | null,
): Promise<CloudDriveBrowserItemsPage> {
  const drive = requireDriveClient();
  return listDriveCollectionPage(kbId, cursor, (driveSpaceId, pageCursor) =>
    drive.drive.favorites.list({
      spaceId: driveSpaceId,
      pageSize: String(DRIVE_COLLECTION_PAGE_SIZE),
      cursor: pageCursor}));
}

export async function listRecentCloudDriveItemsPage(
  kbId: string,
  cursor?: string | null,
): Promise<CloudDriveBrowserItemsPage> {
  const drive = requireDriveClient();
  return listDriveCollectionPage(kbId, cursor, (driveSpaceId, pageCursor) =>
    drive.drive.recent.list({
      spaceId: driveSpaceId,
      pageSize: String(DRIVE_COLLECTION_PAGE_SIZE),
      cursor: pageCursor}));
}

export async function listSharedCloudDriveItemsPage(
  kbId: string,
  cursor?: string | null,
): Promise<CloudDriveBrowserItemsPage> {
  const drive = requireDriveClient();
  return listDriveCollectionPage(kbId, cursor, (driveSpaceId, pageCursor) =>
    drive.drive.sharedWithMe.list({
      spaceId: driveSpaceId,
      pageSize: String(DRIVE_COLLECTION_PAGE_SIZE),
      cursor: pageCursor}));
}

function buildIdempotencyKey(spaceId: string, item: CloudDriveBrowserItem): string {
  const nodeId = item.driveNodeId ?? item.id;
  return `pc-drive-import-${spaceId}-${nodeId}`.slice(0, 128);
}

async function importDriveFile(
  numericSpaceId: string,
  item: CloudDriveBrowserItem,
): Promise<CloudDriveImportResultItem> {
  const client = requireSdkClient();
  const driveNodeId = item.driveNodeId ?? item.id;
  if (isBlank(driveNodeId)) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DRIVE_NODE_ID_MISSING);
  }
  const driveSpaceId = item.driveSpaceId?.trim();
  if (!driveSpaceId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DRIVE_SPACE_MISSING);
  }

  const result = await client.knowledge.driveImports.create({
    spaceId: numericSpaceId,
    title: item.name,
    idempotencyKey: buildIdempotencyKey(numericSpaceId, item),
    driveSpaceId,
    driveNodeId,
    language: null});

  return {
    title: result.document.title,
    type: mapMimeToLegacyType(item.name, item.mimeType),
    documentId: result.document.id,
    content: `# ${result.document.title}\n\nImported from enterprise drive.`};
}

export async function importCloudDriveItems(
  spaceId: string,
  items: CloudDriveBrowserItem[],
  targetParentFolderId?: string | null,
): Promise<CloudDriveImportResultItem[]> {
  const numericSpaceId = requireRegisteredSpaceId(spaceId);
  const imported: CloudDriveImportResultItem[] = [];
  const failures: CloudDriveImportFailure[] = [];

  for (const item of items) {
    if (item.type === 'folder') {
      continue;
    }

    try {
      const result = await importDriveFile(numericSpaceId, item);
      if (targetParentFolderId?.trim() && result.documentId) {
        await placeDocumentInParentFolder(
          String(result.documentId),
          spaceId,
          targetParentFolderId,
        );
      }
      imported.push(result);
    } catch (error) {
      failures.push({
        title: item.name,
        message: error instanceof Error ? error.message : String(error)});
    }
  }

  if (imported.length === 0 && failures.length > 0) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED, { cause: failures });
  }

  if (failures.length > 0) {
    console.warn('[KnowledgeDriveImportService] partial import failures', failures);
  }

  if (imported.length > 0) {
    invalidateKnowledgeBrowserNodeCacheForKbIds(spaceId);
  }

  return imported;
}
