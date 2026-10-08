export interface KnowledgeRetrievalTrace {
  retrievalTraceId: string;
  status: string;
  latencyMs?: string | null;
  resultCount: number;
}
