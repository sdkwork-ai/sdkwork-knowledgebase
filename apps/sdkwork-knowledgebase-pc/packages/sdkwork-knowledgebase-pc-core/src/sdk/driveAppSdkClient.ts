import {
  createClient,
  type SdkworkDriveAppClient,
} from '@sdkwork/drive-app-sdk';
import { resolveSharedSdkApiBaseUrl } from '../config/resolveSdkApiBaseUrl';
import type { KnowledgebaseRuntimeConfig } from '../config/runtimeConfig';
import type { KnowledgebaseSdkTokenManager } from './sdkTokenManager';

const APP_API_PREFIX = '/app/v3/api';
const DRIVE_APP_SDK_FAMILY_ID = 'sdkwork-drive-app-sdk';

export interface KnowledgebaseDriveAppSdkClient {
  client: SdkworkDriveAppClient;
  setTokenManager(manager: KnowledgebaseSdkTokenManager): void;
}

export interface KnowledgebaseDriveAppSdkClientOptions {
  config: KnowledgebaseRuntimeConfig;
  sdkClient?: SdkworkDriveAppClient;
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

function resolveDriveAppApiBaseUrl(config: KnowledgebaseRuntimeConfig): string {
  // The shared `SDKWORK_API_BASE_URL` key resolved through `@sdkwork/sdk-common`
  // wins; the per-app keys behind the runtime config only survive as a fallback.
  return resolveSharedSdkApiBaseUrl()
    ?? config.sdkBaseUrls.dependencySdkBaseUrls[DRIVE_APP_SDK_FAMILY_ID]?.appApiBaseUrl
    ?? config.platformApiGatewayBaseUrl;
}

export function createKnowledgebaseDriveAppSdkClient({
  config,
  sdkClient,
  tokenManager,
}: KnowledgebaseDriveAppSdkClientOptions): KnowledgebaseDriveAppSdkClient {
  const generatedClient = sdkClient ?? createClient({
    authMode: 'dual-token',
    baseUrl: normalizeGeneratedSdkBaseUrl(
      resolveDriveAppApiBaseUrl(config),
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
