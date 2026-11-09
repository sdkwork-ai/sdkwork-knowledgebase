import type { KnowledgeAuditLegalHoldItem } from './knowledge-audit-legal-hold-item';

export interface ComplianceAuditLegalHoldsCreateResponse201 {
  code: 0;
  data: unknown & { item: KnowledgeAuditLegalHoldItem; };
  /** Server-owned request correlation id. */
  traceId: string;
}
