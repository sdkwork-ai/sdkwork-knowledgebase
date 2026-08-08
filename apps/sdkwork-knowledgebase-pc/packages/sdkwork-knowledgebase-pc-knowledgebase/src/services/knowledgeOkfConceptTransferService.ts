import type { KnowledgeBrowserNode } from 'sdkwork-knowledgebase-pc-core';
import {
  KnowledgebaseErrorCodes,
  parseKnowledgeSpaceId,
  requireKnowledgebaseAppSdkHttpClient,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import type { DocumentMeta } from './document';
import {
  listKnowledgeBrowserNodesPage,
  invalidateKnowledgeBrowserNodeCacheForSpaceIds} from './knowledgeBrowserListService';

const OKF_BUNDLE_BROWSER_VIEW = 'okf_bundle' as const;

function requireSdkClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function spaceIdFromKbId(kbId: string): string {
  return parseKnowledgeSpaceId(kbId);
}

function buildCopiedConceptId(conceptId: string): string {
  const suffix = `-copy-${Date.now().toString(36)}`;
  const maxLength = 240;
  if (conceptId.length + suffix.length <= maxLength) {
    return `${conceptId}${suffix}`;
  }
  return `${conceptId.slice(0, maxLength - suffix.length)}${suffix}`;
}

function patchOkfMarkdownTitle(markdown: string, title: string): string {
  if (!markdown.startsWith('---')) {
    return `# ${title}\n\n${markdown}`;
  }

  const closingIndex = markdown.indexOf('\n---', 3);
  if (closingIndex < 0) {
    return markdown;
  }

  let frontmatter = markdown.slice(4, closingIndex);
  if (/^title:\s/m.test(frontmatter)) {
    frontmatter = frontmatter.replace(/^title:\s.*$/m, `title: ${title}`);
  } else {
    frontmatter = `title: ${title}\n${frontmatter}`;
  }

  return `---\n${frontmatter}---${markdown.slice(closingIndex + 4)}`;
}

function normalizeBrowserPath(path: string): string {
  return path.replace(/^\/+/, '');
}

function okfConceptLogicalPath(conceptId: string): string {
  return `okf/${conceptId}.md`;
}

function matchOkfConceptNode(
  node: KnowledgeBrowserNode,
  expectedLogicalPath: string,
): node is KnowledgeBrowserNode & { conceptId: string } {
  if (node.nodeType !== 'okf_concept' || !node.conceptId) {
    return false;
  }
  const nodePath = normalizeBrowserPath(node.path);
  return nodePath === expectedLogicalPath || nodePath.endsWith(`/${expectedLogicalPath}`);
}

function isFolderNode(node: KnowledgeBrowserNode): boolean {
  return node.nodeType === 'folder' || node.nodeType === 'virtual_folder';
}

async function findOkfConceptRowIdInPaginatedBrowser(
  spaceId: string,
  conceptIdString: string,
  logicalPath?: string,
): Promise<string | null> {
  const expectedLogicalPath = normalizeBrowserPath(
    logicalPath ?? okfConceptLogicalPath(conceptIdString),
  );
  const folderQueue: Array<string | null> = [null];
  const visitedParents = new Set<string>();

  while (folderQueue.length > 0) {
    const parentId = folderQueue.shift()!;
    const parentKey = parentId ?? '__root__';
    if (visitedParents.has(parentKey)) {
      continue;
    }
    visitedParents.add(parentKey);

    let cursor: string | null = null;
    do {
      const page = await listKnowledgeBrowserNodesPage(spaceId, parentId, {
        cursor,
        view: OKF_BUNDLE_BROWSER_VIEW,
      });
      for (const node of page.items) {
        if (matchOkfConceptNode(node, expectedLogicalPath)) {
          return node.conceptId;
        }
        if (isFolderNode(node)) {
          folderQueue.push(node.id);
        }
      }
      cursor = page.hasMore ? page.nextCursor : null;
    } while (cursor);
  }

  return null;
}

async function findOkfConceptRowIdViaRootBrowserPages(
  spaceId: string,
  conceptIdString: string,
): Promise<string | null> {
  const client = requireSdkClient();
  let cursor: string | null = null;

  do {
    const page = await listKnowledgeBrowserNodesPage(spaceId, null, {
      cursor,
      view: OKF_BUNDLE_BROWSER_VIEW,
    });
    for (const node of page.items) {
      if (node.nodeType !== 'okf_concept' || !node.conceptId) {
        continue;
      }
      const summary = await client.knowledge.okf.concepts.retrieve(String(node.conceptId));
      if (summary.conceptId === conceptIdString) {
        return node.conceptId;
      }
    }
    cursor = page.hasMore ? page.nextCursor : null;
  } while (cursor);

  return null;
}

async function resolveOkfConceptRowIdByConceptId(
  spaceId: string,
  conceptIdString: string,
  logicalPath?: string,
): Promise<string> {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    if (attempt > 0) {
      await new Promise((resolve) => setTimeout(resolve, 250));
      invalidateKnowledgeBrowserNodeCacheForSpaceIds(spaceId);
    }

    const matched = await findOkfConceptRowIdInPaginatedBrowser(
      spaceId,
      conceptIdString,
      logicalPath,
    );
    if (matched) {
      return matched;
    }
  }

  const fallback = await findOkfConceptRowIdViaRootBrowserPages(spaceId, conceptIdString);
  if (fallback) {
    return fallback;
  }

  throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_RESOLVE_FAILED, {
    cause: conceptIdString,
  });
}

export async function copyOkfConcept(
  sourceSpaceId: string,
  sourceConceptRowId: string,
  targetKbId: string,
  readMarkdown: (spaceId: string, conceptRowId: string) => Promise<string>,
  options?: { titleSuffix?: string },
): Promise<DocumentMeta> {
  const client = requireSdkClient();
  const targetSpaceId = spaceIdFromKbId(targetKbId);
  const sourceConcept = await client.knowledge.okf.concepts.retrieve(String(sourceConceptRowId));
  const titleSuffix = options?.titleSuffix ?? ' (Copy)';
  const targetTitle = `${sourceConcept.title}${titleSuffix}`;
  const targetConceptId = buildCopiedConceptId(sourceConcept.conceptId);
  const markdown = patchOkfMarkdownTitle(
    await readMarkdown(sourceSpaceId, sourceConceptRowId),
    targetTitle,
  );

  const upserted = await client.knowledge.okf.concepts.update({
    spaceId: targetSpaceId,
    conceptId: targetConceptId,
    markdown,
    actor: 'pc-knowledgebase',
    publish: true,
  });

  const targetConceptRowId = await resolveOkfConceptRowIdByConceptId(
    targetSpaceId,
    targetConceptId,
    upserted.logicalPath,
  );
  invalidateKnowledgeBrowserNodeCacheForSpaceIds(sourceSpaceId, targetSpaceId);
  return {
    id: `okf:${targetKbId}:${targetConceptRowId}`,
    title: targetTitle,
    type: 'markdown',
    kbId: targetKbId,
    parentId: null,
    updatedAt: new Date().toISOString(),
    author: 'Knowledgebase',
    tags: sourceConcept.tags,
  };
}

export async function moveOkfConcept(
  sourceSpaceId: string,
  sourceConceptRowId: string,
  targetKbId: string,
  readMarkdown: (spaceId: string, conceptRowId: string) => Promise<string>,
): Promise<DocumentMeta> {
  const targetSpaceId = spaceIdFromKbId(targetKbId);
  if (sourceSpaceId === targetSpaceId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.TRANSFER_SAME_KB);
  }

  const client = requireSdkClient();
  const sourceConcept = await client.knowledge.okf.concepts.retrieve(String(sourceConceptRowId));
  const markdown = await readMarkdown(sourceSpaceId, sourceConceptRowId);

  const upserted = await client.knowledge.okf.concepts.update({
    spaceId: targetSpaceId,
    conceptId: sourceConcept.conceptId,
    markdown,
    actor: 'pc-knowledgebase',
    publish: true,
  });

  const targetConceptRowId = await resolveOkfConceptRowIdByConceptId(
    targetSpaceId,
    sourceConcept.conceptId,
    upserted.logicalPath,
  );

  try {
    await client.knowledge.okf.concepts.delete(String(sourceConceptRowId));
  } catch (deleteError) {
    try {
      await client.knowledge.okf.concepts.delete(String(targetConceptRowId));
    } catch (rollbackError) {
      console.error(
        '[KnowledgeOkfConceptTransferService] move rollback failed after source delete error.',
        { deleteError, rollbackError, targetConceptRowId },
      );
    }
    const detail = deleteError instanceof Error ? deleteError.message : String(deleteError);
    throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED, { cause: detail });
  }

  invalidateKnowledgeBrowserNodeCacheForSpaceIds(sourceSpaceId, targetSpaceId);
  return {
    id: `okf:${targetKbId}:${targetConceptRowId}`,
    title: sourceConcept.title,
    type: 'markdown',
    kbId: targetKbId,
    parentId: null,
    updatedAt: new Date().toISOString(),
    author: 'Knowledgebase',
    tags: sourceConcept.tags,
  };
}
