/**
 * View models, page models, and route params for the knowledge capability.
 *
 * The generated ArkTS app SDK target is not emitted yet, so these stay view
 * models mapped structurally from the app-api envelope. They are never
 * transport DTOs.
 */
export interface KnowledgebaseListItem {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly documentCount: number;
}

export interface KnowledgebaseDocument {
  readonly id: string;
  readonly knowledgebaseId: string;
  readonly title: string;
  readonly preview: string;
}

export interface KnowledgebaseSearchHit {
  readonly documentId: string;
  readonly knowledgebaseId: string;
  readonly title: string;
  readonly snippet: string;
  readonly score: number;
}

export interface KnowledgebaseRouteParams {
  readonly documentId?: string;
  readonly ticket?: string;
}
