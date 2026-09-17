import { createClient, type SdkworkDriveAppClient } from "@sdkwork/drive-app-sdk";

import { resolveKnowledgebaseMpTransportBaseUrl } from "./knowledgebaseMpAppSdkClient";

export type { SdkworkDriveAppClient };

/** Drive-backed knowledge asset storage; consumes the generated Drive app SDK. */
export function createKnowledgebaseMpDriveAppSdkClient(): SdkworkDriveAppClient {
  return createClient({
    baseUrl: resolveKnowledgebaseMpTransportBaseUrl(),
    platform: "mini-program",
  });
}
