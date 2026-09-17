/**
 * Route registry metadata shared by the shell container and the route
 * contributions assembled in `src/bootstrap/routes.ts`.
 */
export interface SdkworkKnowledgebaseH5RouteEntry {
  id: string;
  path: string;
  titleKey: string;
}

/** Physical H5 paths are a shell mapping concern; route ids stay cross-platform. */
export const KNOWLEDGEBASE_H5_ROUTE_REGISTRY: readonly SdkworkKnowledgebaseH5RouteEntry[] = [
  { id: 'app.intelligence.knowledgebase.list', path: '/knowledgebase', titleKey: 'list:title' },
  { id: 'app.intelligence.knowledgebase.detail', path: '/knowledgebase/:spaceId', titleKey: 'detail:title' },
  { id: 'app.intelligence.knowledgebase.search', path: '/knowledgebase/search', titleKey: 'search:title' },
  { id: 'app.intelligence.knowledgebase.settings', path: '/knowledgebase/:spaceId/settings', titleKey: 'settings:title' },
  { id: 'app.intelligence.knowledgebase.launch', path: '/group-launch', titleKey: 'launch:title' },
];

export function resolveKnowledgebaseH5RoutePath(routeId: string): string | undefined {
  return KNOWLEDGEBASE_H5_ROUTE_REGISTRY.find((entry) => entry.id === routeId)?.path;
}
