import type { IngestionJobState } from './ingestion-job-state';

export interface IngestionJob {
  id: string;
  spaceId: string;
  sourceType: string;
  idempotencyKey: string;
  state: IngestionJobState;
  errorMessage?: string | null;
}
