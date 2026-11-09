import {
  createClient as createGeneratedKnowledgebaseOpenClient,
  SdkworkKnowledgebaseOpenClient,
} from '../generated/server-openapi/src/index';
import type { SdkworkCustomConfig } from '../generated/server-openapi/src/types/common';

export { SdkworkKnowledgebaseOpenClient, createGeneratedKnowledgebaseOpenClient };
export type { SdkworkCustomConfig };

/** SDKWork v3 standard config alias for the open-api client. */
export type SdkworkConfig = SdkworkCustomConfig;

export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';

/**
 * Creates the Knowledgebase open-api client (API-key authenticated).
 *
 * @param config Client configuration; the API key is supplied through the injected
 *   credential provider or `setApiKey` rather than assembled by hand.
 */
export function createKnowledgebaseOpenClient(
  config: SdkworkCustomConfig,
): SdkworkKnowledgebaseOpenClient {
  return createGeneratedKnowledgebaseOpenClient(config);
}

/** Backwards-compatible factory alias. */
export function createClient(config: SdkworkCustomConfig): SdkworkKnowledgebaseOpenClient {
  return createKnowledgebaseOpenClient(config);
}
