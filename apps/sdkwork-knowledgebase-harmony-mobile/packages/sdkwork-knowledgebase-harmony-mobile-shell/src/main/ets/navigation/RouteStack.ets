/**
 * Navigation stack assembly for the HarmonyOS mobile shell.
 *
 * Route ids follow `<surface>.<domain>.<capability>.<screen>` and stay aligned
 * with the PC, H5, mini program, and Flutter roots. Physical Harmony page paths
 * may differ while route ids stay stable.
 */
export interface SdkworkKnowledgebaseRouteRegistration {
  readonly id: string;
  readonly pagePath: string;
  readonly titleKey: string;
  readonly authRequired: boolean;
}

export function createSdkworkKnowledgebaseRouteStack(
  routes: SdkworkKnowledgebaseRouteRegistration[],
): SdkworkKnowledgebaseRouteRegistration[] {
  return routes.filter((route) => route.pagePath.length > 0);
}
