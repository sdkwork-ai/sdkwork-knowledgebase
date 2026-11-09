import type { KnowledgeFilter } from './knowledge-filter';
import type { KnowledgeRetrievalBinding } from './knowledge-retrieval-binding';
import type { KnowledgeRetrievalMethod } from './knowledge-retrieval-method';

export interface KnowledgeRetrievalRequest {
  query: string;
  retrievalProfileId?: string | null;
  bindings: KnowledgeRetrievalBinding[];
  methods?: KnowledgeRetrievalMethod[];
  topK?: number | null;
  includeCitations: boolean;
  includeTrace: boolean;
  contextBudgetTokens?: number | null;
  metadata?: KnowledgeFilter[];
}
