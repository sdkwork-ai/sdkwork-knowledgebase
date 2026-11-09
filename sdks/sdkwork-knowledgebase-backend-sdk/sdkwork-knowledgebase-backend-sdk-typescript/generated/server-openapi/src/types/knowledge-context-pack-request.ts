import type { KnowledgeRetrievalBinding } from './knowledge-retrieval-binding';

export interface KnowledgeContextPackRequest {
  query: string;
  retrievalProfileId?: string | null;
  bindings: KnowledgeRetrievalBinding[];
  contextBudgetTokens: number;
  includeCitations: boolean;
  memoryPolicyRef?: string | null;
}
