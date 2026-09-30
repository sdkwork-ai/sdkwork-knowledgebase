import type { SdkWorkCommandData } from './sdk-work-command-data';

export interface SpacesMembersCreateResponse201 {
  code: 0;
  data: unknown & SdkWorkCommandData;
  /** Server-owned request correlation id. */
  traceId: string;
}
