/** Mirrors pc-core specs/component.spec.json#contracts.sdkDependencies workspace order. */
export const sdkworkKnowledgebasePcSdkInventory = [
  'sdkwork-agents-app-sdk',
  'sdkwork-drive-app-sdk',
  'sdkwork-iam-app-sdk',
  'sdkwork-knowledgebase-app-sdk',
] as const;

export function listSdkworkKnowledgebasePcAppSdkFamilies() {
  return sdkworkKnowledgebasePcSdkInventory;
}
