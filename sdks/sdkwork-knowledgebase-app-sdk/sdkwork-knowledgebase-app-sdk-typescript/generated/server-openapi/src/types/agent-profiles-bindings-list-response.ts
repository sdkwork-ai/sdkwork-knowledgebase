import type { KnowledgeAgentBindingList } from './knowledge-agent-binding-list';

export interface AgentProfilesBindingsListResponse {
  code: 0;
  data: unknown & { item: KnowledgeAgentBindingList; };
  /** Server-owned request correlation id. */
  traceId: string;
}
