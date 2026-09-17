import { type KnowledgebaseAppSdkClient } from '@sdkwork/sdkwork-knowledgebase-harmony-mobile-core';

import { type KnowledgebaseListItem } from '../models/KnowledgeModels';

/**
 * Use-case orchestration for the knowledgebase catalog.
 *
 * Services receive explicit runtime inputs (injected SDK clients, config, host
 * adapters) and never construct SDK clients themselves.
 */
export class KnowledgeCatalogService {
  private readonly client: KnowledgebaseAppSdkClient;

  constructor(client: KnowledgebaseAppSdkClient) {
    this.client = client;
  }

  async listKnowledgebase(pageSize: number): Promise<KnowledgebaseListItem[]> {
    if (pageSize <= 0) {
      throw new Error('pageSize must be a positive integer');
    }
    // The ArkTS-adapted transport is not generated yet; the port is injected so
    // this service compiles unchanged once the adapter lands.
    return [];
  }

  baseUrl(): string {
    return this.client.baseUrl;
  }
}
