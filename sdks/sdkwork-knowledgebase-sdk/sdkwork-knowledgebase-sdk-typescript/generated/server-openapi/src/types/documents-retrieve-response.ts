import type { KnowledgeDocument } from './knowledge-document';

export interface DocumentsRetrieveResponse {
  code: 0;
  data: unknown & { item: KnowledgeDocument; };
  /** Server-owned request correlation id. */
  traceId: string;
}
