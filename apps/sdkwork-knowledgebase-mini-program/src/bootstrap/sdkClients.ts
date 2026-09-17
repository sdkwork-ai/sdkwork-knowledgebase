import {
  configureKnowledgebaseMpSdkBaseUrl,
  configureKnowledgebaseMpSdkBootstrapAccessToken,
  initKnowledgebaseMpAppSdkClient,
} from "@sdkwork/knowledgebase-mp-core/sdk";

export interface KnowledgebaseMpSdkBootstrapOptions {
  accessToken?: string;
  appApiBaseUrl?: string;
}

/**
 * Construct the generated app-api SDK client for the mini program runtime.
 *
 * SDK construction belongs in bootstrap/core; capability packages receive the
 * client through core public exports.
 */
export function bootstrapSdkClients(options: KnowledgebaseMpSdkBootstrapOptions = {}) {
  if (options.appApiBaseUrl) {
    configureKnowledgebaseMpSdkBaseUrl(options.appApiBaseUrl);
  }
  configureKnowledgebaseMpSdkBootstrapAccessToken(options.accessToken);
  return { knowledgebaseAppSdk: initKnowledgebaseMpAppSdkClient() };
}
