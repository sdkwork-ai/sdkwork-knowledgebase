import {
  configureKnowledgebaseMpSdkBaseUrl,
  configureKnowledgebaseMpSdkBootstrapAccessToken,
  getKnowledgebaseMpAppSdkClient,
} from "@sdkwork/knowledgebase-mp-core/sdk";

import { bootstrap } from "./runtime";

export interface KnowledgebaseMiniProgramRuntimeOptions {
  accessToken?: string;
  appApiBaseUrl?: string;
}

export function bootstrapKnowledgebaseMiniProgram(
  options: KnowledgebaseMiniProgramRuntimeOptions = {},
) {
  return bootstrap(options);
}

export function getKnowledgebaseMpSdkClient() {
  return getKnowledgebaseMpAppSdkClient();
}

export { configureKnowledgebaseMpSdkBaseUrl, configureKnowledgebaseMpSdkBootstrapAccessToken };

// Capability surface projected into the runtime bundle. Platform pages consume
// these instead of duplicating mapping/pagination logic, and never construct
// SDK clients themselves.
export {
  createKnowledgebaseMpCatalogService,
  createKnowledgebaseMpSearchService,
  knowledgebaseMpRouteContributions,
  resolveKnowledgebaseMpCatalogScreenState,
  type KnowledgebaseMpCatalogPage,
  type KnowledgebaseMpCatalogService,
} from "@sdkwork/knowledgebase-mp-knowledge";
export { listKnowledgebaseMpRootPages, projectKnowledgebaseMpPages } from "@sdkwork/knowledgebase-mp-shell";
export { knowledgebaseMpTokens, resolveKnowledgebaseMpScreenStatus } from "@sdkwork/knowledgebase-mp-commons";
export { createKnowledgebaseMpWxHostAdapter } from "@sdkwork/knowledgebase-mp-host";
