import { createKnowledgebaseAppClient, type SdkworkKnowledgebaseAppClient } from '@sdkwork/knowledgebase-app-sdk';
import type { KnowledgebaseRuntimeConfig } from '../config/runtimeConfig';
import type { KnowledgebaseSdkTokenManager } from './sdkTokenManager';

const APP_API_PREFIX = '/app/v3/api';
const KNOWLEDGEBASE_APP_SDK_FAMILY_ID = 'sdkwork-knowledgebase-app-sdk';

export interface KnowledgebaseAppSdkClient {
  client: SdkworkKnowledgebaseAppClient;
  setTokenManager(manager: KnowledgebaseSdkTokenManager): void;
}

export interface KnowledgebaseAppSdkClientOptions {
  config: KnowledgebaseRuntimeConfig;
  sdkClient?: SdkworkKnowledgebaseAppClient;
  tokenManager: KnowledgebaseSdkTokenManager;
}

function normalizeGeneratedSdkBaseUrl(baseUrl: string, apiPrefix: string): string {
  const normalizedBaseUrl = baseUrl.replace(/\/+$/, '');
  const normalizedApiPrefix = apiPrefix.replace(/\/+$/, '');
  if (normalizedBaseUrl.endsWith(normalizedApiPrefix)) {
    return normalizedBaseUrl.slice(0, -normalizedApiPrefix.length) || normalizedBaseUrl;
  }
  return normalizedBaseUrl;
}

function resolveKnowledgebaseAppApiBaseUrl(config: KnowledgebaseRuntimeConfig): string {
  return config.sdkBaseUrls.dependencySdkBaseUrls[KNOWLEDGEBASE_APP_SDK_FAMILY_ID]?.appApiBaseUrl
    ?? config.appApiBaseUrl;
}

export function createKnowledgebaseAppSdkClient({
  config,
  sdkClient,
  tokenManager,
}: KnowledgebaseAppSdkClientOptions): KnowledgebaseAppSdkClient {
  const generatedClient = sdkClient ?? createKnowledgebaseAppClient({
    authMode: 'dual-token',
    baseUrl: normalizeGeneratedSdkBaseUrl(
      resolveKnowledgebaseAppApiBaseUrl(config),
      APP_API_PREFIX,
    ),
    tokenManager: tokenManager as never,
  });
  generatedClient.setTokenManager(tokenManager as never);

  return {
    client: generatedClient,
    setTokenManager(manager) {
      generatedClient.setTokenManager(manager as never);
    },
  };
}
