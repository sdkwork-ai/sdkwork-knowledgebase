import type { KnowledgeRetrievalResult } from './knowledge-retrieval-result';

export interface RetrievalsCreateResponse201 {
  code: 0;
  data: unknown & { item: KnowledgeRetrievalResult; };
  /** Server-owned request correlation id. */
  traceId: string;
}
