import { normalizeListPageSize } from "@sdkwork/knowledgebase-mp-core/sdk";

import type { KnowledgebaseMpSpace } from "../models/knowledgeModels";

export interface KnowledgebaseMpCatalogPage {
  hasMore: boolean;
  items: KnowledgebaseMpSpace[];
  nextOffset: number;
}

/** SDK port injected from core/bootstrap; the feature package never builds transport. */
export interface KnowledgebaseMpCatalogPort {
  listKnowledgeSpaces(request: {
    kind?: KnowledgebaseMpSpace["kind"];
    offset: number;
    pageSize: number;
  }): Promise<KnowledgebaseMpCatalogPage>;
  subscribeKnowledgeSpace(spaceId: string): Promise<void>;
}

export interface KnowledgebaseMpCatalogService {
  list(request?: {
    kind?: KnowledgebaseMpSpace["kind"];
    offset?: number;
    pageSize?: number;
  }): Promise<KnowledgebaseMpCatalogPage>;
  subscribe(spaceId: string): Promise<void>;
}

export function createKnowledgebaseMpCatalogService(options: {
  port: KnowledgebaseMpCatalogPort;
}): KnowledgebaseMpCatalogService {
  return {
    async list(request = {}) {
      const offset = request.offset ?? 0;
      const pageSize = normalizeListPageSize(request.pageSize);
      const page = await options.port.listKnowledgeSpaces({
        ...(request.kind ? { kind: request.kind } : {}),
        offset,
        pageSize,
      });
      return {
        ...page,
        nextOffset: page.hasMore ? offset + page.items.length : offset + page.items.length,
      };
    },
    subscribe: (spaceId) => options.port.subscribeKnowledgeSpace(spaceId),
  };
}

export function resolveKnowledgebaseMpCatalogScreenState(input: {
  error?: string;
  itemCount: number;
  loading: boolean;
}): "empty" | "error" | "loading" | "ready" {
  if (input.loading) {
    return "loading";
  }
  if (input.error) {
    return "error";
  }
  return input.itemCount > 0 ? "ready" : "empty";
}
