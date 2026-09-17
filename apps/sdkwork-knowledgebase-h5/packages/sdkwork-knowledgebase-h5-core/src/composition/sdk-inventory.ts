/** Mirrors specs/component.spec.json#contracts.sdkDependencies workspace order. */
export const sdkworkKnowledgebaseH5SdkInventory = [
  'sdkwork-drive-app-sdk',
  'sdkwork-iam-app-sdk',
  'sdkwork-knowledgebase-app-sdk',
] as const;

export function listSdkworkKnowledgebaseH5AppSdkFamilies() {
  return sdkworkKnowledgebaseH5SdkInventory;
}
