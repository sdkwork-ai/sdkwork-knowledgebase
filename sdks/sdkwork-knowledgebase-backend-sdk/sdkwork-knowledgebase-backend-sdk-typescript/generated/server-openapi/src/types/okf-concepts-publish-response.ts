import type { SdkWorkCommandData } from './sdk-work-command-data';

export interface OkfConceptsPublishResponse {
  code: 0;
  data: unknown & SdkWorkCommandData;
  /** Server-owned request correlation id. */
  traceId: string;
}
