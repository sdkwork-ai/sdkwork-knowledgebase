/** Cursor pagination helpers shared by the H5 knowledgebase services. */

export const DEFAULT_LIST_PAGE_SIZE = 20;
export const MAX_LIST_PAGE_SIZE = 100;

export interface OffsetPageInfo {
  hasMore: boolean;
  nextOffset: number;
  offset: number;
  pageSize: number;
  total?: number;
}

export function normalizeListPageSize(requested?: number): number {
  if (!requested || !Number.isFinite(requested) || requested <= 0) {
    return DEFAULT_LIST_PAGE_SIZE;
  }
  return Math.min(Math.trunc(requested), MAX_LIST_PAGE_SIZE);
}

export function toOffsetPageInfo(input: {
  itemCount: number;
  offset: number;
  pageSize: number;
  total?: number;
}): OffsetPageInfo {
  const consumed = input.offset + input.itemCount;
  const hasMore = typeof input.total === "number"
    ? consumed < input.total
    : input.itemCount >= input.pageSize;
  return {
    hasMore,
    nextOffset: hasMore ? consumed : consumed,
    offset: input.offset,
    pageSize: input.pageSize,
    ...(typeof input.total === "number" ? { total: input.total } : {}),
  };
}
