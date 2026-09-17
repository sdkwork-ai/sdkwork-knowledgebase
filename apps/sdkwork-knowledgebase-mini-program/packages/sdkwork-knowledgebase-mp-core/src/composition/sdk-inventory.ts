/** Mirrors specs/component.spec.json#contracts.sdkDependencies workspace order. */
export const sdkworkKnowledgebaseMpSdkInventory = [
  'sdkwork-drive-app-sdk',
  'sdkwork-iam-app-sdk',
  'sdkwork-knowledgebase-app-sdk',
] as const;

export function listSdkworkKnowledgebaseMpAppSdkFamilies() {
  return sdkworkKnowledgebaseMpSdkInventory;
}
