import type { KnowledgeDocumentVisibility } from './knowledge-document-visibility';

export interface CreateKnowledgeDocumentRequest {
  spaceId: string;
  sourceId?: string | null;
  title: string;
  mimeType?: string | null;
  language?: string | null;
  visibility?: KnowledgeDocumentVisibility;
}
