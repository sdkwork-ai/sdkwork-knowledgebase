export interface CreateKnowledgeAuditLegalHoldRequest {
  subjectType: 'actor' | 'resource';
  subjectId: string;
  reason: string;
  caseReference?: string | null;
}
