/**
 * SDK client packages consumed by Harmony core.
 *
 * Authority: `specs/component.spec.json` `contracts.sdkDependencies`.
 */
export const knowledgebaseHarmonyCoreSdkInventory: string[] = [
  'sdkwork-drive-app-sdk',
  'sdkwork-iam-app-sdk',
  'sdkwork-knowledgebase-app-sdk',
];

export function listSdkworkCoreSdkInventory(): string[] {
  return knowledgebaseHarmonyCoreSdkInventory;
}
