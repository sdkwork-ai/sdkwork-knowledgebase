import type { KnowledgeFilter } from './knowledge-filter';

export interface KnowledgeAgentBindingRequest {
  profileId: string;
  spaceId: string;
  sourceFilter?: KnowledgeFilter[] | null;
  documentFilter?: KnowledgeFilter[] | null;
  priority: number;
  topK?: number | null;
  minScore?: number | null;
  enabled: boolean;
}
