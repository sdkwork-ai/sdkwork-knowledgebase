import {
  configureKnowledgebaseAppSdkBaseUrl,
  getKnowledgebaseAppSdkClient,
  resetKnowledgebaseAppSdkClient,
  type KnowledgebaseAppSdkClient,
  type KnowledgebaseAppSdkClientConfig,
} from '@sdkwork/sdkwork-knowledgebase-harmony-mobile-core';

/**
 * SDK client construction for the HarmonyOS root.
 *
 * Authority: `APP_SDK_INTEGRATION_SPEC.md` and
 * `HARMONY_APP_MOBILE_ARCHITECTURE_SPEC.md` section 6. Concrete clients are
 * constructed here (bootstrap only) and injected into feature packages.
 */
export class KnowledgebaseHarmonySdkClients {
  private knowledgeClient: KnowledgebaseAppSdkClient | null = null;

  initialize(appApiBaseUrl: string, accessToken?: string): void {
    configureKnowledgebaseAppSdkBaseUrl(appApiBaseUrl);
    const config: KnowledgebaseAppSdkClientConfig = {
      baseUrl: appApiBaseUrl,
      accessToken: accessToken,
    };
    this.knowledgeClient = getKnowledgebaseAppSdkClient(config);
  }

  get knowledge(): KnowledgebaseAppSdkClient {
    if (this.knowledgeClient === null) {
      throw new Error('Knowledgebase app SDK client must be initialized by bootstrap before use');
    }
    return this.knowledgeClient;
  }

  reset(): void {
    resetKnowledgebaseAppSdkClient();
    this.knowledgeClient = null;
  }
}

let sdkClients: KnowledgebaseHarmonySdkClients | null = null;

export function createKnowledgebaseHarmonySdkClients(): KnowledgebaseHarmonySdkClients {
  sdkClients = new KnowledgebaseHarmonySdkClients();
  return sdkClients;
}

export function getKnowledgebaseHarmonySdkClients(): KnowledgebaseHarmonySdkClients {
  if (sdkClients === null) {
    return createKnowledgebaseHarmonySdkClients();
  }
  return sdkClients;
}
