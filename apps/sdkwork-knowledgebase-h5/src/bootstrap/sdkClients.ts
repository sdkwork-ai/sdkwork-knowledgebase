import {
  createKnowledgebaseAppSdkClient,
  createKnowledgebaseDriveAppSdkClient,
  createKnowledgebaseSessionTokenManager,
} from "@sdkwork/knowledgebase-h5-core/sdk";

import { resolveKnowledgebaseH5Environment } from "./environment";
import { createKnowledgebaseH5IamRuntime } from "./iamRuntime";

/**
 * Construct the generated app-api SDK clients for one authenticated session.
 *
 * SDK construction belongs in bootstrap/core
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 8). Capability packages
 * receive these clients as injected ports through core public exports.
 */
export function bootstrapSdkClients() {
  const environment = resolveKnowledgebaseH5Environment();
  // The token manager adapts the live session STORE (read/update/subscribe),
  // not a point-in-time snapshot.
  const sessionStore = createKnowledgebaseH5IamRuntime();
  const tokenManager = createKnowledgebaseSessionTokenManager(sessionStore);
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
