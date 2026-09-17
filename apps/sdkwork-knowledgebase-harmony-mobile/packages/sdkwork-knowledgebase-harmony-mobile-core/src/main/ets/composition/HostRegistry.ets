/**
 * Host adapter capability registry.
 *
 * Renderer/feature code routes through the declared capability set; direct
 * platform branches on HarmonyOS globals are forbidden.
 */
export const knowledgebaseHarmonyHostCapabilities: string[] = [
  'secureStorage',
  'networkStatus',
  'appLifecycle',
  'deepLinks',
  'filePicker',
  'camera',
  'qrScanner',
  'pushNotifications',
  'clipboard',
  'deviceInfo',
];

export function hasKnowledgebaseHarmonyHostCapability(capability: string): boolean {
  return knowledgebaseHarmonyHostCapabilities.includes(capability);
}
