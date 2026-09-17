import { normalizeListPageSize, toOffsetPageInfo, type OffsetPageInfo } from "@sdkwork/knowledgebase-h5-core/sdk";

import type { KnowledgeSpaceKind, KnowledgeSpaceSummary } from "../models/knowledgeModels";

export interface KnowledgeSpaceListRequest {
  kind?: KnowledgeSpaceKind;
  offset?: number;
  pageSize?: number;
  query?: string;
}

export interface KnowledgeSpaceListResult {
  items: KnowledgeSpaceSummary[];
  pageInfo: OffsetPageInfo;
}

/** SDK port injected from core/bootstrap; the feature package never builds transport. */
export interface KnowledgeSpaceCatalogPort {
  listKnowledgeSpaces(request: KnowledgeSpaceListRequest): Promise<KnowledgeSpaceListResult>;
  subscribeKnowledgeSpace(spaceId: string): Promise<void>;
  unsubscribeKnowledgeSpace(spaceId: string): Promise<void>;
}

export interface CreateKnowledgeCatalogServiceOptions {
  port: KnowledgeSpaceCatalogPort;
}

export interface KnowledgeCatalogService {
  list(request: KnowledgeSpaceListRequest): Promise<KnowledgeSpaceListResult>;
  subscribe(spaceId: string): Promise<void>;
  unsubscribe(spaceId: string): Promise<void>;
}

/** Knowledge space catalog service; mirrors the PC `knowledgeBrowserListService` contract. */
export function createKnowledgeCatalogService(
  options: CreateKnowledgeCatalogServiceOptions,
): KnowledgeCatalogService {
  const pageSizeOf = (requested?: number) => normalizeListPageSize(requested);

  return {
    async list(request) {
      const pageSize = pageSizeOf(request.pageSize);
      const result = await options.port.listKnowledgeSpaces({
        ...request,
        offset: request.offset ?? 0,
        pageSize,
      });
      return {
        items: result.items,
        pageInfo: result.pageInfo ?? toOffsetPageInfo({
          itemCount: result.items.length,
          offset: request.offset ?? 0,
          pageSize,
        }),
      };
    },
    subscribe: (spaceId) => options.port.subscribeKnowledgeSpace(spaceId),
    unsubscribe: (spaceId) => options.port.unsubscribeKnowledgeSpace(spaceId),
  };
}
