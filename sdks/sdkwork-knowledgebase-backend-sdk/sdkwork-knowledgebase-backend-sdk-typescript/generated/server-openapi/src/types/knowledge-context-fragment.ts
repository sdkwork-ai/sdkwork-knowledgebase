import type { KnowledgeCitation } from './knowledge-citation';
import type { KnowledgeRetrievalMethod } from './knowledge-retrieval-method';

export interface KnowledgeContextFragment {
  chunkId: string;
  documentId: string;
  documentVersionId?: string | null;
  spaceId: string;
  title: string;
  content: string;
  score?: number | null;
  rank: number;
  tokenCount?: number | null;
  retrievalMethod: KnowledgeRetrievalMethod;
  citation?: KnowledgeCitation | null;
}
