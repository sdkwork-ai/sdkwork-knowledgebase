import {
  isDevSameOriginApiEnabled,
  KnowledgebaseErrorCodes,
  throwKnowledgebaseError,
  type KnowledgebaseRuntimeConfig,
} from 'sdkwork-knowledgebase-pc-core';
import type { KnowledgebaseBackendSdkClient } from '../sdk/knowledgebaseBackendSdkClient';

let backendSdkClient: KnowledgebaseBackendSdkClient | null = null;
let backendApiEnabled = false;

export function configureKnowledgebaseBackendSdk(client: KnowledgebaseBackendSdkClient): void {
  backendSdkClient = client;
}

export function getKnowledgebaseBackendSdkClient(): KnowledgebaseBackendSdkClient {
  if (!backendSdkClient) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_SDK);
  }
  return backendSdkClient;
}

export function setKnowledgebaseBackendApiEnabled(enabled: boolean): void {
  backendApiEnabled = enabled;
}

export function isKnowledgebaseBackendApiAvailable(): boolean {
  return backendApiEnabled && backendSdkClient !== null;
}

export function isKnowledgebaseBackendApiConfigured(
  config: KnowledgebaseRuntimeConfig,
  env: Record<string, string | undefined> = import.meta.env as Record<string, string | undefined>,
): boolean {
  return Boolean(config.backendApiBaseUrl || config.appApiBaseUrl || config.sdkBaseUrls.appApiBaseUrl)
    || isDevSameOriginApiEnabled(config, env);
}

export function requireKnowledgebaseBackendApi(operation: string): void {
  if (!isKnowledgebaseBackendApiAvailable()) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE, {
      cause: operation,
    });
  }
}

// The backend-api policy accepts the exact platform permission only. Wildcard or
// domain-level grants must not open the admin console, because the server rejects them
// and the UI would render a console whose every call 403s.
const KNOWLEDGE_PLATFORM_MANAGE_SCOPES = new Set([
  'knowledge.platform.manage',
]);

export function canAccessKnowledgebaseAdminConsole(permissionScope?: string[]): boolean {
  if (!permissionScope?.length) {
    return false;
  }
  return permissionScope.some((scope) => KNOWLEDGE_PLATFORM_MANAGE_SCOPES.has(scope));
}
