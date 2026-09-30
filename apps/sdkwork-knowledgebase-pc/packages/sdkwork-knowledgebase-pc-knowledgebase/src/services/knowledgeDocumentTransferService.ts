import type { DriveNode, KnowledgeBrowserNode } from 'sdkwork-knowledgebase-pc-core';
import {
  isKnowledgebaseDriveApiAvailable,
  KnowledgebaseErrorCodes,
  parseKnowledgeSpaceId,
  requireDriveApiClient,
  requireDriveNodeId,
  requireDriveSpaceIdFromKbSpace,
  requireKnowledgebaseAppSdkHttpClient,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import type { DocumentMeta } from './document';
import {
  invalidateKnowledgeBrowserNodeCacheForKbIds,
  MAX_AGGREGATION_SCAN_PAGES,
} from './knowledgeBrowserListService';
import { resolveKnowledgeBrowserParentDriveNodeId } from './knowledgeBrowserParentResolver';
import { resolveDriveNodeId } from './knowledgeDriveDocumentMetadataService';
import { normalizeDriveNodePage, readDriveNode } from './knowledgeDriveSdkResponse';

type TransferMode = 'move' | 'copy';

function requireKnowledgeClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function requireDriveClient() {
  return requireDriveApiClient();
}

function spaceIdFromKbId(kbId: string): string {
  return parseKnowledgeSpaceId(kbId);
}

async function resolveDriveSpaceId(kbId: string): Promise<string> {
  const space = await requireKnowledgeClient().knowledge.spaces.retrieve(spaceIdFromKbId(kbId));
  return requireDriveSpaceIdFromKbSpace(space.driveSpaceId);
}

function isFolderNode(node: KnowledgeBrowserNode): boolean {
  return node.nodeType === 'folder' || node.nodeType === 'virtual_folder';
}

function buildTransferIdempotencyKey(
  targetSpaceId: string,
  sourceId: string,
  driveNodeId: string,
  mode: TransferMode,
): string {
  return `pc-${mode}-${targetSpaceId}-${sourceId}-${driveNodeId}`.slice(0, 128);
}

async function importTransferredDriveNode(
  targetKbId: string,
  driveNode: Pick<DriveNode, 'id' | 'nodeName' | 'spaceId'>,
  sourceId: string,
  mode: TransferMode,
): Promise<DocumentMeta> {
  const targetSpaceId = spaceIdFromKbId(targetKbId);
  const result = await requireKnowledgeClient().knowledge.driveImports.create({
    spaceId: targetSpaceId,
    title: driveNode.nodeName,
    idempotencyKey: buildTransferIdempotencyKey(targetSpaceId, sourceId, driveNode.id, mode),
    driveSpaceId: driveNode.spaceId,
    driveNodeId: driveNode.id,
    language: null,
  });

  return {
    id: String(result.document.id),
    title: result.document.title,
    type: 'file',
    kbId: targetKbId,
    parentId: null,
    updatedAt: new Date().toISOString(),
    author: 'Knowledgebase',
  };
}

async function transferDriveBackedDocument(
  sourceId: string,
  sourceNode: KnowledgeBrowserNode,
  targetKbId: string,
  targetParentId: string | null,
  mode: TransferMode,
  titleSuffix?: string,
): Promise<DocumentMeta> {
  const driveNodeId = requireDriveNodeId(resolveDriveNodeId(sourceNode));
  const targetDriveSpaceId = await resolveDriveSpaceId(targetKbId);
  const drive = requireDriveClient();
  const targetDriveParentId = await resolveKnowledgeBrowserParentDriveNodeId(
    targetKbId,
    targetParentId,
  );
  const copiedNode = readDriveNode(
    await drive.drive.nodes.copy(driveNodeId, {
      id: driveNodeId,
      targetSpaceId: targetDriveSpaceId,
      targetParentNodeId: targetDriveParentId,
      nodeName: titleSuffix ? `${sourceNode.name}${titleSuffix}` : undefined,
    }),
  );

  return importTransferredDriveNode(targetKbId, copiedNode, sourceId, mode);
}

async function collectDriveFileNodesForFolderTransfer(
  driveSpaceId: string,
  rootNodeId: string,
): Promise<DriveNode[]> {
  const drive = requireDriveClient();
  const files: DriveNode[] = [];
  const queue: string[] = [rootNodeId];
  const visited = new Set<string>();
  let scannedPages = 0;

  while (queue.length > 0) {
    const parentId = queue.shift()!;
    if (visited.has(parentId)) {
      continue;
    }
    visited.add(parentId);

    let cursor: string | null = null;
    do {
      scannedPages += 1;
      if (scannedPages > MAX_AGGREGATION_SCAN_PAGES) {
        // Failing closed instead of truncating: a partial file list would
        // silently drop documents from the transfer.
        throwKnowledgebaseError(
          KnowledgebaseErrorCodes.TRANSFER_SCAN_LIMIT_EXCEEDED,
        );
      }
      const page = normalizeDriveNodePage(await drive.drive.nodes.list(driveSpaceId, {
        parentNodeId: parentId,
        pageSize: '100',
        cursor: cursor ?? undefined,
      }));
      for (const node of page.items) {
        if (node.nodeType === 'folder') {
          queue.push(node.id);
        } else if (node.nodeType === 'file') {
          files.push(node);
        }
      }
      cursor = page.hasMore ? page.nextCursor : null;
    } while (cursor);
  }

  return files;
}

function buildFolderDocumentMeta(
  kbId: string,
  driveNode: Pick<DriveNode, 'id' | 'nodeName'>,
  parentId: string | null,
): DocumentMeta {
  return {
    id: driveNode.id,
    title: driveNode.nodeName,
    type: 'folder',
    kbId,
    parentId,
    updatedAt: new Date().toISOString(),
    author: 'Knowledgebase',
  };
}

async function transferDriveFolder(
  sourceId: string,
  sourceNode: KnowledgeBrowserNode,
  sourceKbId: string,
  targetKbId: string,
  targetParentId: string | null,
  mode: TransferMode,
  titleSuffix?: string,
  deleteSourceDocument?: (sourceId: string) => Promise<boolean>,
): Promise<DocumentMeta> {
  const driveNodeId = requireDriveNodeId(resolveDriveNodeId(sourceNode));
  const targetDriveSpaceId = await resolveDriveSpaceId(targetKbId);
  const drive = requireDriveClient();
  const targetDriveParentId = await resolveKnowledgeBrowserParentDriveNodeId(
    targetKbId,
    targetParentId,
  );
  const copiedRoot = readDriveNode(
    await drive.drive.nodes.copy(driveNodeId, {
      id: driveNodeId,
      targetSpaceId: sourceKbId === targetKbId ? undefined : targetDriveSpaceId,
      targetParentNodeId: targetDriveParentId,
      nodeName: titleSuffix ? `${sourceNode.name}${titleSuffix}` : undefined,
    }),
  );

  const files = await collectDriveFileNodesForFolderTransfer(copiedRoot.spaceId, copiedRoot.id);
  for (const file of files) {
    await importTransferredDriveNode(
      targetKbId,
      file,
      `${sourceId}-folder-file-${file.id}`,
      mode,
    );
  }

  if (mode === 'move' && deleteSourceDocument) {
    await deleteSourceDocument(sourceId);
  }

  invalidateKnowledgeBrowserNodeCacheForKbIds(sourceKbId, targetKbId);
  return buildFolderDocumentMeta(targetKbId, copiedRoot, targetParentId);
}

export async function transferDocumentAcrossKnowledgeBases(
  sourceId: string,
  sourceKbId: string,
  targetKbId: string,
  targetParentId: string | null,
  mode: TransferMode,
  options?: {
    titleSuffix?: string;
    sourceNode?: KnowledgeBrowserNode | null;
    ingestTextDocument?: (
      sourceId: string,
      targetKbId: string,
      targetParentId: string | null,
      titleSuffix?: string,
    ) => Promise<DocumentMeta>;
    deleteSourceDocument?: (sourceId: string) => Promise<boolean>;
  },
): Promise<DocumentMeta> {
  if (sourceKbId === targetKbId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.TRANSFER_SAME_KB);
  }

  const sourceNode = options?.sourceNode ?? null;
  if (sourceNode && isFolderNode(sourceNode)) {
    if (!isKnowledgebaseDriveApiAvailable()) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
    }
    return transferDriveFolder(
      sourceId,
      sourceNode,
      sourceKbId,
      targetKbId,
      targetParentId,
      mode,
      options?.titleSuffix,
      options?.deleteSourceDocument,
    );
  }

  let created: DocumentMeta;
  const driveNodeId = sourceNode ? resolveDriveNodeId(sourceNode) : null;
  if (driveNodeId && isKnowledgebaseDriveApiAvailable()) {
    created = await transferDriveBackedDocument(
      sourceId,
      sourceNode!,
      targetKbId,
      targetParentId,
      mode,
      options?.titleSuffix,
    );
  } else if (options?.ingestTextDocument) {
    created = await options.ingestTextDocument(
      sourceId,
      targetKbId,
      targetParentId,
      options.titleSuffix,
    );
  } else {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
  }

  if (mode === 'move' && options?.deleteSourceDocument) {
    await options.deleteSourceDocument(sourceId);
  }

  invalidateKnowledgeBrowserNodeCacheForKbIds(sourceKbId, targetKbId);
  return created;
}

export async function copyDriveFolderWithinKnowledgeBase(
  sourceId: string,
  sourceNode: KnowledgeBrowserNode,
  targetKbId: string,
  targetParentId: string | null,
  titleSuffix?: string,
): Promise<DocumentMeta> {
  if (!isKnowledgebaseDriveApiAvailable()) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
  }
  return transferDriveFolder(
    sourceId,
    sourceNode,
    targetKbId,
    targetKbId,
    targetParentId,
    'copy',
    titleSuffix,
  );
}

export async function copyDriveFileWithinKnowledgeBase(
  sourceId: string,
  sourceNode: KnowledgeBrowserNode,
  targetKbId: string,
  targetParentId: string | null,
  titleSuffix?: string,
): Promise<DocumentMeta> {
  if (!isKnowledgebaseDriveApiAvailable()) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
  }
  const created = await transferDriveBackedDocument(
    sourceId,
    sourceNode,
    targetKbId,
    targetParentId,
    'copy',
    titleSuffix,
  );
  invalidateKnowledgeBrowserNodeCacheForKbIds(targetKbId);
  return created;
}
