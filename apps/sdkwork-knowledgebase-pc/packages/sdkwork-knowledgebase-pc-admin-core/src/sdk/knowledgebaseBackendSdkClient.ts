import { createClient, type SdkworkKnowledgebaseBackendClient } from '@sdkwork/knowledgebase-backend-sdk';
import type { KnowledgebaseRuntimeConfig, KnowledgebaseSdkTokenManager } from 'sdkwork-knowledgebase-pc-core';

const BACKEND_API_PREFIX = '/backend/v3/api';
const KNOWLEDGEBASE_BACKEND_SDK_FAMILY_ID = 'sdkwork-knowledgebase-backend-sdk';

export interface KnowledgebaseBackendSdkClient {
  client: SdkworkKnowledgebaseBackendClient;
  setTokenManager(manager: KnowledgebaseSdkTokenManager): void;
}

export interface KnowledgebaseBackendSdkClientOptions {
  config: KnowledgebaseRuntimeConfig;
  sdkClient?: SdkworkKnowledgebaseBackendClient;
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

function resolveKnowledgebaseBackendApiBaseUrl(config: KnowledgebaseRuntimeConfig): string {
  return config.backendApiBaseUrl
    ?? config.sdkBaseUrls.dependencySdkBaseUrls[KNOWLEDGEBASE_BACKEND_SDK_FAMILY_ID]?.appApiBaseUrl
    ?? config.appApiBaseUrl;
}

export function createKnowledgebaseBackendSdkClient({
  config,
  sdkClient,
  tokenManager,
}: KnowledgebaseBackendSdkClientOptions): KnowledgebaseBackendSdkClient {
  const generatedClient = sdkClient ?? createClient({
    authMode: 'dual-token',
    baseUrl: normalizeGeneratedSdkBaseUrl(
      resolveKnowledgebaseBackendApiBaseUrl(config),
      BACKEND_API_PREFIX,
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
