export interface KnowledgeAuditLegalHoldItem {
  holdId: string;
  uuid: string;
  subjectType: 'actor' | 'resource';
  subjectId: string;
  reason: string;
  caseReference?: string | null;
  status: 'active' | 'released';
  createdBy: string;
  releasedBy?: string | null;
  releasedAt?: string | null;
  createdAt: string;
}
