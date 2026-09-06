import type { AuthTokenManager } from '@sdkwork/knowledgebase-app-sdk';
import type { SessionSnapshot } from 'sdkwork-knowledgebase-pc-core';
import { KnowledgebaseErrorCodes, throwKnowledgebaseError } from 'sdkwork-knowledgebase-pc-core';
import type { SdkworkKnowledgebaseAppClient, SdkworkDriveAppClient } from 'sdkwork-knowledgebase-pc-core';

import type { HostKnowledgeWindowRequest } from './hostKnowledgeWindowRequest';

export type { HostKnowledgeWindowRequest };

export interface KnowledgebasePcSdkPorts {
  getKnowledgebaseClient: () => SdkworkKnowledgebaseAppClient;
  getDriveClient: () => SdkworkDriveAppClient;
  /**
   * @returns the embedding host's shared TokenManager. Host-managed surfaces
   * bind it instead of minting a second instance (APP_SDK_INTEGRATION_SPEC
   * TokenManager closure rule); standalone runtimes omit this port.
   */
  getTokenManager?: () => AuthTokenManager;
  readHostSession: () => SessionSnapshot | null;
  subscribeHostSession?: (listener: () => void) => () => void;
  resolveHostLanguage?: () => string;
  subscribeHostLanguage?: (listener: (language: string) => void) => () => void;
  openHostKnowledgeWindow?: (request: HostKnowledgeWindowRequest) => Promise<boolean>;
}

let sdkPorts: KnowledgebasePcSdkPorts | null = null;

export function configureKnowledgebasePcSdkPorts(ports: KnowledgebasePcSdkPorts): void {
  sdkPorts = ports;
}

export function getKnowledgebasePcSdkPorts(): KnowledgebasePcSdkPorts {
  if (!sdkPorts) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.SDK_PORTS_MISSING);
  }
  return sdkPorts;
}

export function getConfiguredKnowledgebaseAppSdkClient(): SdkworkKnowledgebaseAppClient {
  return getKnowledgebasePcSdkPorts().getKnowledgebaseClient();
}

export function getConfiguredKnowledgebaseDriveAppSdkClient(): SdkworkDriveAppClient {
  return getKnowledgebasePcSdkPorts().getDriveClient();
}
