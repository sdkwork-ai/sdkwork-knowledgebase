import type { KnowledgeSpaceStatus } from './knowledge-space-status';

export interface KnowledgeSpace {
  id: string;
  uuid: string;
  name: string;
  description?: string | null;
  driveSpaceId?: string | null;
  status: KnowledgeSpaceStatus;
  okfBundleInitialized: boolean;
}
