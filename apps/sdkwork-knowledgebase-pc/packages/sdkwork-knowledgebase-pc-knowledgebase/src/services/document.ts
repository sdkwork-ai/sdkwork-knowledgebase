import { isBlank } from '@sdkwork/utils';
import {
  KnowledgebaseErrorCodes,
  requireKnowledgebaseApiAvailable,
  requireKnowledgebaseNetworkOnline,
  requireNonEmptyString,
} from 'sdkwork-knowledgebase-pc-core';
import * as KnowledgebaseDocumentApiBridge from './knowledgebaseDocumentApiBridge';
import * as KnowledgeGitImportService from './knowledgeGitImportService';
import * as KnowledgeGitSyncService from './knowledgeGitSyncService';
import * as KnowledgeMarketService from './knowledgeMarketService';
import * as KnowledgeFileUploadService from './knowledgeFileUploadService';
import { importWebLinkToKnowledgeBase } from './knowledgeWebLinkImportService';
import {
  listKnowledgeAssetLibraryItems,
  listKnowledgeAssetLibraryItemsPage,
  searchKnowledgeMediaDocuments,
  type KnowledgeAssetLibraryItem,
  type KnowledgeAssetLibraryPage,
} from './knowledgeAssetLibraryService';
import { KnowledgeSpaceMembersService, type KnowledgeSpaceMemberUi } from './knowledgeSpaceMembersService';
export type { KnowledgeSpaceMemberUi } from './knowledgeSpaceMembersService';

export interface DocumentMeta {
  id: string;
  title: string;
  type: 'richtext' | 'code' | 'markdown' | 'file' | 'image' | 'audio' | 'video' | 'folder' | 'pdf' | 'music';
  updatedAt: string;
  author: string;
  kbId?: string;
  size?: string;
  url?: string;
  driveUri?: string;
  driveSpaceId?: string;
  driveNodeId?: string;
  content?: string;
  parentId?: string | null;
  order?: number;
  isPinned?: boolean;
  tags?: string[];
}

export interface FolderNode {
  id: string;
  title: string;
  type: 'folder';
  children: (FolderNode | DocumentMeta)[];
  parentId?: string | null;
  updatedAt?: string;
  author?: string;
  isPinned?: boolean;
  tags?: string[];
}

export interface KnowledgeBase {
  id: string;
  title: string;
  icon?: string;
  avatar?: string;
  type?: 'team' | 'personal' | 'public';
  provider?: string;
  modelName?: string;
  temperature?: number;
  maxTokens?: number;
  systemPrompt?: string;
  publicPermission?: 'none' | 'read' | 'write' | 'admin';
  guestLinkEnabled?: boolean;
}

export interface MarketKnowledgeBase {
  id: string;
  title: string;
  icon: string;
  description: string;
  author: string;
  tags: string[];
  subscribersCount: number;
  documentsCount: number;
  provider: string;
  modelName: string;
  isSubscribed?: boolean;
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

async function withKnowledgebaseApi<T>(apiCall: () => Promise<T>): Promise<T> {
  requireKnowledgebaseApiAvailable();
  requireKnowledgebaseNetworkOnline();
  return apiCall();
}

/**
 * Frontend service facade over the generated Knowledgebase app SDK bridge.
 */
export class DocumentService {
  static async getKnowledgeBases(): Promise<{ team: KnowledgeBase[]; personal: KnowledgeBase[]; public: KnowledgeBase[] }> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.getKnowledgeBases());
  }

  static async createKnowledgeBase(kb: Partial<KnowledgeBase>): Promise<KnowledgeBase> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.createKnowledgeBase(kb));
  }

  static async hydrateKnowledgeBase(kb: KnowledgeBase): Promise<KnowledgeBase> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.hydrateKnowledgeBase(kb));
  }

  static async updateKnowledgeBase(id: string, updates: Partial<KnowledgeBase>): Promise<KnowledgeBase> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.updateKnowledgeBase(id, updates));
  }

  static async deleteKnowledgeBase(id: string): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.deleteKnowledgeBase(id));
  }

  static async getDocuments(kbId: string): Promise<(FolderNode | DocumentMeta)[]> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.getDocuments(kbId));
  }

  static async ensureFolderChildrenLoaded(
    kbId: string,
    folderId: string | null,
  ): Promise<void> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.ensureFolderChildrenLoaded(kbId, folderId));
  }

  static hasMoreFolderChildren(kbId: string, folderId: string | null): boolean {
    return KnowledgebaseDocumentApiBridge.hasMoreFolderChildren(kbId, folderId);
  }

  static async loadMoreFolderChildren(
    kbId: string,
    folderId: string | null,
  ): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.loadMoreFolderChildren(kbId, folderId));
  }

  static async getDocumentContent(id: string): Promise<string> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.getDocumentContent(id));
  }

  static async hydrateDocumentForViewer(doc: DocumentMeta): Promise<DocumentMeta> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.hydrateDocumentForViewer(doc));
  }

  static async saveDocumentContent(
    id: string,
    content: string,
    options?: import('./knowledgebaseDocumentApiBridge').SaveDocumentContentOptions,
  ): Promise<import('./knowledgebaseDocumentApiBridge').SaveDocumentContentResult> {
    return withKnowledgebaseApi(() =>
      KnowledgebaseDocumentApiBridge.saveDocumentContent(id, content, options),
    );
  }

  static async updateDocument(id: string, updates: Partial<DocumentMeta>): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.updateDocument(id, updates));
  }

  static async copyDocument(
    sourceId: string,
    targetKbId: string,
    targetParentId: string | null,
    options?: { titleSuffix?: string },
  ): Promise<DocumentMeta> {
    return withKnowledgebaseApi(() =>
      KnowledgebaseDocumentApiBridge.copyDocument(sourceId, targetKbId, targetParentId, options),
    );
  }

  static async createDocument(doc: Partial<DocumentMeta>): Promise<DocumentMeta> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.createDocument(doc));
  }

  static async uploadFiles(
    files: File[],
    kbId: string,
    parentId?: string,
    overrideType?: DocumentMeta['type'],
  ): Promise<DocumentMeta[]> {
    return withKnowledgebaseApi(() =>
      KnowledgeFileUploadService.uploadKnowledgebaseFiles(
        files,
        kbId,
        overrideType,
        parentId,
      ),
    );
  }

  static async importWebLink(params: {
    kbId: string;
    parentId?: string | null;
    url: string;
    title?: string;
  }): Promise<DocumentMeta> {
    return withKnowledgebaseApi(() => importWebLinkToKnowledgeBase(params));
  }

  static async deleteDocument(id: string): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.deleteDocument(id));
  }

  static async getMarketKnowledgeBases(): Promise<MarketKnowledgeBase[]> {
    return withKnowledgebaseApi(() => KnowledgeMarketService.listMarketKnowledgeBases());
  }

  static async listMarketKnowledgeBasesPage(
    cursor?: string | null,
  ): Promise<{ items: MarketKnowledgeBase[]; nextCursor: string | null; hasMore: boolean }> {
    return withKnowledgebaseApi(() => KnowledgeMarketService.listMarketKnowledgeBasesPage(cursor));
  }

  static async subscribeMarketKb(id: string): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgeMarketService.subscribeMarketListing(id));
  }

  static async unsubscribeMarketKb(id: string): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgeMarketService.unsubscribeMarketListing(id));
  }

  static async importGitRepository(
    kbId: string,
    repoUrl: string,
    branch: string = 'main',
    options?: {
      accessToken?: string;
      onProgress?: (progress: KnowledgeGitImportService.GitImportProgress) => void;
    },
  ): Promise<boolean> {
    return withKnowledgebaseApi(async () => {
      const result = await KnowledgeGitImportService.importGitRepository(
        kbId,
        repoUrl,
        branch,
        options?.accessToken,
        options?.onProgress,
      );
      return result.importedCount > 0;
    });
  }

  static async syncGitRepository(
    kbId: string,
    commitMessage: string,
    options?: {
      repoUrl: string;
      branch?: string;
      accessToken?: string;
      onProgress?: (progress: KnowledgeGitSyncService.GitSyncProgress) => void;
    },
  ): Promise<{ accepted: boolean; hash: string }> {
    if (!options || isBlank(options.repoUrl)) {
      requireNonEmptyString('', KnowledgebaseErrorCodes.REPO_URL_REQUIRED);
      throw new Error('Git repository URL is required');
    }

    const { repoUrl, branch, accessToken, onProgress } = options;
    return withKnowledgebaseApi(async () => {
      const result = await KnowledgeGitSyncService.syncGitRepository(
        kbId,
        repoUrl,
        branch ?? 'main',
        commitMessage,
        accessToken,
        onProgress,
      );
      return { accepted: result.accepted, hash: result.hash };
    });
  }

  static async searchAll(query: string): Promise<{
    kbs: KnowledgeBase[];
    docs: DocumentMeta[];
  }> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.searchAll(query));
  }

  static async getRecentDocuments(limit: number = 8): Promise<DocumentMeta[]> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.getRecentDocuments(limit));
  }

  static async touchDocument(id: string): Promise<boolean> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.touchDocument(id));
  }

  static async listDocumentVersions(documentId: string): Promise<DocumentVersionSummary[]> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.listDocumentVersions(documentId));
  }

  static async getDocumentAccess(documentId: string): Promise<DocumentAccessSummary> {
    return withKnowledgebaseApi(() => KnowledgebaseDocumentApiBridge.getDocumentAccess(documentId));
  }

  static async updateDocumentVisibility(
    documentId: string,
    visibility: KnowledgeDocumentVisibility,
  ): Promise<DocumentAccessSummary> {
    return withKnowledgebaseApi(() =>
      KnowledgebaseDocumentApiBridge.updateDocumentVisibility(documentId, visibility),
    );
  }

  static async loadKnowledgeSpaceMembers(spaceId: string | number): Promise<KnowledgeSpaceMemberUi[]> {
    return withKnowledgebaseApi(() => KnowledgeSpaceMembersService.loadMembers(String(spaceId)));
  }

  static async loadKnowledgeSpaceMembersPage(
    spaceId: string | number,
    cursor: string | null = null,
    pageSize = 20,
  ): Promise<import('./knowledgeSpaceMembersService').KnowledgeSpaceMembersPage> {
    return withKnowledgebaseApi(() =>
      KnowledgeSpaceMembersService.loadMembersPage(String(spaceId), cursor, pageSize),
    );
  }

  static async syncKnowledgeSpaceMembers(
    spaceId: string | number,
    desired: KnowledgeSpaceMemberUi[],
    previous: KnowledgeSpaceMemberUi[],
  ): Promise<void> {
    return withKnowledgebaseApi(() =>
      KnowledgeSpaceMembersService.syncMembers(String(spaceId), desired, previous),
    );
  }

  static async syncKnowledgeSpaceMembersPartial(
    spaceId: string | number,
    uiMembers: KnowledgeSpaceMemberUi[],
    baselineMembers: KnowledgeSpaceMemberUi[],
    loadedEmails: ReadonlySet<string>,
  ): Promise<void> {
    return withKnowledgebaseApi(() =>
      KnowledgeSpaceMembersService.syncMembersPartial(
        String(spaceId),
        uiMembers,
        baselineMembers,
        loadedEmails,
      ),
    );
  }

  static async listAssetLibraryItems(
    kbId: string,
    assetType: 'image' | 'audio' | 'video',
  ): Promise<{ items: KnowledgeAssetLibraryItem[]; truncated: boolean }> {
    return withKnowledgebaseApi(() => listKnowledgeAssetLibraryItems(kbId, assetType));
  }

  static async listAssetLibraryItemsPage(
    kbId: string,
    assetType: 'image' | 'audio' | 'video',
    cursor?: string | null,
    pageSize?: number,
  ): Promise<KnowledgeAssetLibraryPage> {
    return withKnowledgebaseApi(() =>
      listKnowledgeAssetLibraryItemsPage(kbId, assetType, cursor, pageSize),
    );
  }

  static async searchMediaDocuments(query: string, limit: number = 8): Promise<DocumentMeta[]> {
    return withKnowledgebaseApi(() => searchKnowledgeMediaDocuments(query, limit));
  }
}
