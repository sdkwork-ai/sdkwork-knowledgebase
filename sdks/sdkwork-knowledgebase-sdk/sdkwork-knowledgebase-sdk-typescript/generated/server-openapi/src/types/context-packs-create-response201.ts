import type { KnowledgeContextPack } from './knowledge-context-pack';

export interface ContextPacksCreateResponse201 {
  code: 0;
  data: unknown & { item: KnowledgeContextPack; };
  /** Server-owned request correlation id. */
  traceId: string;
}
