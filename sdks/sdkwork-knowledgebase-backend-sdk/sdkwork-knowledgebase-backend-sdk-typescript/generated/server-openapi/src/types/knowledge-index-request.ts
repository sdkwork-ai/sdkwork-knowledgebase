export interface KnowledgeIndexRequest {
  spaceId: string;
  indexKind: string;
  embeddingProviderId?: string | null;
  embeddingModel?: string | null;
  dimension?: number | null;
  metric?: string | null;
}
