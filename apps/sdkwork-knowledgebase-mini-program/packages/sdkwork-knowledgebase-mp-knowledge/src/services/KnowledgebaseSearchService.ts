import { normalizeListPageSize } from "@sdkwork/knowledgebase-mp-core/sdk";

import type { KnowledgebaseMpSearchHit } from "../models/knowledgeModels";

export interface KnowledgebaseMpSearchPort {
  searchKnowledge(request: {
    offset: number;
    pageSize: number;
    query: string;
    spaceId?: string;
  }): Promise<{ hasMore: boolean; hits: KnowledgebaseMpSearchHit[] }>;
}

export interface KnowledgebaseMpSearchService {
  search(request: { pageSize?: number; query: string; spaceId?: string }): Promise<{
    hasMore: boolean;
    hits: KnowledgebaseMpSearchHit[];
  }>;
}

export function createKnowledgebaseMpSearchService(options: {
  port: KnowledgebaseMpSearchPort;
}): KnowledgebaseMpSearchService {
  return {
    search(request) {
      const query = request.query.trim();
      if (!query) {
        return Promise.resolve({ hasMore: false, hits: [] });
      }
      return options.port.searchKnowledge({
        ...(request.spaceId ? { spaceId: request.spaceId } : {}),
        offset: 0,
        pageSize: normalizeListPageSize(request.pageSize),
        query,
      });
    },
  };
}
