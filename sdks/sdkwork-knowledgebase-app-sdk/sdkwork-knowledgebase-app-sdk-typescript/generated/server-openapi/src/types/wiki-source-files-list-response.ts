import type { KnowledgeWikiSourceFile } from './knowledge-wiki-source-file';
import type { PageInfo } from './page-info';

export interface WikiSourceFilesListResponse {
  code: 0;
  data: unknown & { items: KnowledgeWikiSourceFile[]; pageInfo: PageInfo; };
  /** Server-owned request correlation id. */
  traceId: string;
}
