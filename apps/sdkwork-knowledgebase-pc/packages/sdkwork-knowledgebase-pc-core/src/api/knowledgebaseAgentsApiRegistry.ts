import type { KnowledgebaseAgentsAppSdkClient } from '../sdk/agentsAppSdkClient';
import { KnowledgebaseErrorCodes } from '../errors/knowledgebaseErrorCodes';
import { throwKnowledgebaseError } from '../errors/knowledgebaseAppError';
import { isKnowledgebaseApiAvailable } from './knowledgebaseApiRegistry';

let agentsSdkClient: KnowledgebaseAgentsAppSdkClient | null = null;

export function configureKnowledgebaseAgentsAppSdk(
  client: KnowledgebaseAgentsAppSdkClient,
): void {
  agentsSdkClient = client;
}

export function getKnowledgebaseAgentsAppSdkClient(): KnowledgebaseAgentsAppSdkClient {
  if (!agentsSdkClient) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_AGENTS);
  }
  return agentsSdkClient;
}

export function isKnowledgebaseAgentsApiAvailable(): boolean {
  return isKnowledgebaseApiAvailable() && agentsSdkClient !== null;
}
