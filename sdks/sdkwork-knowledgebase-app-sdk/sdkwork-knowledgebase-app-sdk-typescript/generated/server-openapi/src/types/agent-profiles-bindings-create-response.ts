import type { KnowledgeAgentBinding } from './knowledge-agent-binding';

export interface AgentProfilesBindingsCreateResponse {
  code: 0;
  data: unknown & { item: KnowledgeAgentBinding; };
  /** Server-owned request correlation id. */
  traceId: string;
}
