import type { OkfProfileDocument } from './okf-profile-document';

export interface OkfBundleProfileRetrieveResponse {
  code: 0;
  data: unknown & { item: OkfProfileDocument; };
  /** Server-owned request correlation id. */
  traceId: string;
}
