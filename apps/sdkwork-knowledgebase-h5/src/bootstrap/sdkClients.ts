import {
  createKnowledgebaseAppSdkClient,
  createKnowledgebaseDriveAppSdkClient,
  createKnowledgebaseSessionTokenManager,
} from "@sdkwork/knowledgebase-h5-core/sdk";

import { resolveKnowledgebaseH5Environment } from "./environment";
import { getKnowledgebaseH5Session } from "./iamRuntime";

/**
 * Construct the generated app-api SDK clients for one authenticated session.
 *
 * SDK construction belongs in bootstrap/core
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 8). Capability packages
 * receive these clients as injected ports through core public exports.
 */
export function bootstrapSdkClients() {
  const environment = resolveKnowledgebaseH5Environment();
  const session = getKnowledgebaseH5Session();
  const tokenManager = createKnowledgebaseSessionTokenManager(session);
  const knowledgebase = createKnowledgebaseAppSdkClient({
    baseUrl: environment.appApiBaseUrl,
    tokenManager,
  });
  const drive = createKnowledgebaseDriveAppSdkClient({
    baseUrl: environment.appApiBaseUrl,
    tokenManager,
  });

  return { drive, knowledgebase, tokenManager };
}
