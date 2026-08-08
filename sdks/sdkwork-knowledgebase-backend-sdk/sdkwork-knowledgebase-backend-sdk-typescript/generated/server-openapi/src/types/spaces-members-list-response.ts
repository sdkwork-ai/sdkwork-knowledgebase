import type { KnowledgeSpaceMember } from './knowledge-space-member';
import type { PageInfo } from './page-info';

export interface SpacesMembersListResponse {
  code: 0;
  data: unknown & { items: KnowledgeSpaceMember[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
