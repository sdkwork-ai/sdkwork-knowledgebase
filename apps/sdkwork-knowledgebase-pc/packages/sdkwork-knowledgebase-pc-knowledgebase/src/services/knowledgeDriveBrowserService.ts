import type { KnowledgeBrowserNode } from 'sdkwork-knowledgebase-pc-core';
import {
  parseKnowledgeSpaceId,
  requireDriveApiClient,
  requireDriveNodeId,
  requireDriveSpaceIdFromKbSpace,
  requireKnowledgebaseAppSdkHttpClient} from 'sdkwork-knowledgebase-pc-core';

import type { DocumentMeta } from './document';
import { resolveKnowledgeBrowserParentDriveNodeId } from './knowledgeBrowserParentResolver';
import { readDriveNode } from './knowledgeDriveSdkResponse';

function resolveDriveNodeId(node: KnowledgeBrowserNode): string | null {
  return node.driveNodeId?.trim() || node.id?.trim() || null;
}

function spaceIdFromKbId(kbId: string): string {
  return parseKnowledgeSpaceId(kbId);
}

export async function resolveKnowledgeDriveSpaceId(kbId: string): Promise<string> {
  const client = requireKnowledgebaseAppSdkHttpClient();
  const space = await client.knowledge.spaces.retrieve(spaceIdFromKbId(kbId));
  return requireDriveSpaceIdFromKbSpace(space.driveSpaceId);
}

export async function createKnowledgeDriveFolder(input: {
  kbId: string;
  nodeName: string;
  parentDriveNodeId?: string | null;
}): Promise<{ driveNodeId: string; nodeName: string }> {
  const driveSpaceId = await resolveKnowledgeDriveSpaceId(input.kbId);
  const folder = readDriveNode(
    await requireDriveApiClient().drive.nodes.folders.create({
      spaceId: driveSpaceId,
      parentNodeId: input.parentDriveNodeId?.trim() || undefined,
      nodeName: input.nodeName.trim()}),
  );
  return {
    driveNodeId: folder.id,
    nodeName: folder.nodeName?.trim() || input.nodeName.trim()};
}

export async function applyDriveBrowserNodeUpdates(
  kbId: string,
  node: KnowledgeBrowserNode,
  updates: Pick<DocumentMeta, 'title' | 'parentId' | 'isPinned'>,
): Promise<void> {
  const driveNodeId = requireDriveNodeId(resolveDriveNodeId(node));
  const drive = requireDriveApiClient();

  if (updates.title !== undefined && updates.title.trim() !== node.name) {
    await drive.drive.nodes.update(driveNodeId, {
      nodeName: updates.title.trim()});
  }

  if (updates.parentId !== undefined) {
    const targetParent = updates.parentId?.trim() || undefined;
    const currentParent = node.parentId?.trim() || undefined;
    if (targetParent !== currentParent) {
      const targetDriveParent = await resolveKnowledgeBrowserParentDriveNodeId(
        kbId,
        targetParent ?? null,
      );
      await drive.drive.nodes.move(driveNodeId, {
        targetParentNodeId: targetDriveParent});
    }
  }

  if (updates.isPinned !== undefined) {
    if (updates.isPinned) {
      await drive.drive.favorites.update(driveNodeId, {});
    } else {
      await drive.drive.favorites.delete(driveNodeId);
    }
  }
}

export async function deleteDriveBrowserNode(node: KnowledgeBrowserNode): Promise<void> {
  const driveNodeId = requireDriveNodeId(resolveDriveNodeId(node));
  await requireDriveApiClient().drive.nodes.delete(driveNodeId);
}

export async function ensureDriveFolderPath(
  driveSpaceId: string,
  rootParentNodeId: string | null | undefined,
  relativePath: string | undefined,
  folderCache: Map<string, string>,
): Promise<string | undefined> {
  if (!relativePath || !relativePath.includes('/')) {
    return rootParentNodeId?.trim() || undefined;
  }

  const parts = relativePath.split('/');
  parts.pop();
  if (parts.length === 0) {
    return rootParentNodeId?.trim() || undefined;
  }

  let currentParent = rootParentNodeId?.trim() || undefined;
  let pathAccumulator = '';

  for (const folderName of parts) {
    pathAccumulator = pathAccumulator ? `${pathAccumulator}/${folderName}` : folderName;
    const cached = folderCache.get(pathAccumulator);
    if (cached) {
      currentParent = cached;
      continue;
    }

    const folder = readDriveNode(
      await requireDriveApiClient().drive.nodes.folders.create({
        spaceId: driveSpaceId,
        parentNodeId: currentParent,
        nodeName: folderName}),
    );
    folderCache.set(pathAccumulator, folder.id);
    currentParent = folder.id;
  }

  return currentParent;
}
