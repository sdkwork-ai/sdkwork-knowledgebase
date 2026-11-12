import type { SdkworkDependencySdkBaseUrls } from '../config/runtimeConfig.js';
import { sdkworkKnowledgebasePcSdkInventory } from './sdk-inventory.js';

export function listDependencySdkWorkspaces(): string[] {
  return [...sdkworkKnowledgebasePcSdkInventory];
}

export function buildDependencySdkBaseUrls(input: {
  appApiBaseUrl: string;
  backendApiBaseUrl: string;
  openApiBaseUrl: string;
  iamAppApiBaseUrl: string;
  driveAppApiBaseUrl: string;
  agentsAppApiBaseUrl: string;
}): Record<string, SdkworkDependencySdkBaseUrls> {
  const result: Record<string, SdkworkDependencySdkBaseUrls> = {};

  for (const workspace of sdkworkKnowledgebasePcSdkInventory) {
    if (workspace === 'sdkwork-iam-app-sdk') {
      result[workspace] = { appApiBaseUrl: input.iamAppApiBaseUrl };
      continue;
    }
    if (workspace === 'sdkwork-drive-app-sdk') {
      result[workspace] = { appApiBaseUrl: input.driveAppApiBaseUrl };
      continue;
    }
    if (workspace === 'sdkwork-agents-app-sdk') {
      result[workspace] = { appApiBaseUrl: input.agentsAppApiBaseUrl };
      continue;
    }
    if (workspace === 'sdkwork-knowledgebase-app-sdk') {
      result[workspace] = { appApiBaseUrl: input.appApiBaseUrl };
      continue;
    }
    result[workspace] = { openApiBaseUrl: input.openApiBaseUrl };
  }

  return result;
}
