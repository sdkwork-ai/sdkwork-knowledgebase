import { sdkworkKnowledgebaseMpSdkInventory } from "./sdk-inventory";

export interface SdkworkKnowledgebaseMpModuleRegistration {
  moduleId: string;
  sdkWorkspaces: readonly string[];
}

export const sdkworkKnowledgebaseMpModuleRegistry: readonly SdkworkKnowledgebaseMpModuleRegistration[] = [
  { moduleId: 'knowledge', sdkWorkspaces: sdkworkKnowledgebaseMpSdkInventory },
];

export function listSdkworkKnowledgebaseMpModules() {
  return sdkworkKnowledgebaseMpModuleRegistry;
}
