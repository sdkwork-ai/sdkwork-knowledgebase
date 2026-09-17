import { createClient, type SdkworkKnowledgebaseAppClient } from "@sdkwork/knowledgebase-app-sdk";
import type { SdkworkAppConfig } from "@sdkwork/sdk-common";

import {
  readKnowledgebaseMpSessionTokens,
  resolveKnowledgebaseMpAccessToken,
  resolveKnowledgebaseMpAuthToken,
  type KnowledgebaseMpSession,
} from "../session/session";

export type { SdkworkKnowledgebaseAppClient };
export type SdkworkKnowledgebaseMpAppClientConfig = SdkworkAppConfig;

export const KNOWLEDGEBASE_MP_APP_API_PREFIX = "/app/v3/api";

let knowledgebaseMpAppSdkClient: SdkworkKnowledgebaseAppClient | null = null;
let configuredBaseUrl: string | null = null;
let bootstrapAccessToken: string | null = null;

/**
 * Configure the app-api base URL for the mini program runtime.
 *
 * The mini program has no browser origin to derive a base URL from, so the
 * projected runtime env is the only source. A duplicated `/app/v3/api` suffix
 * is rejected instead of silently doubled.
 */
export function configureKnowledgebaseMpSdkBaseUrl(baseUrl: string): void {
  const normalized = baseUrl.trim().replace(/\/+$/u, "");
  if (normalized.length === 0) {
    throw new Error("SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL is required");
  }
  if (!normalized.endsWith(KNOWLEDGEBASE_MP_APP_API_PREFIX)) {
    throw new Error(`knowledgebase app-api base URL must end with ${KNOWLEDGEBASE_MP_APP_API_PREFIX}`);
  }
  configuredBaseUrl = normalized;
}

export function configureKnowledgebaseMpSdkBootstrapAccessToken(accessToken?: string): void {
  const normalized = accessToken?.trim();
  bootstrapAccessToken = normalized && normalized.length > 0 ? normalized : null;
}

export function resolveKnowledgebaseMpSdkBaseUrl(): string {
  if (configuredBaseUrl) {
    return configuredBaseUrl;
  }
  throw new Error("SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL must be configured before SDK bootstrap");
}

/** The generated client takes a bare origin, so the app-api suffix is stripped. */
export function resolveKnowledgebaseMpTransportBaseUrl(): string {
  return resolveKnowledgebaseMpSdkBaseUrl().slice(0, -KNOWLEDGEBASE_MP_APP_API_PREFIX.length);
}

export function createKnowledgebaseMpAppSdkClientConfig(
  session?: KnowledgebaseMpSession | null,
): SdkworkKnowledgebaseMpAppClientConfig {
  const currentSession = session ?? readKnowledgebaseMpSessionTokens();
  return {
    accessToken: resolveKnowledgebaseMpAccessToken(currentSession) ?? bootstrapAccessToken ?? undefined,
    authToken: resolveKnowledgebaseMpAuthToken(currentSession),
    baseUrl: resolveKnowledgebaseMpTransportBaseUrl(),
    platform: "mini-program",
  };
}

export function initKnowledgebaseMpAppSdkClient(
  config: SdkworkKnowledgebaseMpAppClientConfig = createKnowledgebaseMpAppSdkClientConfig(),
): SdkworkKnowledgebaseAppClient {
  knowledgebaseMpAppSdkClient = createClient(config);
  return knowledgebaseMpAppSdkClient;
}

export function getKnowledgebaseMpAppSdkClient(): SdkworkKnowledgebaseAppClient {
  return knowledgebaseMpAppSdkClient ?? initKnowledgebaseMpAppSdkClient();
}

export function resetKnowledgebaseMpAppSdkClient(): void {
  knowledgebaseMpAppSdkClient = null;
  bootstrapAccessToken = null;
}

export type { KnowledgeMarketCatalogItem } from "@sdkwork/knowledgebase-app-sdk";
