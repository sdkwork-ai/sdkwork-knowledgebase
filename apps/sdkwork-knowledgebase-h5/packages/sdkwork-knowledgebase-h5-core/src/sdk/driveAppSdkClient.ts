import { createClient, type SdkworkDriveAppClient } from "@sdkwork/drive-app-sdk";

import { normalizeGeneratedSdkBaseUrl, resolveKnowledgebaseAppSdkBaseUrl } from "./knowledgebaseAppSdkClient";

export type { SdkworkDriveAppClient };

export interface KnowledgebaseDriveAppSdkClientOptions {
  baseUrl: string;
  tokenManager?: Parameters<typeof createClient>[0]["tokenManager"];
}

/**
 * Knowledgebase consumes Drive through the generated Drive app SDK because
 * cloud-drive import and knowledge asset storage are Drive-owned contracts.
 */
export function createKnowledgebaseDriveAppSdkClient(
  options: KnowledgebaseDriveAppSdkClientOptions,
): SdkworkDriveAppClient {
  return createClient({
    baseUrl: normalizeGeneratedSdkBaseUrl(options.baseUrl || resolveKnowledgebaseAppSdkBaseUrl()),
    platform: "h5",
    ...(options.tokenManager ? { tokenManager: options.tokenManager } : {}),
  });
}
