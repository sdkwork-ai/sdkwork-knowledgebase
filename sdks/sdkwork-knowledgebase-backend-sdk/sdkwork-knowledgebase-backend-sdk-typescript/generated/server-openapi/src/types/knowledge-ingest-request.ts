export interface KnowledgeIngestRequest {
  spaceId: string;
  title: string;
  payloadMarkdown?: string;
  sourceUrl?: string;
  idempotencyKey: string;
}
