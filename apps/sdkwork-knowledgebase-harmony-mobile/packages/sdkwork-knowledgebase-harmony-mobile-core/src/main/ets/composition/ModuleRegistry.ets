import { knowledgebaseRouteContributions } from '@sdkwork/sdkwork-knowledgebase-harmony-mobile-knowledge';

/**
 * Module registry.
 *
 * The root bootstrap reads this registry instead of importing capability
 * packages directly.
 */
export interface KnowledgebaseHarmonyModuleRegistration {
  readonly id: string;
  readonly routeIdPrefix: string;
}

export function listKnowledgebaseHarmonyModules(): KnowledgebaseHarmonyModuleRegistration[] {
  return [
    { id: 'knowledge', routeIdPrefix: 'app.intelligence.knowledgebase.' },
  ];
}

export function listKnowledgebaseHarmonyModuleRouteCount(): number {
  return knowledgebaseRouteContributions.length;
}
