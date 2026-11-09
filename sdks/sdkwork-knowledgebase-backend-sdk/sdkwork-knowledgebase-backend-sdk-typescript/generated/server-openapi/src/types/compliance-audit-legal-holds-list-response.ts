import type { KnowledgeAuditLegalHoldItem } from './knowledge-audit-legal-hold-item';
import type { PageInfo } from './page-info';

export interface ComplianceAuditLegalHoldsListResponse {
  code: 0;
  data: unknown & { items: KnowledgeAuditLegalHoldItem[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
