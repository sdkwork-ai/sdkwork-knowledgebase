import { sdkworkKnowledgebaseH5SdkInventory } from "./sdk-inventory";

export interface SdkworkKnowledgebaseH5ModuleRegistration {
  moduleId: string;
  sdkWorkspaces: readonly string[];
}

export const sdkworkKnowledgebaseH5ModuleRegistry: readonly SdkworkKnowledgebaseH5ModuleRegistration[] = [
  {
    moduleId: 'knowledge',
    sdkWorkspaces: sdkworkKnowledgebaseH5SdkInventory,
  },
];

export function listSdkworkKnowledgebaseH5Modules() {
  return sdkworkKnowledgebaseH5ModuleRegistry;
}
