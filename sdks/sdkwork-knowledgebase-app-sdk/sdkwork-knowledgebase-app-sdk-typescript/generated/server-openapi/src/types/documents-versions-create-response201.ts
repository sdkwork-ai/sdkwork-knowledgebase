import type { KnowledgeDocumentVersion } from './knowledge-document-version';

export interface DocumentsVersionsCreateResponse201 {
  code: 0;
  data: unknown & { item: KnowledgeDocumentVersion; };
  /** Server-owned request correlation id. */
  traceId: string;
}
