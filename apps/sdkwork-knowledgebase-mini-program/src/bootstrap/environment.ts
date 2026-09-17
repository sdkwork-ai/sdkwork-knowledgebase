/** Runtime env projection types for the mini program bundle. */
export interface KnowledgebaseMiniProgramRuntimeEnv {
  SDKWORK_DEPLOYMENT_PROFILE?: string;
  SDKWORK_ENVIRONMENT?: string;
  SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL?: string;
  SDKWORK_KNOWLEDGEBASE_MP_PLATFORM?: string;
  SDKWORK_PROFILE_ID?: string;
  SDKWORK_RUNTIME_TARGET?: string;
}

export interface KnowledgebaseMiniProgramEnvironment {
  appApiBaseUrl: string;
  environment: string;
  deploymentProfile: string;
  platform: string;
  profileId: string;
}

/** Reject a duplicated `/app/v3/api` suffix on the projected app-api base URL. */
export function normalizeMiniProgramAppApiBaseUrl(value: string): string {
  const normalized = value.trim().replace(/\/+$/u, "");
  if (!normalized) {
    throw new Error("SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL is required");
  }
  return normalized;
}

export function createKnowledgebaseMiniProgramEnvironment(
  runtimeEnv: KnowledgebaseMiniProgramRuntimeEnv,
): KnowledgebaseMiniProgramEnvironment {
  const appApiBaseUrl = runtimeEnv.SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL;
  if (!appApiBaseUrl) {
    throw new Error("SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL is required");
  }
  return {
    appApiBaseUrl: normalizeMiniProgramAppApiBaseUrl(appApiBaseUrl),
    deploymentProfile: runtimeEnv.SDKWORK_DEPLOYMENT_PROFILE ?? "standalone",
    environment: runtimeEnv.SDKWORK_ENVIRONMENT ?? "development",
    platform: runtimeEnv.SDKWORK_KNOWLEDGEBASE_MP_PLATFORM ?? "MP_WEIXIN",
    profileId: runtimeEnv.SDKWORK_PROFILE_ID ?? "standalone.development",
  };
}
