import type { KnowledgeFilter } from './knowledge-filter';

export interface KnowledgeAgentBinding {
  bindingId: string;
  profileId: string;
  tenantId: string;
  spaceId: string;
  sourceFilter?: KnowledgeFilter[] | null;
  documentFilter?: KnowledgeFilter[] | null;
  priority: number;
  topK?: number | null;
  minScore?: number | null;
  enabled: boolean;
}
