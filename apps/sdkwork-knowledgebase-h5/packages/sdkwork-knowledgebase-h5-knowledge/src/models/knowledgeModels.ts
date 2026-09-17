/** Cross-client knowledge domain models (alignment spec section 7 route identity). */

export type KnowledgeSpaceKind = 'personal' | 'team' | 'shared' | 'subscribed';

export interface KnowledgeSpaceSummary {
  documentCount?: number;
  id: string;
  kind: KnowledgeSpaceKind;
  name: string;
  updatedAt?: string;
}

export interface KnowledgeDocumentSummary {
  id: string;
  kind: 'document' | 'folder' | 'link' | 'code' | 'note';
  parentId?: string;
  title: string;
  updatedAt?: string;
}

export interface KnowledgeSearchHit {
  documentId: string;
  score: number;
  snippet: string;
  spaceId: string;
  title: string;
}

export interface KnowledgeRouteParams {
  documentId?: string;
  spaceId?: string;
}

/** Canonical cross-platform route ids for this capability. */
export const KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS = [
  'app.intelligence.knowledgebase.list',
  'app.intelligence.knowledgebase.detail',
  'app.intelligence.knowledgebase.search',
  'app.intelligence.knowledgebase.settings',
  'app.intelligence.knowledgebase.launch',
] as const;

export type KnowledgebaseCapabilityRouteId = (typeof KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS)[number];
