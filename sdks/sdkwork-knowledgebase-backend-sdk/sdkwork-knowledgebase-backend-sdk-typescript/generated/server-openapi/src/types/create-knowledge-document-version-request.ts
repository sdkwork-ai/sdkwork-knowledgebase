export interface CreateKnowledgeDocumentVersionRequest {
  documentId: string;
  originalObjectRefId: string;
  checksumSha256Hex?: string | null;
  sizeBytes: string;
  mimeType?: string | null;
}
