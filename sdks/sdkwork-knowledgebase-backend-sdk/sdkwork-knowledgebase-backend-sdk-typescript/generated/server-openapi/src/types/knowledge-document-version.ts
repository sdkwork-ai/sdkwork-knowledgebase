import type { KnowledgeDocumentVersionState } from './knowledge-document-version-state';

export interface KnowledgeDocumentVersion {
  id: string;
  documentId: string;
  versionNo: string;
  originalObjectRefId: string;
  checksumSha256Hex?: string | null;
  sizeBytes: string;
  mimeType?: string | null;
  parseState: KnowledgeDocumentVersionState;
  indexState: KnowledgeDocumentVersionState;
}
