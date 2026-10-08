import type { KnowledgeRetrievalResult } from './knowledge-retrieval-result';

export interface RetrievalsRetrieveResponse {
  code: 0;
  data: unknown & { item: KnowledgeRetrievalResult; };
  /** Server-owned request correlation id. */
  traceId: string;
}
