import { normalizeListPageSize, type OffsetPageInfo, toOffsetPageInfo } from "@sdkwork/knowledgebase-h5-core/sdk";

import type { KnowledgeSearchHit } from "../models/knowledgeModels";

export interface KnowledgeSearchRequest {
  offset?: number;
  pageSize?: number;
  query: string;
  spaceId?: string;
}

export interface KnowledgeSearchResult {
  hits: KnowledgeSearchHit[];
  pageInfo: OffsetPageInfo;
}

/** SDK port injected from core/bootstrap. */
export interface KnowledgeSearchPort {
  searchKnowledge(request: KnowledgeSearchRequest): Promise<KnowledgeSearchResult>;
}

export interface KnowledgeSearchService {
  search(request: KnowledgeSearchRequest): Promise<KnowledgeSearchResult>;
}

/** Retrieval service; mirrors the PC search capability surface. */
export function createKnowledgeSearchService(options: {
  port: KnowledgeSearchPort;
}): KnowledgeSearchService {
  return {
    async search(request) {
      const pageSize = normalizeListPageSize(request.pageSize);
      const result = await options.port.searchKnowledge({
        ...request,
        offset: request.offset ?? 0,
        pageSize,
      });
      return {
        hits: result.hits,
        pageInfo: result.pageInfo ?? toOffsetPageInfo({
          itemCount: result.hits.length,
          offset: request.offset ?? 0,
          pageSize,
        }),
      };
    },
  };
}
