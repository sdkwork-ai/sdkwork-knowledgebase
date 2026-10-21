import type { KnowledgeAgentBinding } from './knowledge-agent-binding';
import type { PageInfo } from './page-info';

export interface AgentProfilesBindingsListResponse {
  code: 0;
  data: unknown & { items: KnowledgeAgentBinding[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
