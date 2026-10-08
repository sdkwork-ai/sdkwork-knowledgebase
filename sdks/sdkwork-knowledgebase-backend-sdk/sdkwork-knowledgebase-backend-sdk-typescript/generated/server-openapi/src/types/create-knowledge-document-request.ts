export interface CreateKnowledgeDocumentRequest {
  spaceId: string;
  sourceId?: number | null;
  title: string;
  mimeType?: string | null;
  language?: string | null;
}
