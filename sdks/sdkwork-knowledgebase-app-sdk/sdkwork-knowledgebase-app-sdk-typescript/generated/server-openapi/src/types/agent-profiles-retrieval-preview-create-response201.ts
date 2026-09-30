import type { KnowledgeRetrievalResult } from './knowledge-retrieval-result';

export interface AgentProfilesRetrievalPreviewCreateResponse201 {
  code: 0;
  data: unknown & { item: KnowledgeRetrievalResult; };
  /** Server-owned request correlation id. */
  traceId: string;
}
