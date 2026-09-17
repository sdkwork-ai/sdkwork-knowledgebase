import type { KnowledgeDocumentSummary } from "../models/knowledgeModels";

export interface KnowledgeDocumentTreeResult {
  documents: KnowledgeDocumentSummary[];
  spaceId: string;
}

export interface KnowledgeDocumentContent {
  body: string;
  documentId: string;
  title: string;
  version?: string;
}

/** SDK port injected from core/bootstrap. */
export interface KnowledgeDocumentPort {
  getDocumentContent(documentId: string): Promise<KnowledgeDocumentContent>;
  listDocumentTree(spaceId: string, parentId?: string): Promise<KnowledgeDocumentTreeResult>;
}

export interface KnowledgeDocumentService {
  loadContent(documentId: string): Promise<KnowledgeDocumentContent>;
  loadTree(spaceId: string, parentId?: string): Promise<KnowledgeDocumentTreeResult>;
}

/** Document tree and body service; mirrors the PC `document.ts` / `knowledgebaseDocumentApiBridge` contract. */
export function createKnowledgeDocumentService(options: {
  port: KnowledgeDocumentPort;
}): KnowledgeDocumentService {
  return {
    loadContent: (documentId) => options.port.getDocumentContent(documentId),
    loadTree: (spaceId, parentId) => options.port.listDocumentTree(spaceId, parentId),
  };
}
