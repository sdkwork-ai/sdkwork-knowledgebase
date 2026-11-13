import type { KnowledgeBrowserNode } from 'sdkwork-knowledgebase-pc-core';
import { isBlank } from '@sdkwork/utils';
import * as KnowledgeSpaceSettingsService from './knowledgeSpaceSettingsService';
import {
  applyDriveBrowserNodeUpdates,
  createKnowledgeDriveFolder,
  deleteDriveBrowserNode,
} from './knowledgeDriveBrowserService';
import {
  enrichDocumentTreeMetadata,
  listDriveFavoriteNodeIds,
  resolveDriveNodeId,
  writeDriveDocumentOrder,
  writeDriveDocumentTags,
} from './knowledgeDriveDocumentMetadataService';
import {
  readOkfConceptTags,
  updateOkfConceptTags,
} from './knowledgeOkfDocumentMetadataService';
import { copyOkfConcept, moveOkfConcept } from './knowledgeOkfConceptTransferService';
import { resolveIngestedDocument, waitForIngestJob } from './knowledgeIngestService';
import {
  transferDocumentAcrossKnowledgeBases,
  copyDriveFolderWithinKnowledgeBase,
  copyDriveFileWithinKnowledgeBase,
} from './knowledgeDocumentTransferService';
import {
  ensureKnowledgeBrowserFolderLoaded,
  findKnowledgeBrowserNodeByDocumentId,
  getLoadedKnowledgeBrowserNodes,
  hasMoreKnowledgeBrowserNodes,
  listKnowledgeBrowserNodesPage,
  listLoadedKnowledgeBrowserNodes,
  loadMoreKnowledgeBrowserNodes,
  invalidateKnowledgeBrowserNodeCacheForKbIds,
  invalidateKnowledgeBrowserNodeCacheForSpaceIds,
  resolveBrowserDocumentId,
  scanKnowledgeBrowserNodes,
} from './knowledgeBrowserListService';
import { resolveKnowledgeBrowserParentDriveNodeId } from './knowledgeBrowserParentResolver';
import { readDriveNodeTextContent } from './knowledgeDriveContentService';
import { hydrateDocumentMediaUrl } from './knowledgeDriveMediaService';
import {
  partitionKnowledgeSpaceIdsByMode,
  resolveKnowledgeSpaceKnowledgeMode,
} from './knowledgeSpaceKnowledgeMode';
import {
  fetchKnowledgeDocumentContent,
  getKnowledgebaseActorId,
  getKnowledgebaseTenantId,
  isKnowledgebaseAppError,
  isKnowledgebaseDriveApiAvailable,
  KnowledgebaseErrorCodes,
  parseKnowledgeSpaceId,
  readLocalDocumentContent,
  readRecentDocuments,
  readRegisteredSpaces,
  removeLocalDocumentContent,
  removeRecentDocument,
  removeRegisteredSpace,
  requireKnowledgebaseAppSdkHttpClient,
  requireKnowledgebaseTenantId,
  requireRegisteredSpaceId,
  parseSdkProblemDetails,
  throwKnowledgebaseError,
  touchRecentDocument,
  type KnowledgebaseSpaceKbType,
  type RegisteredKnowledgebaseSpace,
  updateRegisteredSpace,
  upsertRegisteredSpace,
  writeLocalDocumentContent,
} from 'sdkwork-knowledgebase-pc-core';

import type { DocumentMeta, FolderNode, KnowledgeBase } from './document';
import { listSpaceNotes } from './knowledgeNotesListService';
import { normalizeSdkWorkListPage } from './sdkWorkListPage';
import { getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId } from '../workspaceMode';

function requireTenantId(): string {
  return requireKnowledgebaseTenantId();
}

function requireSdkClient() {
  return requireKnowledgebaseAppSdkHttpClient();
}

function spaceIdFromKbId(kbId: string): string {
  return parseKnowledgeSpaceId(kbId);
}

function getEphemeralWorkspaceSpaceId(): string | null {
  return getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId();
}

function shouldPersistBrowserWorkspaceState(): boolean {
  return getEphemeralWorkspaceSpaceId() === null;
}

function requireWorkspaceSpaceId(kbId: string): string {
  const spaceId = spaceIdFromKbId(kbId);
  const ephemeralSpaceId = getEphemeralWorkspaceSpaceId();
  if (ephemeralSpaceId !== null && ephemeralSpaceId !== spaceId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.SPACE_REQUIRED);
  }
  return spaceId;
}

function requireRegisteredOrEphemeralWorkspaceSpace(kbId: string): string {
  const spaceId = requireWorkspaceSpaceId(kbId);
  if (getEphemeralWorkspaceSpaceId() === null) {
    requireRegisteredSpaceId(kbId);
  }
  return spaceId;
}

function assertEphemeralWorkspaceSpaceId(spaceId: string): void {
  const ephemeralSpaceId = getEphemeralWorkspaceSpaceId();
  if (ephemeralSpaceId !== null && ephemeralSpaceId !== spaceId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.SPACE_REQUIRED);
  }
}

function formatBytes(bytes: string | number | null | undefined): string | undefined {
  const value = toSafeNumber(bytes);
  if (!value || value <= 0) {
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

function toSafeNumber(value: string | number | null | undefined): number {
  const parsed = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function parseOkfDocumentId(id: string): { spaceId: string; conceptRowId: string } | null {
  const match = /^okf:(\d+):(\d+)$/.exec(id);
  if (!match) {
    return null;
  }
  const spaceId = match[1];
  const conceptRowId = match[2];
  if (!spaceId || /^0+$/u.test(spaceId) || !conceptRowId || /^0+$/u.test(conceptRowId)) {
    return null;
  }
  return { spaceId, conceptRowId };
}

/**
 * Resolve the Drive node that holds one OKF concept's Markdown body.
 *
 * Concept files live under the space's `okf/` bundle root, so they are only
 * listed by the OKF browser view; the default `files` view starts at
 * `sources/raw`. A loaded tree answers without a request, and a cold tree costs
 * one bounded page scan.
 *
 * @param spaceId - owning knowledge space.
 * @param conceptRowId - concept row id (`KnowledgeBrowserNode.conceptId`).
 * @returns the concept file's Drive node id, or null when it is not listed.
 */
async function resolveOkfConceptDriveNodeId(
  spaceId: string,
  conceptRowId: string,
): Promise<string | null> {
  const loaded = getLoadedKnowledgeBrowserNodes(spaceId, { view: 'okf_bundle' });
  const cached = loaded.find((node) => node.conceptId === conceptRowId);
  if (cached) {
    return resolveDriveNodeId(cached);
  }
  const scanned = await scanKnowledgeBrowserNodes(
    spaceId,
    (node) => node.conceptId === conceptRowId,
    { view: 'okf_bundle' },
  );
  return scanned ? resolveDriveNodeId(scanned) : null;
}

/**
 * Read one OKF concept's authoritative Markdown body.
 *
 * The body is the concept file in the space's OKF bundle, read through the
 * Drive object API. The knowledge retrieval API is RAG-only and the gateway
 * refuses it for OKF-bundle spaces (`rag_retrieval_mode_required`, 40003), so it
 * cannot serve a concept body. The concept's own title and description remain
 * the safe fallback when the file cannot be read.
 *
 * @param spaceId - owning knowledge space.
 * @param conceptRowId - concept row id.
 * @param driveNodeId - already-resolved concept file node, when the caller has
 *   the browser node at hand.
 * @returns the Markdown body, or the title/description fallback.
 */
async function readOkfConceptMarkdown(
  spaceId: string,
  conceptRowId: string,
  driveNodeId?: string | null,
): Promise<string> {
  const concept = await requireSdkClient().knowledge.okf.concepts.retrieve(conceptRowId);
  const resolvedDriveNodeId = driveNodeId ?? await resolveOkfConceptDriveNodeId(spaceId, conceptRowId);
  if (resolvedDriveNodeId) {
    const markdown = await readDriveNodeTextContent(resolvedDriveNodeId);
    if (markdown !== null) {
      return markdown;
    }
  }
  return `# ${concept.title}\n\n${concept.description}`;
}

function mapNodeType(node: KnowledgeBrowserNode): DocumentMeta['type'] {
  if (node.nodeType === 'folder' || node.nodeType === 'virtual_folder') {
    return 'folder';
  }
  if (node.nodeType === 'okf_concept') {
    return 'markdown';
  }

  const mime = node.mimeType ?? '';
  if (mime.startsWith('image/')) {
    return 'image';
  }
  if (mime.startsWith('video/')) {
    return 'video';
  }
  if (mime.startsWith('audio/')) {
    return 'audio';
  }
  if (mime.includes('pdf') || node.name.toLowerCase().endsWith('.pdf')) {
    return 'pdf';
  }
  if (mime.includes('markdown') || node.name.toLowerCase().endsWith('.md')) {
    return 'markdown';
  }
  if (/\.(ts|tsx|js|jsx|html|htm|css|json|xml|py|java|go|rs)$/i.test(node.name)) {
    return 'code';
  }
  return 'file';
}

function mapBrowserNodeToDocument(node: KnowledgeBrowserNode, kbId: string): DocumentMeta | FolderNode {
  const base: DocumentMeta = {
    id: resolveBrowserDocumentId(node, kbId),
    title: node.name,
    type: mapNodeType(node),
    kbId,
    parentId: node.parentId ?? null,
    updatedAt: node.updatedAt,
    author: 'Knowledgebase',
    size: formatBytes(node.sizeBytes ?? undefined),
  };

  if (base.type === 'folder') {
    return {
      ...base,
      type: 'folder',
      children: [],
    };
  }

  return base;
}

function buildKnowledgeBase(
  space: RegisteredKnowledgebaseSpace,
  name: string,
): KnowledgeBase {
  return {
    id: String(space.spaceId),
    title: name,
    icon: space.icon ?? '📘',
    type: space.kbType,
    avatar: space.avatar,
  };
}

function groupDocumentsByParent(
  nodes: KnowledgeBrowserNode[],
  kbId: string,
): (FolderNode | DocumentMeta)[] {
  const map = new Map<string, FolderNode | DocumentMeta>();
  const roots: (FolderNode | DocumentMeta)[] = [];

  for (const node of nodes) {
    map.set(node.id, mapBrowserNodeToDocument(node, kbId));
  }

  for (const node of nodes) {
    const current = map.get(node.id);
    if (!current) {
      continue;
    }

    if (node.parentId && map.has(node.parentId)) {
      const parent = map.get(node.parentId);
      if (parent && parent.type === 'folder') {
        const folderParent = parent as FolderNode;
        folderParent.children = folderParent.children ?? [];
        folderParent.children.push(current);
      }
    } else {
      roots.push(current);
    }
  }

  return roots;
}

/**
 * Default-space creations in flight, keyed by `<tenantId>:<userId>`.
 *
 * Several surfaces list knowledge bases during first paint (the sidebar, the
 * notes workspace, the composer catalog) and an account switch empties the
 * registry, so concurrent callers used to create one "我的知识库" each. The entry
 * is dropped as soon as the creation settles: it is a request de-duplicator, not
 * a cache of the created space.
 */
const defaultSpaceInFlight = new Map<string, Promise<{ entry: RegisteredKnowledgebaseSpace; name: string }>>();

function defaultSpaceInFlightKey(tenantId: string): string {
  return `${tenantId}:${getKnowledgebaseActorId() ?? ''}`;
}

async function ensureDefaultSpace(tenantId: string): Promise<{ entry: RegisteredKnowledgebaseSpace; name: string }> {
  const inFlightKey = defaultSpaceInFlightKey(tenantId);
  const existing = defaultSpaceInFlight.get(inFlightKey);
  if (existing) {
    return existing;
  }
  const promise = createDefaultSpace(tenantId).finally(() => {
    defaultSpaceInFlight.delete(inFlightKey);
  });
  defaultSpaceInFlight.set(inFlightKey, promise);
  return promise;
}

async function createDefaultSpace(tenantId: string): Promise<{ entry: RegisteredKnowledgebaseSpace; name: string }> {
  const client = requireSdkClient();
  const created = await client.knowledge.spaces.create({
    name: '我的知识库',
    description: 'Default knowledge space',
  });
  const entry: RegisteredKnowledgebaseSpace = {
    spaceId: created.id,
    kbType: 'personal',
    icon: '📓',
    createdAt: new Date().toISOString(),
  };
  upsertRegisteredSpace(tenantId, entry);
  return { entry, name: created.name || '我的知识库' };
}

export interface KnowledgeBaseGroups {
  team: KnowledgeBase[];
  personal: KnowledgeBase[];
  public: KnowledgeBase[];
  /** Registry space ids skipped this round because their retrieve failed with a
   *  transient error. They stay registered but are missing from the groups
   *  above, so callers can surface the degradation. */
  unavailableSpaceIds: string[];
}

/**
 * Reports whether a failed space read is the gateway refusing this actor, rather
 * than a transient read or a deployment scope mismatch.
 *
 * The shared browser profile can still carry ids the signed-in account cannot
 * open (adopted legacy registry, an access that was revoked). Those answers are
 * definitive for the reader, so the reference has to leave the registry; a 5xx
 * or a tenant mismatch must keep it.
 */
function isSpaceAccessDenied(reason: unknown): boolean {
  const problem = parseSdkProblemDetails(reason);
  if (problem?.status !== 403) {
    return false;
  }
  const code = problem.code ?? '';
  return code === 'knowledge_space_access_denied'
    || code === 'group_knowledge_space_access_denied'
    || code === '40301'
    || (problem.detail ?? '').includes('does not have access to space');
}

export async function getKnowledgeBases(): Promise<KnowledgeBaseGroups> {
  const ephemeralSpaceId = getEphemeralWorkspaceSpaceId();
  if (ephemeralSpaceId !== null) {
    const space = await requireSdkClient().knowledge.spaces.retrieve(ephemeralSpaceId);
    return {
      team: [{
        id: ephemeralSpaceId,
        title: space.name,
        icon: '📘',
        type: 'team',
      }],
      personal: [],
      public: [],
      unavailableSpaceIds: [],
    };
  }

  const tenantId = requireTenantId();
  const client = requireSdkClient();
  let registry = readRegisteredSpaces(tenantId);

  if (registry.length === 0) {
    registry = [(await ensureDefaultSpace(tenantId)).entry];
  }

  const grouped: Record<KnowledgebaseSpaceKbType, KnowledgeBase[]> = {
    team: [],
    personal: [],
    public: [],
  };
  const unavailableSpaceIds: string[] = [];
  let retainedSpaceCount = 0;

  // Concurrent resolution keeps the sidebar's first paint independent of the registry
  // size; a serialized loop makes group latency grow linearly with each round trip.
  const registeredEntries = registry.filter((entry) => entry.spaceId && entry.spaceId !== '0');
  const settleResults = await Promise.allSettled(
    registeredEntries.map((entry) => client.knowledge.spaces.retrieve(entry.spaceId)),
  );
  registeredEntries.forEach((entry, index) => {
    const result = settleResults[index];
    if (!result) {
      return;
    }
    if (result.status === 'fulfilled') {
      retainedSpaceCount += 1;
      grouped[entry.kbType].push(buildKnowledgeBase(entry, result.value.name));
      return;
    }
    // A failed retrieve is transient by default (gateway restart, network
    // blip, 5xx): permanently deleting the registry reference turned every
    // backend hiccup into silent knowledge-base loss after refresh. Only a
    // definitive answer may remove the reference: 404 (space deleted
    // server-side) and 403 (this actor may not open the space, so the id was
    // registered by another account on this device).
    const problem = parseSdkProblemDetails(result.reason);
    if (problem?.status === 404 || isSpaceAccessDenied(result.reason)) {
      removeRegisteredSpace(tenantId, entry.spaceId);
      return;
    }
    retainedSpaceCount += 1;
    unavailableSpaceIds.push(entry.spaceId);
  });

  // Nothing here belongs to this account: it has no knowledge base on this
  // device. Give it its own instead of leaving the sidebar occupied by
  // references it can never open.
  if (retainedSpaceCount === 0) {
    const seeded = await ensureDefaultSpace(tenantId);
    grouped.personal.push(buildKnowledgeBase(seeded.entry, seeded.name));
  }

  if (unavailableSpaceIds.length > 0) {
    console.warn(
      `[KnowledgebaseDocumentApiBridge] ${unavailableSpaceIds.length} registered knowledge base(s) unavailable this round and hidden from the sidebar: ${unavailableSpaceIds.join(', ')}`,
    );
  }

  return { ...grouped, unavailableSpaceIds };
}

export async function createKnowledgeBase(
  kb: Partial<KnowledgeBase>,
): Promise<KnowledgeBase> {
  if (getEphemeralWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
  }

  const tenantId = requireTenantId();
  const client = requireSdkClient();
  const kbType = kb.type ?? 'personal';

  const created = await client.knowledge.spaces.create({
    name: kb.title?.trim() || 'Untitled',
    description: null,
  });

  const entry: RegisteredKnowledgebaseSpace = {
    spaceId: created.id,
    kbType,
    icon: kb.icon ?? '📁',
    createdAt: new Date().toISOString(),
  };
  upsertRegisteredSpace(tenantId, entry);

  await KnowledgeSpaceSettingsService.applyKnowledgeSpaceSettings(created.id, kb);

  const modelSettings = await KnowledgeSpaceSettingsService.loadKnowledgeSpaceModelSettings(created.id);
  const permissionSettings = await KnowledgeSpaceSettingsService.loadKnowledgeSpacePermissionSettings(created.id);

  return {
    id: String(created.id),
    title: created.name,
    icon: entry.icon,
    type: kbType,
    ...modelSettings,
    ...permissionSettings,
  };
}

export async function updateKnowledgeBase(
  id: string,
  updates: Partial<KnowledgeBase>,
): Promise<KnowledgeBase> {
  if (getEphemeralWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
  }

  const tenantId = requireTenantId();
  const spaceId = spaceIdFromKbId(id);
  const client = requireSdkClient();
  const space = await client.knowledge.spaces.retrieve(spaceId);

  if (updates.title && updates.title.trim() !== space.name) {
    await client.knowledge.spaces.update(spaceId, {
      name: updates.title.trim(),
    });
  }

  await KnowledgeSpaceSettingsService.applyKnowledgeSpaceSettings(spaceId, updates);

  const registryPatch: Parameters<typeof updateRegisteredSpace>[2] = {};
  if (updates.type !== undefined) {
    registryPatch.kbType = updates.type;
  }
  if (updates.icon !== undefined) {
    registryPatch.icon = updates.icon;
  }
  if (updates.avatar !== undefined) {
    registryPatch.avatar = updates.avatar;
  }
  if (Object.keys(registryPatch).length > 0) {
    updateRegisteredSpace(tenantId, spaceId, registryPatch);
  }

  const modelSettings = await KnowledgeSpaceSettingsService.loadKnowledgeSpaceModelSettings(spaceId);
  const permissionSettings = await KnowledgeSpaceSettingsService.loadKnowledgeSpacePermissionSettings(spaceId);
  const registry = readRegisteredSpaces(tenantId).find((entry) => entry.spaceId === spaceId);
  const refreshedSpace = await client.knowledge.spaces.retrieve(spaceId);
  return {
    id,
    title: refreshedSpace.name,
    icon: registry?.icon ?? '📘',
    type: registry?.kbType ?? 'personal',
    avatar: registry?.avatar,
    ...modelSettings,
    ...permissionSettings,
  };
}

export async function hydrateKnowledgeBase(kb: KnowledgeBase): Promise<KnowledgeBase> {
  return KnowledgeSpaceSettingsService.hydrateKnowledgeBaseFromApi(kb);
}

export async function deleteKnowledgeBase(id: string): Promise<boolean> {
  if (getEphemeralWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
  }

  const tenantId = requireTenantId();
  const spaceId = spaceIdFromKbId(id);
  const client = requireSdkClient();
  await client.knowledge.spaces.delete(spaceId);
  removeRegisteredSpace(tenantId, spaceId);
  return true;
}

export async function getDocuments(kbId: string): Promise<(FolderNode | DocumentMeta)[]> {
  const spaceId = requireWorkspaceSpaceId(kbId);
  await ensureKnowledgeBrowserFolderLoaded(spaceId, null);
  const nodes = await listLoadedKnowledgeBrowserNodes(spaceId);
  const grouped = groupDocumentsByParent(nodes, kbId);

  let favoriteNodeIds: Set<string> | undefined;
  if (isKnowledgebaseDriveApiAvailable()) {
    try {
      const client = requireSdkClient();
      const space = await client.knowledge.spaces.retrieve(spaceId);
      const driveSpaceId = space.driveSpaceId?.trim();
      if (driveSpaceId) {
        favoriteNodeIds = await listDriveFavoriteNodeIds(driveSpaceId);
      }
    } catch {
      // Pin state is optional when Drive favorites are unavailable.
    }
  }

  await enrichDocumentTreeMetadata(grouped, nodes, kbId, async (conceptRowId) => {
    try {
      return await readOkfConceptTags(conceptRowId);
    } catch {
      return undefined;
    }
  }, favoriteNodeIds);

  await appendAuthoredNotes(grouped, nodes, kbId);
  return grouped;
}

/**
 * Merge notes created inside the knowledge base (documents without a drive
 * node) into the file tree.
 *
 * The browser tree only lists drive nodes, so a note created with "新的笔记"
 * — or any document the editor owns — used to vanish from the workspace as
 * soon as the list refreshed, leaving no way to reopen it. Notes that the
 * browser already represents (a document linked to a drive file) are skipped so
 * the tree never shows the same document twice.
 *
 * Newest notes come first: the list is ascending by document id, and a freshly
 * created note is appended last by the API.
 */
async function appendAuthoredNotes(
  roots: (FolderNode | DocumentMeta)[],
  nodes: KnowledgeBrowserNode[],
  kbId: string,
): Promise<void> {
  const represented = new Set<string>();
  for (const node of nodes) {
    represented.add(node.id);
    represented.add(resolveBrowserDocumentId(node, kbId));
    if (node.documentId) represented.add(String(node.documentId));
  }

  let notes;
  try {
    notes = await listSpaceNotes(kbId);
  } catch {
    // The drive tree stays usable when the notes listing is unavailable.
    return;
  }

  for (let index = notes.length - 1; index >= 0; index -= 1) {
    const note = notes[index];
    if (represented.has(note.id)) {
      continue;
    }
    const isMarkdown = note.mimeType?.trim().toLowerCase() === 'text/markdown';
    const noteDocument: DocumentMeta = {
      id: note.id,
      title: note.title || note.id,
      type: isMarkdown ? 'markdown' : 'richtext',
      kbId,
      parentId: null,
      updatedAt: '',
      author: 'Knowledgebase',
    };
    roots.unshift(noteDocument);
  }
}

export async function ensureFolderChildrenLoaded(
  kbId: string,
  folderId: string | null,
): Promise<void> {
  const spaceId = requireWorkspaceSpaceId(kbId);
  await ensureKnowledgeBrowserFolderLoaded(spaceId, folderId);
}

export function hasMoreFolderChildren(kbId: string, folderId: string | null): boolean {
  const spaceId = requireWorkspaceSpaceId(kbId);
  return hasMoreKnowledgeBrowserNodes(spaceId, folderId);
}

export async function loadMoreFolderChildren(
  kbId: string,
  folderId: string | null,
): Promise<boolean> {
  const spaceId = requireWorkspaceSpaceId(kbId);
  return loadMoreKnowledgeBrowserNodes(spaceId, folderId);
}

/**
 * Read an indexed document's text through the RAG chunk index.
 *
 * Only a RAG space answers `POST /knowledge/retrievals`; the gateway rejects the
 * call for OKF-bundle and external spaces. Those spaces are served by the
 * authoritative document-content API or their Drive file instead, so this
 * helper reports "no chunk content" rather than sending a request that cannot
 * succeed.
 *
 * @param spaceId - owning knowledge space.
 * @param documentId - indexed document id.
 * @param title - document title used as the retrieval query.
 * @returns the matching chunk text, or null when the space is not RAG or no
 *   chunk belongs to this document.
 */
async function readIndexedDocumentContent(
  spaceId: string,
  documentId: string,
  title: string,
): Promise<string | null> {
  if (await resolveKnowledgeSpaceKnowledgeMode(spaceId) !== 'rag') {
    return null;
  }

  const client = requireSdkClient();
  const result = await client.knowledge.retrievals.create({
    query: title,
    bindings: [{ spaceId: String(spaceId), priority: 0, topK: 8 }],
    includeCitations: false,
    includeTrace: false,
    topK: 8,
  });
  // Only a hit that actually belongs to this document may be served: falling
  // back to the top-ranked chunk of any document returned foreign content that
  // was then cached as this document's body.
  const hit =
    result.hits.find((entry) => entry.documentId === documentId)
    ?? result.hits.find((entry) => entry.title === title);
  const content = hit?.content?.trim();
  return content ? content : null;
}

/** Node mime types whose Drive object is text this editor may open as Markdown. */
const TEXTUAL_BROWSER_NODE_EXTENSIONS = /\.(?:md|markdown|txt|text|json|ya?ml|csv|html?|css|tsx?|jsx?|py|rs|go|java|toml|ini|log)$/i;

function isTextualBrowserNode(node: KnowledgeBrowserNode): boolean {
  const mimeType = (node.mimeType ?? '').trim().toLowerCase();
  if (mimeType.startsWith('text/')) {
    return true;
  }
  if (mimeType === '' || mimeType === 'application/json') {
    return TEXTUAL_BROWSER_NODE_EXTENSIONS.test(node.path);
  }
  return false;
}

async function readBrowserNodeContent(
  node: KnowledgeBrowserNode,
  spaceId: string,
): Promise<string | null> {
  if (node.conceptId) {
    return readOkfConceptMarkdown(spaceId, node.conceptId, resolveDriveNodeId(node));
  }
  if (node.documentId) {
    const indexed = await readIndexedDocumentContent(
      spaceId,
      String(node.documentId),
      node.name,
    );
    if (indexed) {
      return indexed;
    }
  }
  // The browser node is a Drive object. Textual files — including every source
  // file of an OKF bundle — are served from that object, which is the only
  // content source for a space without a RAG chunk index.
  const driveNodeId = resolveDriveNodeId(node);
  if (driveNodeId && isTextualBrowserNode(node)) {
    const text = await readDriveNodeTextContent(driveNodeId);
    if (text !== null) {
      return text;
    }
  }
  return null;
}

async function resolveBrowserNodeByDocumentId(
  documentId: string,
): Promise<{ spaceId: string; node: KnowledgeBrowserNode } | null> {
  const ephemeralSpaceId = getEphemeralWorkspaceSpaceId();
  if (ephemeralSpaceId !== null) {
    const okfRef = parseOkfDocumentId(documentId);
    if (okfRef) {
      assertEphemeralWorkspaceSpaceId(okfRef.spaceId);
    }
    return resolveBrowserNodeInSpace(ephemeralSpaceId, documentId);
  }

  const tenantId = getKnowledgebaseTenantId();
  if (!tenantId) {
    return null;
  }

  const okfRef = parseOkfDocumentId(documentId);
  if (okfRef) {
    return resolveBrowserNodeInSpace(okfRef.spaceId, documentId);
  }

  const documentIdForSdk = parseSafeNumericDocumentId(documentId);
  if (documentIdForSdk) {
    try {
      const document = await requireSdkClient().knowledge.documents.retrieve(documentIdForSdk);
      const resolved = await resolveBrowserNodeInSpace(document.spaceId, documentId);
      if (resolved) {
        return resolved;
      }
    } catch {
      // Fall through to registry scan.
    }
  }

  for (const entry of readRegisteredSpaces(tenantId)) {
    const resolved = await resolveBrowserNodeInSpace(entry.spaceId, documentId);
    if (resolved) {
      return resolved;
    }
  }

  return null;
}

async function resolveBrowserNodeInSpace(
  spaceId: string,
  documentId: string,
): Promise<{ spaceId: string; node: KnowledgeBrowserNode } | null> {
  try {
    const kbId = String(spaceId);
    const loaded = getLoadedKnowledgeBrowserNodes(spaceId);
    const cached = findKnowledgeBrowserNodeByDocumentId(loaded, documentId, kbId);
    if (cached) {
      return { spaceId, node: cached };
    }

    const documentIdForSdk = parseSafeNumericDocumentId(documentId);
    if (documentIdForSdk && shouldPersistBrowserWorkspaceState()) {
      try {
        const document = await requireSdkClient().knowledge.documents.retrieve(documentIdForSdk);
        if (document.spaceId === spaceId && document.originalFileDriveNodeId) {
          const byDriveNode = loaded.find(
            (candidate) =>
              candidate.driveNodeId === document.originalFileDriveNodeId
              || candidate.id === document.originalFileDriveNodeId,
          );
          if (byDriveNode) {
            return { spaceId, node: byDriveNode };
          }
        }
      } catch {
        // Continue with paginated browser search.
      }
    }

    const found = await scanKnowledgeBrowserNodes(spaceId, (candidate) => {
      return findKnowledgeBrowserNodeByDocumentId([candidate], documentId, kbId) !== null
        || (documentIdForSdk !== null && candidate.documentId === documentIdForSdk);
    });
    if (found) {
      return { spaceId, node: found };
    }
  } catch {
    // Skip spaces that fail to list.
  }
  return null;
}

export async function getDocumentContent(id: string): Promise<string> {
  const persistBrowserState = shouldPersistBrowserWorkspaceState();
  const tenantId = persistBrowserState ? requireTenantId() : null;
  const numericDocumentId = parseSafeNumericDocumentId(id);

  if (numericDocumentId !== null && persistBrowserState) {
    let authoritative;
    try {
      authoritative = await fetchKnowledgeDocumentContent(numericDocumentId);
    } catch (error) {
      // A denial from the authoritative read must never be answered from the
      // local copy: drop the copy and surface the denial instead.
      if (tenantId !== null) {
        removeLocalDocumentContent(tenantId, id);
      }
      throw error;
    }
    if (authoritative?.contentMarkdown !== undefined) {
      const contentVersion = authoritative.contentVersion?.trim()
        || authoritative.contentSource?.trim()
        || `document-${authoritative.documentId}`;
      if (tenantId !== null) {
        const cached = readLocalDocumentContent(tenantId, id, contentVersion);
        if (cached !== undefined) {
          return cached;
        }
      }
      if (authoritative.contentMarkdown.trim()) {
        if (tenantId !== null) {
          writeLocalDocumentContent(tenantId, id, authoritative.contentMarkdown, contentVersion);
        }
        return authoritative.contentMarkdown;
      }
    }
  }

  if (tenantId !== null) {
    const cached = readLocalDocumentContent(tenantId, id);
    if (cached !== undefined) {
      const recent = readRecentDocuments(tenantId).find((entry) => entry.id === id);
      if (recent) {
        touchRecentDocument(tenantId, recent);
      }
      return cached;
    }
  }

  const okfRef = parseOkfDocumentId(id);
  if (okfRef) {
    try {
      assertEphemeralWorkspaceSpaceId(okfRef.spaceId);
      const content = await readOkfConceptMarkdown(okfRef.spaceId, okfRef.conceptRowId);
      if (tenantId !== null) {
        writeLocalDocumentContent(tenantId, id, content, `okf-${okfRef.conceptRowId}`);
      }
      return content;
    } catch (error) {
      console.warn('[KnowledgebaseDocumentApiBridge] okf concept read failed.', error);
    }
  }

  const client = requireSdkClient();
  if (numericDocumentId !== null && persistBrowserState) {
    try {
      const document = await client.knowledge.documents.retrieve(String(numericDocumentId));
      assertEphemeralWorkspaceSpaceId(String(document.spaceId));
      trackRecentDocument({
        id: String(document.id),
        title: document.title,
        type: 'markdown',
        kbId: String(document.spaceId),
        author: 'Knowledgebase',
      });
      const indexed = await readIndexedDocumentContent(
        document.spaceId,
        String(document.id),
        document.title,
      );
      if (indexed) {
        if (tenantId !== null) {
          writeLocalDocumentContent(tenantId, id, indexed, `indexed-${document.id}`);
        }
        return indexed;
      }
      // A document without any persisted version (e.g. a freshly created note
      // opened from another device, with no local cache) is EMPTY — never seed
      // the editor with fabricated placeholder text, or the autosave would
      // persist that placeholder as the note's first real version.
      if (!document.currentVersionId) {
        return '';
      }
      // Only a RAG space reads its body out of the chunk index; an OKF-bundle
      // space has no chunk body to fall back on, so it continues to the
      // browser-node (Drive) read below instead of a placeholder.
      if (await resolveKnowledgeSpaceKnowledgeMode(document.spaceId) === 'rag') {
        return `# ${document.title}\n\nThis document is indexed in Knowledgebase but no retrieval chunks were found yet.`;
      }
    } catch {
      // Fall through to browser-node resolution.
    }
  }

  const browserMatch = await resolveBrowserNodeByDocumentId(id);
  if (browserMatch) {
    try {
      const content = await readBrowserNodeContent(browserMatch.node, browserMatch.spaceId);
      if (content) {
        if (tenantId !== null) {
          writeLocalDocumentContent(tenantId, id, content);
        }
        return content;
      }
    } catch (error) {
      console.warn('[KnowledgebaseDocumentApiBridge] browser node read failed.', error);
    }
  }

  throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_RESOLVE_FAILED);
}

function parseSafeNumericDocumentId(id: string): string | null {
  if (!/^[0-9]+$/.test(id.trim())) {
    return null;
  }
  const documentId = id.trim();
  if (/^0+$/u.test(documentId)) {
    return null;
  }
  return documentId;
}

export interface SaveDocumentContentOptions {
  /** Server `currentVersionId` observed when the document was last opened or last saved by
   * this client. When provided, the save fails with `DOCUMENT_CONFLICT` if the document
   * was updated elsewhere in the meantime, so concurrent editors are never silently
   * overwritten (last-writer-wins on stale content). */
  baseVersionId?: string | null;
}

export interface SaveDocumentContentResult {
  saved: boolean;
  /** Server `currentVersionId` after the save; store it as the next `baseVersionId`. */
  currentVersionId: string | null;
}

export async function saveDocumentContent(
  id: string,
  content: string,
  options?: SaveDocumentContentOptions,
): Promise<SaveDocumentContentResult> {
  const persistBrowserState = shouldPersistBrowserWorkspaceState();
  const tenantId = persistBrowserState ? requireTenantId() : null;

  const okfRef = parseOkfDocumentId(id);
  if (okfRef) {
    assertEphemeralWorkspaceSpaceId(okfRef.spaceId);
    const client = requireSdkClient();
    const concept = await client.knowledge.okf.concepts.retrieve(String(okfRef.conceptRowId));
    await client.knowledge.okf.concepts.update({
      spaceId: okfRef.spaceId,
      conceptId: concept.conceptId,
      markdown: content,
      publish: true,
    });
    invalidateKnowledgeBrowserNodeCacheForSpaceIds(okfRef.spaceId);
    return { saved: true, currentVersionId: null };
  }

  const ephemeralSpaceId = getEphemeralWorkspaceSpaceId();
  const scopedBrowserMatch = ephemeralSpaceId !== null
    ? await resolveBrowserNodeByDocumentId(id)
    : null;
  const numericDocumentId = ephemeralSpaceId !== null
    ? scopedBrowserMatch?.node.documentId ?? null
    : await resolveNumericDocumentId(id);
  if (!numericDocumentId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_NOT_INDEXED);
  }

  let currentVersionId: string | null = null;
  try {
    const client = requireSdkClient();
    let spaceId: string;
    let title: string;
    if (ephemeralSpaceId !== null) {
      if (!scopedBrowserMatch) {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_RESOLVE_FAILED);
      }
      spaceId = ephemeralSpaceId;
      title = scopedBrowserMatch.node.name;
    } else {
      const document = await client.knowledge.documents.retrieve(String(numericDocumentId));
      spaceId = String(document.spaceId);
      title = document.title;
      currentVersionId = typeof document.currentVersionId === 'string'
        ? document.currentVersionId
        : null;
      if (
        !options?.baseVersionId
        || !currentVersionId
        || options.baseVersionId === currentVersionId
      ) {
        // No conflict: the observed base matches the current server version (or the
        // server does not expose a version id yet).
      } else {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_CONFLICT, {
          cause: 'document was updated elsewhere since it was opened',
        });
      }
    }
    const job = await client.knowledge.ingests.create({
      spaceId,
      title,
      payloadMarkdown: content,
      // Every save attempt is a distinct operation: the ingest create-or-get
      // deduplicates by key, so a deterministic per-document key made every
      // later edit return the first job and silently drop the edit.
      idempotencyKey: buildSaveDocumentIngestIdempotencyKey(numericDocumentId),
    });
    const finalJob = job.state === 'succeeded' ? job : await waitForIngestJob(job.id);
    if (finalJob.state !== 'succeeded') {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.INGEST_FAILED, {
        cause: finalJob.errorMessage ?? undefined,
      });
    }
    if (tenantId !== null) {
      writeLocalDocumentContent(
        tenantId,
        id,
        content,
        `ingest-${finalJob.id}`,
      );
    }
    invalidateKnowledgeBrowserNodeCacheForSpaceIds(spaceId);
    // Refresh the server version id so the next save compares against the version this
    // client just created instead of the pre-save one.
    if (currentVersionId !== null && ephemeralSpaceId === null) {
      try {
        const refreshed = await client.knowledge.documents.retrieve(String(numericDocumentId));
        if (typeof refreshed.currentVersionId === 'string') {
          currentVersionId = refreshed.currentVersionId;
        }
      } catch {
        // The ingest succeeded, so the version advanced; only its id is unknown.
        // Reporting the stale pre-save id would make the NEXT save falsely
        // conflict, so report "unknown" (null) and skip the check once instead.
        currentVersionId = null;
      }
    }
  } catch (error) {
    // Typed failures (conflict, ingest failure, validation, API unavailability)
    // keep their canonical codes so `isDocumentConflictError` and the UI's
    // merge prompt keep working; only unexpected errors become a generic
    // OPERATION_FAILED.
    if (isKnowledgebaseAppError(error)) {
      throw error;
    }
    const detail = error instanceof Error ? error.message : String(error);
    throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED, { cause: detail });
  }

  return { saved: true, currentVersionId };
}

function buildSaveDocumentIngestIdempotencyKey(numericDocumentId: string): string {
  const randomSuffix = Math.random().toString(36).slice(2, 10);
  return `pc-save-${numericDocumentId}-${Date.now()}-${randomSuffix}`.slice(0, 128);
}

export interface DocumentSaveContext {
  documentId: string;
  title: string;
  spaceId: string;
  /** Server version id to pass back as `saveDocumentContent` `baseVersionId`;
   * null when the document has no persisted version yet or does not expose one. */
  currentVersionId: string | null;
}

/**
 * One-call snapshot used to arm optimistic concurrency when a document is
 * opened: editors record `currentVersionId` as their baseline and pass it to
 * every subsequent `saveDocumentContent` call.
 */
export async function getDocumentSaveContext(id: string): Promise<DocumentSaveContext> {
  const numericDocumentId = await resolveNumericDocumentId(id);
  if (!numericDocumentId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_RESOLVE_FAILED);
  }
  const client = requireSdkClient();
  const document = await client.knowledge.documents.retrieve(String(numericDocumentId));
  assertEphemeralWorkspaceSpaceId(String(document.spaceId));
  return {
    documentId: String(document.id),
    title: document.title,
    spaceId: String(document.spaceId),
    currentVersionId: typeof document.currentVersionId === 'string'
      ? document.currentVersionId
      : null,
  };
}

async function resolveNumericDocumentId(id: string): Promise<string | null> {
  const safeId = parseSafeNumericDocumentId(id);
  if (safeId !== null) {
    return safeId;
  }

  const browserMatch = await resolveBrowserNodeByDocumentId(id);
  if (browserMatch?.node.documentId) {
    return browserMatch.node.documentId;
  }

  return null;
}

export interface DocumentVersionSummary {
  id: string;
  documentId: string;
  versionNo: number;
  sizeBytes: number;
  mimeType?: string | null;
  parseState: string;
  indexState: string;
}

export type KnowledgeDocumentVisibility =
  | 'private'
  | 'space'
  | 'organization'
  | 'public';

export interface DocumentAccessSummary {
  documentId: string;
  spaceId: string;
  title: string;
  visibility: KnowledgeDocumentVisibility;
}

export async function getDocumentAccess(documentId: string): Promise<DocumentAccessSummary> {
  const numericDocumentId = await resolveNumericDocumentId(documentId);
  if (!numericDocumentId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_ID_REQUIRED);
  }

  const client = requireSdkClient();
  const document = await client.knowledge.documents.retrieve(String(numericDocumentId));
  return {
    documentId: document.id,
    spaceId: document.spaceId,
    title: document.title,
    visibility: document.visibility,
  };
}

export async function updateDocumentVisibility(
  documentId: string,
  visibility: KnowledgeDocumentVisibility,
): Promise<DocumentAccessSummary> {
  const numericDocumentId = await resolveNumericDocumentId(documentId);
  if (!numericDocumentId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_ID_REQUIRED);
  }

  const client = requireSdkClient();
  const existing = await client.knowledge.documents.retrieve(String(numericDocumentId));
  const updated = await client.knowledge.documents.update(String(numericDocumentId), {
    spaceId: existing.spaceId,
    title: existing.title,
    mimeType: existing.mimeType,
    language: existing.language,
    visibility,
  });

  return {
    documentId: updated.id,
    spaceId: updated.spaceId,
    title: updated.title,
    visibility: updated.visibility,
  };
}

export async function listDocumentVersions(documentId: string): Promise<DocumentVersionSummary[]> {
  const numericDocumentId = await resolveNumericDocumentId(documentId);
  if (!numericDocumentId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_ID_REQUIRED);
  }

  const client = requireSdkClient();
  const response = await client.knowledge.documents.versions.list(String(numericDocumentId));
  const page = normalizeSdkWorkListPage<{
    id: string;
    documentId: string;
    versionNo: string | number;
    sizeBytes: string | number;
    mimeType?: string | null;
    parseState: string;
    indexState: string;
  }>(response);
  return page.items.map((version) => ({
    id: version.id,
    documentId: version.documentId,
    versionNo: toSafeNumber(version.versionNo),
    sizeBytes: toSafeNumber(version.sizeBytes),
    mimeType: version.mimeType,
    parseState: version.parseState,
    indexState: version.indexState,
  }));
}

async function waitForDriveFolderBrowserNode(
  spaceId: string,
  kbId: string,
  driveNodeId: string,
): Promise<KnowledgeBrowserNode> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt < 4; attempt += 1) {
    if (attempt > 0) {
      await new Promise((resolve) => setTimeout(resolve, 250));
      invalidateKnowledgeBrowserNodeCacheForKbIds(kbId);
    }

    try {
      const found = await scanKnowledgeBrowserNodes(
        spaceId,
        (candidate) =>
          candidate.driveNodeId === driveNodeId || candidate.id === driveNodeId,
        { fresh: attempt > 0 },
      );
      if (found) {
        return found;
      }
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
    }
  }

  throw lastError ?? new Error(
    `Folder was created in Drive (${driveNodeId}) but is not yet visible in the knowledge browser tree. Refresh the file list.`,
  );
}

export async function placeDocumentInParentFolder(
  documentId: string,
  kbId: string,
  parentId: string | null,
): Promise<void> {
  if (isBlank(parentId)) {
    return;
  }

  invalidateKnowledgeBrowserNodeCacheForKbIds(kbId);
  let lastError: unknown;

  for (let attempt = 0; attempt < 4; attempt += 1) {
    if (attempt > 0) {
      await new Promise((resolve) => setTimeout(resolve, 250));
      invalidateKnowledgeBrowserNodeCacheForKbIds(kbId);
    }

    try {
      await updateDocument(documentId, { parentId });
      return;
    } catch (error) {
      lastError = error;
      const message = error instanceof Error ? error.message : String(error);
      if (!message.includes('browser nodes')) {
        throw error;
      }
    }
  }

  throw lastError instanceof Error
    ? lastError
    : new Error(`Could not place document ${documentId} under parent ${parentId}.`);
}

export async function updateDocument(id: string, updates: Partial<DocumentMeta>): Promise<boolean> {
  if (updates.kbId !== undefined) {
    requireWorkspaceSpaceId(updates.kbId);
    const sourceKbId = await resolveDocumentKbId(id);
    if (sourceKbId && updates.kbId !== sourceKbId) {
      let moved: DocumentMeta | null = null;
      if (parseOkfDocumentId(id)) {
        const okfRef = parseOkfDocumentId(id)!;
        moved = await moveOkfConcept(
          okfRef.spaceId,
          okfRef.conceptRowId,
          updates.kbId,
          readOkfConceptMarkdown,
        );
      } else {
        const browserMatch = await resolveBrowserNodeByDocumentId(id);
        moved = await transferDocumentAcrossKnowledgeBases(
          id,
          sourceKbId,
          updates.kbId,
          updates.parentId ?? null,
          'move',
          {
            sourceNode: browserMatch?.node ?? null,
            ingestTextDocument: ingestTextDocumentAcrossKnowledgeBases,
            deleteSourceDocument: deleteDocument,
          },
        );
      }
      if (updates.parentId?.trim() && moved) {
        await placeDocumentInParentFolder(moved.id, updates.kbId, updates.parentId);
      }
      return true;
    }
  }

  const browserMatch = await resolveBrowserNodeByDocumentId(id);
  const okfRef = parseOkfDocumentId(id);
  let metadataUpdated = false;

  if (updates.tags !== undefined) {
    if (okfRef) {
      await updateOkfConceptTags(
        okfRef.spaceId,
        okfRef.conceptRowId,
        updates.tags,
        readOkfConceptMarkdown,
      );
      metadataUpdated = true;
    } else if (browserMatch) {
      const driveNodeId = resolveDriveNodeId(browserMatch.node);
      if (!driveNodeId || !isKnowledgebaseDriveApiAvailable()) {
        throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
      }
      await writeDriveDocumentTags(driveNodeId, updates.tags);
      metadataUpdated = true;
    } else {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
    }
  }

  if (updates.order !== undefined) {
    if (!browserMatch) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
    }
    const driveNodeId = resolveDriveNodeId(browserMatch.node);
    if (!driveNodeId || !isKnowledgebaseDriveApiAvailable()) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
    }
    await writeDriveDocumentOrder(driveNodeId, updates.order);
    metadataUpdated = true;
  }

  const hasDriveMetadataUpdate =
    updates.parentId !== undefined
    || updates.title !== undefined
    || updates.isPinned !== undefined;

  // Drive-node metadata sync only applies to documents that live in the browser
  // tree. Manual notes (no drive node) fall through to the direct
  // `documents.update` branch below — the previous early throw made that
  // fallback unreachable, so manual note titles could never be renamed.
  if (hasDriveMetadataUpdate && browserMatch) {
    if (!isKnowledgebaseDriveApiAvailable()) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
    }
    // Pass the optional fields through untouched: normalizing a missing title
    // or parent to '' made applyDriveBrowserNodeUpdates (whose guards are
    // `!== undefined` checks) rename the node to an empty name and move it to
    // the drive space root on every rename-only or pin-only update.
    await applyDriveBrowserNodeUpdates(String(browserMatch.spaceId), browserMatch.node, {
      title: updates.title,
      parentId: updates.parentId,
      isPinned: updates.isPinned,
    });
  }

  const numericDocumentId = await resolveNumericDocumentId(id);

  if (numericDocumentId && updates.title && !browserMatch) {
    const client = requireSdkClient();
    const existing = await client.knowledge.documents.retrieve(String(numericDocumentId));
    await client.knowledge.documents.update(String(numericDocumentId), {
      spaceId: existing.spaceId,
      title: updates.title ?? existing.title,
      mimeType: updates.type === 'markdown'
        ? 'text/markdown'
        : (existing.mimeType ?? 'text/plain'),
    });
  } else if (numericDocumentId && updates.type) {
    const client = requireSdkClient();
    const existing = await client.knowledge.documents.retrieve(String(numericDocumentId));
    await client.knowledge.documents.update(String(numericDocumentId), {
      spaceId: existing.spaceId,
      title: existing.title,
      mimeType: updates.type === 'markdown'
        ? 'text/markdown'
        : (existing.mimeType ?? 'text/plain'),
    });
  }

  if (updates.content !== undefined) {
    await saveDocumentContent(id, updates.content, { baseVersionId: null });
  }

  // `hasDriveMetadataUpdate` alone must not count as an applied update: when
  // the document has no browser node (e.g. a manually created note), the
  // drive-sync branch above never ran, so reporting success for an unperformed
  // move misleads callers like placeDocumentInParentFolder's retry loop.
  const didUpdate =
    metadataUpdated
    || (hasDriveMetadataUpdate && Boolean(browserMatch))
    || numericDocumentId !== null
    || updates.content !== undefined;
  if (didUpdate) {
    invalidateKnowledgeBrowserNodeCacheForSpaceIds(
      browserMatch?.spaceId,
      okfRef?.spaceId,
    );
    invalidateKnowledgeBrowserNodeCacheForKbIds(updates.kbId);
  }

  return didUpdate;
}

export async function searchAll(query: string): Promise<{
  kbs: KnowledgeBase[];
  docs: DocumentMeta[];
}> {
  if (getEphemeralWorkspaceSpaceId() !== null) {
    return { kbs: [], docs: [] };
  }

  const tenantId = requireTenantId();
  const client = requireSdkClient();
  const trimmedQuery = query.trim();
  if (!trimmedQuery) {
    return { kbs: [], docs: [] };
  }

  const grouped = await getKnowledgeBases();
  const allKbs = [...grouped.team, ...grouped.personal, ...grouped.public];
  const lowerQuery = trimmedQuery.toLowerCase();
  const matchedKbs = allKbs.filter((kb) => kb.title.toLowerCase().includes(lowerQuery));

  const registry = readRegisteredSpaces(tenantId);
  if (registry.length === 0) {
    return { kbs: matchedKbs, docs: [] };
  }

  // `POST /knowledge/retrievals` is RAG-only: one OKF-bundle or external space
  // in the binding list makes the gateway reject the whole search (40003), so
  // only RAG spaces are searched through chunks. OKF-bundle spaces answer the
  // same query through the OKF query API below.
  const { rag: ragSpaceIds, okfBundle: okfSpaceIds } = await partitionKnowledgeSpaceIdsByMode(
    registry.map((entry) => String(entry.spaceId)),
  );

  const bindings = ragSpaceIds.map((spaceId, index) => ({
    spaceId,
    priority: index,
    topK: 20,
  }));

  if (bindings.length > 0) {
    try {
      const result = await client.knowledge.retrievals.create({
        query: trimmedQuery,
        bindings,
        includeCitations: true,
        includeTrace: false,
        topK: 20,
      });

      const docs: DocumentMeta[] = [];
      const seenDocumentIds = new Set<string>();

      for (const hit of result.hits) {
        if (seenDocumentIds.has(hit.documentId)) {
          continue;
        }
        seenDocumentIds.add(hit.documentId);

        const kbId = hit.spaceId;
        docs.push({
          id: hit.documentId,
          title: hit.title?.trim() || 'Untitled',
          type: 'markdown',
          kbId,
          updatedAt: new Date().toISOString(),
          author: 'Knowledgebase',
          content: hit.content,
        });
      }

      if (docs.length > 0) {
        return { kbs: matchedKbs, docs };
      }
    } catch (error) {
      console.warn('[KnowledgebaseDocumentApiBridge] retrieval search failed.', error);
    }
  }

  const okfDocs = await searchOkfConceptDocs(trimmedQuery, okfSpaceIds);
  return { kbs: matchedKbs, docs: okfDocs };
}

async function searchOkfConceptDocs(
  query: string,
  spaceIds: readonly string[],
): Promise<DocumentMeta[]> {
  const client = requireSdkClient();
  const docs: DocumentMeta[] = [];

  for (const spaceId of spaceIds.slice(0, 2)) {
    try {
      const result = await client.knowledge.okf.queries.create({
        spaceId,
        query,
      });
      const snippet = result.answerMarkdown.trim();
      if (!snippet) {
        continue;
      }
      docs.push({
        id: `okf:${spaceId}:query-${docs.length}`,
        title: `OKF ? ${query}`,
        type: 'markdown',
        kbId: String(spaceId),
        updatedAt: new Date().toISOString(),
        author: 'Knowledgebase',
        content: snippet,
      });
    } catch {
      // Skip spaces without initialized OKF bundles.
    }
  }

  return docs;
}

function trackRecentDocument(doc: Pick<DocumentMeta, 'id' | 'title' | 'type' | 'kbId' | 'author'>): void {
  if (!shouldPersistBrowserWorkspaceState()) {
    return;
  }

  const tenantId = getKnowledgebaseTenantId();
  if (!tenantId || doc.type === 'folder') {
    return;
  }

  touchRecentDocument(tenantId, {
    id: doc.id,
    title: doc.title,
    type: doc.type,
    kbId: doc.kbId,
    author: doc.author,
    updatedAt: new Date().toISOString(),
  });
}

function buildCreateDocumentIngestIdempotencyKey(spaceId: string, title: string): string {
  const normalizedTitle = title.trim().replace(/[^a-zA-Z0-9._-]+/g, '-').slice(0, 48);
  const randomSuffix = Math.random().toString(36).slice(2, 10);
  return `pc-create-${spaceId}-${normalizedTitle}-${Date.now()}-${randomSuffix}`.slice(0, 128);
}

async function collectRecentDocumentsFromSpaces(limit: number): Promise<DocumentMeta[]> {
  const tenantId = requireTenantId();
  const registry = readRegisteredSpaces(tenantId);
  const collected: DocumentMeta[] = [];
  const perSpacePageSize = Math.min(Math.max(limit, 1), 20);

  for (const entry of registry) {
    try {
      const page = await listKnowledgeBrowserNodesPage(entry.spaceId, null, {
        pageSize: perSpacePageSize,
      });
      for (const node of page.items) {
        if (node.nodeType === 'folder' || node.nodeType === 'virtual_folder') {
          continue;
        }
        collected.push({
          id: resolveBrowserDocumentId(node, String(entry.spaceId)),
          title: node.name,
          type: mapNodeType(node),
          kbId: String(entry.spaceId),
          parentId: node.parentId ?? null,
          updatedAt: node.updatedAt,
          author: 'Knowledgebase',
          size: formatBytes(node.sizeBytes ?? undefined),
        });
      }
    } catch {
      // Skip spaces that fail to list.
    }
  }

  return collected
    .sort((left, right) => new Date(right.updatedAt || 0).getTime() - new Date(left.updatedAt || 0).getTime())
    .slice(0, limit);
}

export async function getRecentDocuments(limit: number = 8): Promise<DocumentMeta[]> {
  if (!shouldPersistBrowserWorkspaceState()) {
    return [];
  }

  const tenantId = requireTenantId();
  const cached = readRecentDocuments(tenantId)
    .filter((entry) => entry.type !== 'folder')
    .slice(0, limit)
    .map((entry) => ({
      id: entry.id,
      title: entry.title,
      type: entry.type,
      kbId: entry.kbId,
      updatedAt: entry.updatedAt,
      author: entry.author ?? 'Knowledgebase',
    }));

  if (cached.length >= limit) {
    return cached;
  }

  const remote = await collectRecentDocumentsFromSpaces(limit);
  const merged = new Map<string, DocumentMeta>();
  for (const doc of [...cached, ...remote]) {
    merged.set(doc.id, doc);
  }

  return Array.from(merged.values())
    .sort((left, right) => new Date(right.updatedAt || 0).getTime() - new Date(left.updatedAt || 0).getTime())
    .slice(0, limit);
}

export async function touchDocument(id: string): Promise<boolean> {
  if (!shouldPersistBrowserWorkspaceState()) {
    return false;
  }

  const tenantId = requireTenantId();
  const existing = readRecentDocuments(tenantId).find((entry) => entry.id === id);
  if (existing) {
    touchRecentDocument(tenantId, existing);
    return true;
  }

  const okfRef = parseOkfDocumentId(id);
  if (okfRef) {
    try {
      const client = requireSdkClient();
      const concept = await client.knowledge.okf.concepts.retrieve(String(okfRef.conceptRowId));
      trackRecentDocument({
        id,
        title: concept.title,
        type: 'markdown',
        kbId: String(okfRef.spaceId),
        author: 'Knowledgebase',
      });
      return true;
    } catch {
      return false;
    }
  }

  const documentIdForSdk = parseSafeNumericDocumentId(id);
  if (!documentIdForSdk) {
    return false;
  }

  try {
    const client = requireSdkClient();
    const document = await client.knowledge.documents.retrieve(documentIdForSdk);
    trackRecentDocument({
      id: String(document.id),
      title: document.title,
      type: 'markdown',
      kbId: String(document.spaceId),
      author: 'Knowledgebase',
    });
    return true;
  } catch {
    return false;
  }
}

export async function createDocument(doc: Partial<DocumentMeta>): Promise<DocumentMeta> {
  const persistBrowserState = shouldPersistBrowserWorkspaceState();
  const tenantId = persistBrowserState ? requireTenantId() : null;
  const client = requireSdkClient();
  if (!doc.kbId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.KB_ID_REQUIRED);
  }
  const spaceId = requireRegisteredOrEphemeralWorkspaceSpace(doc.kbId);

  if (doc.type === 'folder') {
    if (!isKnowledgebaseDriveApiAvailable()) {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_DRIVE);
    }

    const parentDriveNodeId = await resolveKnowledgeBrowserParentDriveNodeId(
      doc.kbId,
      doc.parentId ?? null,
    );
    const { driveNodeId, nodeName } = await createKnowledgeDriveFolder({
      kbId: doc.kbId,
      nodeName: doc.title?.trim() || 'New Folder',
      parentDriveNodeId,
    });
    invalidateKnowledgeBrowserNodeCacheForKbIds(doc.kbId);

    const spaceId = spaceIdFromKbId(doc.kbId);
    const browserNode = await waitForDriveFolderBrowserNode(spaceId, doc.kbId, driveNodeId);
    const mapped = mapBrowserNodeToDocument(browserNode, doc.kbId);
    if (mapped.type !== 'folder') {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED, { cause: nodeName });
    }

    return {
      id: mapped.id,
      title: mapped.title,
      type: 'folder',
      kbId: doc.kbId,
      parentId: mapped.parentId ?? doc.parentId ?? null,
      updatedAt: mapped.updatedAt ?? new Date().toISOString(),
      author: mapped.author ?? 'Knowledgebase',
    };
  }

  const title = doc.title?.trim() || 'Untitled';
  const content = doc.content ?? '';
  if (!isBlank(content)) {
    const job = await client.knowledge.ingests.create({
      spaceId,
      title,
      payloadMarkdown: content,
      idempotencyKey: buildCreateDocumentIngestIdempotencyKey(spaceId, title),
    });
    const finalJob = job.state === 'succeeded' ? job : await waitForIngestJob(job.id);
    if (finalJob.state !== 'succeeded') {
      throwKnowledgebaseError(KnowledgebaseErrorCodes.INGEST_FAILED, {
        cause: finalJob.errorMessage ?? undefined,
      });
    }
    const document = await resolveIngestedDocument(spaceId, title);
    const createdDoc: DocumentMeta = {
      id: String(document.id),
      title: document.title,
      type: doc.type ?? 'richtext',
      kbId: doc.kbId,
      parentId: doc.parentId ?? null,
      updatedAt: new Date().toISOString(),
      author: doc.author ?? 'Knowledgebase',
      content,
    };
    if (tenantId !== null) {
      writeLocalDocumentContent(tenantId, createdDoc.id, content, `ingest-${finalJob.id}`);
    }
    trackRecentDocument(createdDoc);
    if (doc.parentId) {
      await placeDocumentInParentFolder(createdDoc.id, doc.kbId, doc.parentId);
    }
    invalidateKnowledgeBrowserNodeCacheForKbIds(doc.kbId);
    return createdDoc;
  }

  const created = await client.knowledge.documents.create({
    spaceId,
    title,
    mimeType: doc.type === 'markdown' ? 'text/markdown' : 'text/plain',
  });
  const createdDoc: DocumentMeta = {
    id: String(created.id),
    title: created.title,
    type: doc.type ?? 'richtext',
    kbId: doc.kbId,
    parentId: doc.parentId ?? null,
    updatedAt: new Date().toISOString(),
    author: doc.author ?? 'Knowledgebase',
    content,
  };

  trackRecentDocument(createdDoc);
  if (tenantId !== null) {
    writeLocalDocumentContent(tenantId, createdDoc.id, content, `document-${created.id}`);
  }
  if (doc.parentId) {
    await placeDocumentInParentFolder(createdDoc.id, doc.kbId, doc.parentId);
  }
  invalidateKnowledgeBrowserNodeCacheForKbIds(doc.kbId);
  return createdDoc;
}

async function resolveDocumentKbId(id: string): Promise<string | null> {
  const okfRef = parseOkfDocumentId(id);
  if (okfRef) {
    assertEphemeralWorkspaceSpaceId(String(okfRef.spaceId));
    return String(okfRef.spaceId);
  }

  const browserMatch = await resolveBrowserNodeByDocumentId(id);
  if (browserMatch) {
    return String(browserMatch.spaceId);
  }

  const documentIdForSdk = parseSafeNumericDocumentId(id);
  if (documentIdForSdk) {
    try {
      const document = await requireSdkClient().knowledge.documents.retrieve(documentIdForSdk);
      assertEphemeralWorkspaceSpaceId(String(document.spaceId));
      return String(document.spaceId);
    } catch {
      return null;
    }
  }

  return null;
}

async function ingestTextDocumentAcrossKnowledgeBases(
  sourceId: string,
  targetKbId: string,
  targetParentId: string | null,
  titleSuffix?: string,
): Promise<DocumentMeta> {
  const browserMatch = await resolveBrowserNodeByDocumentId(sourceId);
  if (browserMatch?.node.nodeType === 'folder' || browserMatch?.node.nodeType === 'virtual_folder') {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
  }

  const content = await getDocumentContent(sourceId);
  const suffix = titleSuffix ?? '';
  let sourceTitle = browserMatch?.node.name ?? 'Untitled';
  let sourceType: DocumentMeta['type'] = browserMatch ? mapNodeType(browserMatch.node) : 'markdown';

  const documentIdForSdk = parseSafeNumericDocumentId(sourceId);
  if (documentIdForSdk) {
    try {
      const document = await requireSdkClient().knowledge.documents.retrieve(documentIdForSdk);
      sourceTitle = document.title;
    } catch {
      // Keep browser-derived title when available.
    }
  }

  const created = await createDocument({
    kbId: targetKbId,
    title: `${sourceTitle}${suffix}`,
    type: sourceType,
    content,
    parentId: targetParentId,
  });

  return created;
}

export async function hydrateDocumentForViewer(doc: DocumentMeta): Promise<DocumentMeta> {
  const browserMatch = await resolveBrowserNodeByDocumentId(doc.id);
  return hydrateDocumentMediaUrl(doc, browserMatch?.node ?? null);
}

export async function copyDocument(
  sourceId: string,
  targetKbId: string,
  targetParentId: string | null,
  options?: { titleSuffix?: string },
): Promise<DocumentMeta> {
  requireRegisteredOrEphemeralWorkspaceSpace(targetKbId);
  const okfRef = parseOkfDocumentId(sourceId);
  if (okfRef) {
    assertEphemeralWorkspaceSpaceId(String(okfRef.spaceId));
    const created = await copyOkfConcept(
      okfRef.spaceId,
      okfRef.conceptRowId,
      targetKbId,
      readOkfConceptMarkdown,
      { titleSuffix: options?.titleSuffix },
    );
    if (targetParentId) {
      await placeDocumentInParentFolder(created.id, targetKbId, targetParentId);
    }
    return created;
  }

  const sourceKbId = await resolveDocumentKbId(sourceId);
  if (!sourceKbId) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.DOCUMENT_RESOLVE_FAILED);
  }

  const browserMatch = await resolveBrowserNodeByDocumentId(sourceId);
  if (browserMatch?.node.nodeType === 'folder' || browserMatch?.node.nodeType === 'virtual_folder') {
    if (targetKbId !== sourceKbId) {
      return transferDocumentAcrossKnowledgeBases(
        sourceId,
        sourceKbId,
        targetKbId,
        targetParentId,
        'copy',
        {
          titleSuffix: options?.titleSuffix ?? ' (Copy)',
          sourceNode: browserMatch.node,
        },
      );
    }
    return copyDriveFolderWithinKnowledgeBase(
      sourceId,
      browserMatch.node,
      targetKbId,
      targetParentId,
      options?.titleSuffix,
    );
  }

  if (targetKbId !== sourceKbId) {
    return transferDocumentAcrossKnowledgeBases(
      sourceId,
      sourceKbId,
      targetKbId,
      targetParentId,
      'copy',
      {
        titleSuffix: options?.titleSuffix ?? ' (Copy)',
        sourceNode: browserMatch?.node ?? null,
        ingestTextDocument: ingestTextDocumentAcrossKnowledgeBases,
      },
    );
  }

  if (
    browserMatch
    && resolveDriveNodeId(browserMatch.node)
    && isKnowledgebaseDriveApiAvailable()
  ) {
    return copyDriveFileWithinKnowledgeBase(
      sourceId,
      browserMatch.node,
      targetKbId,
      targetParentId,
      options?.titleSuffix,
    );
  }

  const content = await getDocumentContent(sourceId);
  const titleSuffix = options?.titleSuffix ?? ' (Copy)';
  let sourceTitle = browserMatch?.node.name ?? 'Untitled';
  let sourceType: DocumentMeta['type'] = browserMatch ? mapNodeType(browserMatch.node) : 'markdown';

  const documentIdForSdk = parseSafeNumericDocumentId(sourceId);
  if (documentIdForSdk) {
    try {
      const document = await requireSdkClient().knowledge.documents.retrieve(documentIdForSdk);
      sourceTitle = document.title;
    } catch {
      // Keep browser-derived title when available.
    }
  }

  const created = await createDocument({
    kbId: targetKbId,
    title: `${sourceTitle}${titleSuffix}`,
    type: sourceType,
    content,
    parentId: targetParentId,
  });

  return created;
}

export async function deleteDocument(id: string): Promise<boolean> {
  const persistBrowserState = shouldPersistBrowserWorkspaceState();
  const tenantId = persistBrowserState ? requireTenantId() : null;

  if (parseOkfDocumentId(id)) {
    const okfRef = parseOkfDocumentId(id)!;
    assertEphemeralWorkspaceSpaceId(String(okfRef.spaceId));
    const client = requireSdkClient();
    await client.knowledge.okf.concepts.delete(String(okfRef.conceptRowId));
    // Local cleanup happens only after the remote delete confirmed: dropping
    // recents/cache first made a failed delete leave the document invisible
    // in the UI while it still exists server-side.
    if (tenantId !== null) {
      removeLocalDocumentContent(tenantId, id);
      removeRecentDocument(tenantId, id);
    }
    invalidateKnowledgeBrowserNodeCacheForSpaceIds(okfRef.spaceId);
    return true;
  }

  const browserMatch = await resolveBrowserNodeByDocumentId(id);
  const numericDocumentId = await resolveNumericDocumentId(id);
  let deleted = false;
  const failures: string[] = [];

  if (browserMatch && isKnowledgebaseDriveApiAvailable()) {
    const isFolder =
      browserMatch.node.nodeType === 'folder'
      || browserMatch.node.nodeType === 'virtual_folder';
    const canDeleteDriveNode =
      isFolder
      || browserMatch.node.driveNodeId
      || !numericDocumentId;
    if (canDeleteDriveNode) {
      try {
        await deleteDriveBrowserNode(browserMatch.node);
        deleted = true;
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        failures.push(`drive node: ${detail}`);
      }
    }
  }

  if (numericDocumentId) {
    const client = requireSdkClient();
    try {
      await client.knowledge.documents.delete(String(numericDocumentId));
      deleted = true;
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      failures.push(`document record: ${detail}`);
    }
  } else if (failures.length > 0) {
    const detail = failures.join('; ');
    throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED, { cause: detail });
  }

  if (!deleted) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.UNSUPPORTED_OPERATION);
  }

  if (tenantId !== null) {
    removeLocalDocumentContent(tenantId, id);
    removeRecentDocument(tenantId, id);
  }
  invalidateKnowledgeBrowserNodeCacheForSpaceIds(browserMatch?.spaceId);

  if (failures.length > 0) {
    // The primary record is gone but part of the deletion failed (for example
    // the drive file node remains). Surface it instead of reporting a clean
    // success that leaves orphaned state behind.
    throwKnowledgebaseError(KnowledgebaseErrorCodes.OPERATION_FAILED, {
      cause: `document deleted with partial failures: ${failures.join('; ')}`,
    });
  }
  return true;
}
