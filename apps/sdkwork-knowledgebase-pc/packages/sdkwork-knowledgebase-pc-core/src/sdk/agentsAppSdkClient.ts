import {
  createClient,
  type SdkworkAppClient,
} from '@sdkwork/agents-app-sdk';
import { resolveSharedSdkApiBaseUrl } from '../config/resolveSdkApiBaseUrl';
import type { KnowledgebaseRuntimeConfig } from '../config/runtimeConfig';
import type { KnowledgebaseSdkTokenManager } from './sdkTokenManager';

const APP_API_PREFIX = '/app/v3/api';
const AGENTS_APP_SDK_FAMILY_ID = 'sdkwork-agents-app-sdk';

export interface KnowledgebaseAgentsAppSdkClient {
  client: SdkworkAppClient;
  setTokenManager(manager: KnowledgebaseSdkTokenManager): void;
}

export interface KnowledgebaseAgentsAppSdkClientOptions {
  config: KnowledgebaseRuntimeConfig;
  sdkClient?: SdkworkAppClient;
  tokenManager: KnowledgebaseSdkTokenManager;
}

function resolveAgentsAppApiBaseUrl(config: KnowledgebaseRuntimeConfig): string {
  // The shared `SDKWORK_API_BASE_URL` key resolved through `@sdkwork/sdk-common`
  // wins; the per-app keys behind the runtime config only survive as a fallback.
  return resolveSharedSdkApiBaseUrl()
    ?? config.sdkBaseUrls.dependencySdkBaseUrls[AGENTS_APP_SDK_FAMILY_ID]?.appApiBaseUrl
    ?? config.platformApiGatewayBaseUrl;
}

export function createKnowledgebaseAgentsAppSdkClient({
  config,
  sdkClient,
  tokenManager,
}: KnowledgebaseAgentsAppSdkClientOptions): KnowledgebaseAgentsAppSdkClient {
  // The Agents SDK accepts a gateway origin or the canonical `/app/v3/api`
  // surface path. Dev same-origin mode resolves the base url to an empty
  // string, so fall back to the relative surface path instead of passing an
  // empty base url (the SDK rejects that).
  const generatedClient = sdkClient ?? createClient({
    authMode: 'dual-token',
    baseUrl: resolveAgentsAppApiBaseUrl(config) || APP_API_PREFIX,
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
