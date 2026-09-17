/**
 * Route contributions for the knowledge capability.
 *
 * Route ids follow `<surface>.<domain>.<capability>.<screen>` and are aligned
 * with the PC, H5, mini program, and Flutter roots. Route metadata must not
 * declare HTTP API paths, SDK methods, raw URL constants, or transport details.
 */
export interface KnowledgebaseRouteContribution {
  readonly id: string;
  readonly surface: 'app';
  readonly domain: string;
  readonly capability: string;
  readonly screen: string;
  readonly pagePath: string;
  readonly titleKey: string;
  readonly auth: 'public' | 'required';
  readonly permissionHint?: string;
}

export const knowledgebaseRouteContributions: KnowledgebaseRouteContribution[] = [
  {
    id: 'app.intelligence.knowledgebase.list',
    surface: 'app',
    domain: 'intelligence',
    capability: 'knowledgebase',
    screen: 'list',
    pagePath: 'pages/knowledgebase/KnowledgebaseList',
    titleKey: 'knowledgebase.list.title',
    auth: 'required',
    permissionHint: 'knowledge.knowledge.read',
  },
  {
    id: 'app.intelligence.knowledgebase.detail',
    surface: 'app',
    domain: 'intelligence',
    capability: 'knowledgebase',
    screen: 'detail',
    pagePath: 'pages/knowledgebase/KnowledgebaseDetail',
    titleKey: 'knowledgebase.detail.title',
    auth: 'required',
    permissionHint: 'knowledge.knowledge.read',
  },
  {
    id: 'app.intelligence.knowledgebase.search',
    surface: 'app',
    domain: 'intelligence',
    capability: 'knowledgebase',
    screen: 'search',
    pagePath: 'pages/knowledgebase/KnowledgebaseSearch',
    titleKey: 'knowledgebase.search.title',
    auth: 'required',
    permissionHint: 'knowledge.knowledge.read',
  },
  {
    id: 'app.intelligence.knowledgebase.settings',
    surface: 'app',
    domain: 'intelligence',
    capability: 'knowledgebase',
    screen: 'settings',
    pagePath: 'pages/knowledgebase/KnowledgebaseSettings',
    titleKey: 'knowledgebase.settings.title',
    auth: 'required',
    permissionHint: 'knowledge.knowledge.write',
  },
  {
    id: 'app.intelligence.knowledgebase.launch',
    surface: 'app',
    domain: 'intelligence',
    capability: 'knowledgebase',
    screen: 'launch',
    pagePath: 'pages/knowledgebase/GroupKnowledgebaseLaunch',
    titleKey: 'knowledgebase.launch.title',
    auth: 'required',
    permissionHint: 'knowledge.knowledge.read',
  },
];
