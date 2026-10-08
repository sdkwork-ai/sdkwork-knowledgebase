import type { KnowledgeDocumentState } from './knowledge-document-state';
import type { KnowledgeDocumentVersionState } from './knowledge-document-version-state';
import type { KnowledgeDocumentVisibility } from './knowledge-document-visibility';

export interface KnowledgeDocument {
  id: string;
  spaceId: string;
  sourceId?: number | null;
  originalFileDriveNodeId?: string | null;
  title: string;
  mimeType?: string | null;
  language?: string | null;
  currentVersionId?: number | null;
  visibility: KnowledgeDocumentVisibility;
  contentState: KnowledgeDocumentState;
  indexState: KnowledgeDocumentVersionState;
}
