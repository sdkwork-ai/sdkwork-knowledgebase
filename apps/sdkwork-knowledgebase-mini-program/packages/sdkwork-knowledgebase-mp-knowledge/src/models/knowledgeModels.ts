/** Cross-client knowledge domain models (route identity stays PC-aligned). */

export type KnowledgeSpaceKind = "personal" | "shared" | "subscribed" | "team";

export interface KnowledgebaseMpSpace {
  documentCount?: number;
  id: string;
  kind: KnowledgeSpaceKind;
  name: string;
  updatedAt?: string;
}

export interface KnowledgebaseMpDocument {
  id: string;
  parentId?: string;
  title: string;
  updatedAt?: string;
}

export interface KnowledgebaseMpSearchHit {
  documentId: string;
  score: number;
  snippet: string;
  spaceId: string;
  title: string;
}

export const KNOWLEDGEBASE_MP_CAPABILITY_ROUTE_IDS = [
  "app.intelligence.knowledgebase.list",
  "app.intelligence.knowledgebase.detail",
  "app.intelligence.knowledgebase.search",
  "app.intelligence.knowledgebase.settings",
  "app.intelligence.knowledgebase.launch",
] as const;

export type KnowledgebaseMpCapabilityRouteId = (typeof KNOWLEDGEBASE_MP_CAPABILITY_ROUTE_IDS)[number];
