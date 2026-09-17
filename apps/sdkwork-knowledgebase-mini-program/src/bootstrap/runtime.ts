import { bootstrapSdkClients, type KnowledgebaseMpSdkBootstrapOptions } from "./sdkClients";

export interface KnowledgebaseMiniProgramBootstrapOptions extends KnowledgebaseMpSdkBootstrapOptions {}

export function bootstrap(options: KnowledgebaseMiniProgramBootstrapOptions = {}) {
  return { ready: true, ...bootstrapSdkClients(options) };
}
