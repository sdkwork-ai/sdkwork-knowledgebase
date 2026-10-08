import {
  KnowledgebaseErrorCodes,
  requireKnowledgebaseAppSdkHttpClient,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import type { MarketKnowledgeBase } from './document';

// Listing ids are int64 strings on the wire: routing them through `Number`
// silently rounds ids past 2^53 and the corrupted id is then replayed to the
// subscriptions API, so the canonical decimal text is validated instead.
function parseListingId(id: string): string {
  const trimmed = id.trim();
  if (!/^[0-9]+$/.test(trimmed) || /^0+$/.test(trimmed)) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.INVALID_MARKET_LISTING, {
      cause: `market listing id must be a canonical positive integer, received: ${id}`,
    });
  }
  return trimmed;
}

function mapCatalogItem(item: {
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
  isSubscribed: boolean;
}): MarketKnowledgeBase {
  return {
    id: item.id,
    title: item.title,
    icon: item.icon,
    description: item.description,
    author: item.author,
    tags: item.tags,
    subscribersCount: item.subscribersCount,
    documentsCount: item.documentsCount,
    provider: item.provider,
    modelName: item.modelName,
    isSubscribed: item.isSubscribed,
  };
}

import { normalizeSdkWorkListPage } from './sdkWorkListPage';

export async function listMarketKnowledgeBasesPage(
  cursor?: string | null,
  pageSize = 20,
): Promise<ReturnType<typeof normalizeSdkWorkListPage<MarketKnowledgeBase>>> {
  const client = requireKnowledgebaseAppSdkHttpClient();
  const page = normalizeSdkWorkListPage(
    await client.knowledge.market.listings.list({ cursor: cursor ?? undefined, pageSize }),
  );
  return {
    items: page.items.map((item) => mapCatalogItem(item as Parameters<typeof mapCatalogItem>[0])),
    nextCursor: page.nextCursor,
    hasMore: page.hasMore,
  };
}

export async function listMarketKnowledgeBases(): Promise<MarketKnowledgeBase[]> {
  const firstPage = await listMarketKnowledgeBasesPage();
  return firstPage.items;
}

export async function subscribeMarketListing(id: string): Promise<boolean> {
  const client = requireKnowledgebaseAppSdkHttpClient();
  const result = await client.knowledge.market.subscriptions.create({
    listingId: parseListingId(id),
  });
  return result.accepted === true;
}

export async function unsubscribeMarketListing(id: string): Promise<boolean> {
  const client = requireKnowledgebaseAppSdkHttpClient();
  await client.knowledge.market.subscriptions.delete(parseListingId(id));
  return true;
}
