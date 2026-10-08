import type { KnowledgeSourceType } from './knowledge-source-type';

export interface CreateKnowledgeSourceRequest {
  spaceId: string;
  sourceType: KnowledgeSourceType;
  provider?: string | null;
  driveBucket?: string | null;
  drivePrefix?: string | null;
  /** JSON connector config for external knowledge engines (for example Dify datasetId). */
  connectorMetadataJson?: string | null;
}
