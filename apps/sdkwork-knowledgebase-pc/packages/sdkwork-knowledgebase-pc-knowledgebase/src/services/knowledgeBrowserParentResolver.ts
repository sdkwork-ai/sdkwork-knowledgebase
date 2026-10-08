import type { KnowledgeBrowserNode } from 'sdkwork-knowledgebase-pc-core';
import { isBlank, trim } from '@sdkwork/utils';
import {
  KnowledgebaseErrorCodes,
  isKnowledgebaseDriveApiAvailable,
  parseKnowledgeSpaceId,
  requireDriveApiClient,
  requireKnowledgebaseAppSdkHttpClient,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import {
  findKnowledgeBrowserNodeByDocumentId,
  getLoadedKnowledgeBrowserNodes,
  listKnowledgeBrowserNodesPage,
} from './knowledgeBrowserListService';
import { resolveDriveNodeId } from './knowledgeDriveDocumentMetadataService';

function spaceIdFromKbId(kbId: string): string {
  return parseKnowledgeSpaceId(kbId);
}

function requireSdkClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function normalizeKnowledgeMode(value: unknown): string {
  return typeof value === 'string' && value.trim() ? value.trim() : 'okf_bundle';
}

function parseOkfDocumentReference(reference: string): { conceptRowId: string } | null {
  const match = /^okf:[0-9]+:([0-9]+)$/.exec(reference.trim());
  if (!match || /^0+$/.test(match[1])) {
    return null;
  }
  // Ids stay canonical decimal text: `Number` would silently round ids past
  // 2^53 even when the value is only used for validation.
  return { conceptRowId: match[1] };
}

function isNumericDocumentReference(reference: string): boolean {
  const trimmed = reference.trim();
  return /^[0-9]+$/.test(trimmed) && !/^0+$/.test(trimmed);
}

function looksLikeDriveNodeId(reference: string): boolean {
  const trimmed = reference.trim();
  if (/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(trimmed)) {
    return true;
  }
  return /^dn[_-]/i.test(trimmed) || /^node[_-]/i.test(trimmed);
}

function findBrowserNodeByParentReference(
  nodes: KnowledgeBrowserNode[],
  parentReference: string,
  kbId: string,
): KnowledgeBrowserNode | null {
  return findKnowledgeBrowserNodeByDocumentId(nodes, parentReference, kbId)
    ?? nodes.find((candidate) => candidate.id === parentReference)
    ?? nodes.find((candidate) => candidate.driveNodeId === parentReference)
    ?? null;
}

const READ_ONLY_BROWSER_PERMISSIONS: KnowledgeBrowserNode['permissions'] = {
  canRead: true,
  canUpload: false,
  canRename: false,
  canMove: false,
  canDelete: false,
  canReview: false,
  canPublish: false,
};

function buildSyntheticFolderNode(driveNodeId: string, name: string): KnowledgeBrowserNode {
  return {
    id: driveNodeId,
    nodeType: 'folder',
    name,
    path: '',
    updatedAt: new Date().toISOString(),
    driveNodeId,
    permissions: READ_ONLY_BROWSER_PERMISSIONS,
  };
}

function throwParentFolderNotFound(parentReference: string): never {
  throwKnowledgebaseError(KnowledgebaseErrorCodes.PARENT_FOLDER_NOT_FOUND, {
    cause: parentReference,
  });
}

async function resolveDriveNodeIdFromNumericDocument(documentId: string): Promise<string | null> {
  try {
    const document = await requireSdkClient().knowledge.documents.retrieve(documentId);
    return trim(document.originalFileDriveNodeId ?? '') || null;
  } catch {
    return null;
  }
}

async function tryResolveDriveNodeIdFromReference(reference: string): Promise<string | null> {
  if (!looksLikeDriveNodeId(reference)) {
    return null;
  }
  if (!isKnowledgebaseDriveApiAvailable()) {
    return reference.trim();
  }

  const drive = requireDriveApiClient();
  const retrieve = (drive.drive.nodes as { retrieve?: (nodeId: string) => Promise<{ id: string }> }).retrieve;
  if (typeof retrieve !== 'function') {
    return reference.trim();
  }

  try {
    const node = await retrieve.call(drive.drive.nodes, reference.trim());
    return trim(node.id) || reference.trim();
  } catch {
    return reference.trim();
  }
}

async function resolveBrowserParentNode(
  kbId: string,
  parentReference: string,
): Promise<KnowledgeBrowserNode> {
  const trimmed = trim(parentReference ?? '');
  const spaceId = spaceIdFromKbId(kbId);
  const loadedNodes = getLoadedKnowledgeBrowserNodes(spaceId);

  const cached = findBrowserNodeByParentReference(loadedNodes, trimmed, kbId);
  if (cached) {
    return cached;
  }

  if (isNumericDocumentReference(trimmed)) {
    const driveNodeId = await resolveDriveNodeIdFromNumericDocument(trimmed);
    if (driveNodeId) {
      const byDriveNodeId = loadedNodes.find(
        (candidate) => candidate.driveNodeId === driveNodeId || candidate.id === driveNodeId,
      );
      if (byDriveNodeId) {
        return byDriveNodeId;
      }
      return buildSyntheticFolderNode(driveNodeId, trimmed);
    }
  }

  if (looksLikeDriveNodeId(trimmed)) {
    const driveNodeId = await tryResolveDriveNodeIdFromReference(trimmed);
    if (driveNodeId) {
      const byDriveNodeId = loadedNodes.find(
        (candidate) => candidate.driveNodeId === driveNodeId || candidate.id === driveNodeId,
      );
      if (byDriveNodeId) {
        return byDriveNodeId;
      }
      return buildSyntheticFolderNode(driveNodeId, trimmed);
    }
  }

  if (parseOkfDocumentReference(trimmed)) {
    throwParentFolderNotFound(trimmed);
  }

  throwParentFolderNotFound(trimmed);
}

export async function resolveKnowledgeBrowserParentDriveNodeId(
  kbId: string,
  parentReference: string | null | undefined,
): Promise<string | undefined> {
  if (isBlank(parentReference)) {
    return resolveKnowledgeBrowserFilesRootDriveNodeId(kbId);
  }

  const trimmed = trim(parentReference ?? '');

  if (isNumericDocumentReference(trimmed)) {
    const driveNodeId = await resolveDriveNodeIdFromNumericDocument(trimmed);
    if (!driveNodeId) {
      throwParentFolderNotFound(trimmed);
    }
    return driveNodeId;
  }

  if (looksLikeDriveNodeId(trimmed)) {
    const driveNodeId = await tryResolveDriveNodeIdFromReference(trimmed);
    if (driveNodeId) {
      return driveNodeId;
    }
  }

  const node = await resolveBrowserParentNode(kbId, trimmed);
  const driveNodeId = resolveDriveNodeId(node);
  if (!driveNodeId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PARENT_DRIVE_NODE_MISSING, {
      cause: trimmed,
    });
  }

  return driveNodeId;
}

export async function resolveKnowledgeBrowserFilesRootDriveNodeId(
  kbId: string,
): Promise<string | undefined> {
  const spaceId = spaceIdFromKbId(kbId);
  const page = await listKnowledgeBrowserNodesPage(spaceId, null, {
    fresh: true,
    pageSize: 1,
  });
  if (page.parentId) {
    return page.parentId;
  }

  const space = await requireSdkClient().knowledge.spaces.retrieve(spaceId);
  if (normalizeKnowledgeMode(space.knowledgeMode) === 'okf_bundle') {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.PARENT_DRIVE_NODE_MISSING, {
      cause: 'knowledge browser files root',
    });
  }

  return undefined;
}

export async function resolveKnowledgeBrowserParentNodeId(
  kbId: string,
  parentReference: string | null | undefined,
): Promise<string | null> {
  if (isBlank(parentReference)) {
    return null;
  }

  const node = await resolveBrowserParentNode(kbId, trim(parentReference ?? ''));
  return node.id;
}