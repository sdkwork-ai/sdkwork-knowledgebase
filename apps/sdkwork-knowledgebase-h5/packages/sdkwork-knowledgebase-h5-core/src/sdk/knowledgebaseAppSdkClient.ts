import { createClient, type SdkworkKnowledgebaseAppClient } from "@sdkwork/knowledgebase-app-sdk";
import { readRuntimeEnv, resolveBaseUrl, type Interceptors, type SdkworkAppConfig } from "@sdkwork/sdk-common";

import {
  createKnowledgebaseRequestContextInterceptors,
  readKnowledgebaseSessionTokens,
  resolveKnowledgebaseAccessToken,
  resolveKnowledgebaseAuthToken,
} from "../session/session";

export type { SdkworkKnowledgebaseAppClient };

export interface KnowledgebaseAppSdkClientOptions {
  baseUrl: string;
  tokenManager?: SdkworkAppConfig["tokenManager"];
}

export const KNOWLEDGEBASE_APP_API_PREFIX = "/app/v3/api";

/**
 * Resolve the shared API base origin through `@sdkwork/sdk-common`.
 *
 * `SDKWORK_API_BASE_URL` replaces the per-app `VITE_SDKWORK_*_API_BASE_URL`
 * keys. The generated client appends its own `/app/v3/api` prefix, so the
 * resolved value is a bare origin.
 */
export function resolveKnowledgebaseAppSdkBaseUrl(): string {
  return resolveBaseUrl({ envKey: "SDKWORK_API_BASE_URL" }).url;
}

export function isKnowledgebaseAppSdkConfigured(): boolean {
  return resolveBaseUrl({ envKey: "SDKWORK_API_BASE_URL" }).reason !== "empty";
}

/** Strip a duplicated `/app/v3/api` suffix before handing the origin to the generated client. */
export function normalizeGeneratedSdkBaseUrl(baseUrl: string): string {
  const normalized = baseUrl.trim().replace(/\/+$/u, "");
  return normalized.endsWith(KNOWLEDGEBASE_APP_API_PREFIX)
    ? normalized.slice(0, -KNOWLEDGEBASE_APP_API_PREFIX.length)
    : normalized;
}

export function createKnowledgebaseAppSdkClient(
  options: KnowledgebaseAppSdkClientOptions,
): SdkworkKnowledgebaseAppClient {
  const session = readKnowledgebaseSessionTokens();
  const interceptors: Interceptors = createKnowledgebaseRequestContextInterceptors(
    () => readKnowledgebaseSessionTokens() ?? session,
  );
  return createClient({
    accessToken: resolveKnowledgebaseAccessToken(session) ?? readRuntimeEnv("SDKWORK_ACCESS_TOKEN"),
    authToken: resolveKnowledgebaseAuthToken(session),
    baseUrl: normalizeGeneratedSdkBaseUrl(options.baseUrl || resolveKnowledgebaseAppSdkBaseUrl()),
    interceptors,
    platform: "h5",
    ...(options.tokenManager ? { tokenManager: options.tokenManager } : {}),
  });
}

export type { KnowledgeMarketCatalogItem } from "@sdkwork/knowledgebase-app-sdk";
