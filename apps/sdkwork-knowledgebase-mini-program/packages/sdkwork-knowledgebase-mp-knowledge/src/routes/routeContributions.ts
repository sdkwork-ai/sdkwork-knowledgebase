import { type KnowledgebaseMpRouteContribution } from "@sdkwork/knowledgebase-mp-shell";

import { KNOWLEDGEBASE_MP_CAPABILITY_ROUTE_IDS } from "../models/knowledgeModels";

const PAGE_PATHS: Record<(typeof KNOWLEDGEBASE_MP_CAPABILITY_ROUTE_IDS)[number], string> = {
  "app.intelligence.knowledgebase.list": "pages/knowledgebase/index",
  "app.intelligence.knowledgebase.detail": "pages/knowledgebase-detail/index",
  "app.intelligence.knowledgebase.search": "pages/knowledgebase-search/index",
  "app.intelligence.knowledgebase.settings": "pages/knowledgebase-detail/index",
  "app.intelligence.knowledgebase.launch": "pages/group-launch/index",
};

const SUBPACKAGE_BY_SCREEN: Record<string, string | undefined> = {
  detail: "knowledgebase-content",
  launch: "knowledgebase-group",
  search: "knowledgebase-content",
  settings: "knowledgebase-content",
};

/**
 * Route contributions for the knowledgebase capability.
 *
 * Route ids follow `<surface>.<domain>.<capability>.<screen>` and stay aligned
 * with the PC, H5, Flutter, and HarmonyOS roots. Route metadata must not
 * declare HTTP API paths, SDK methods, raw URL constants, or transport details.
 */
export const knowledgebaseMpRouteContributions: KnowledgebaseMpRouteContribution[] =
  KNOWLEDGEBASE_MP_CAPABILITY_ROUTE_IDS.map((id) => {
    const [, , , screen] = id.split(".");
    const subpackage = SUBPACKAGE_BY_SCREEN[screen];
    return {
      auth: "required",
      capability: "knowledgebase",
      domain: "intelligence",
      id,
      miniProgram: subpackage
        ? { pagePath: PAGE_PATHS[id], subpackage }
        : { pagePath: PAGE_PATHS[id], preload: true, rootPackage: true },
      permissionHint: "knowledge.read",
      screen,
      surface: "app",
      titleKey: `knowledgebase.${screen}.title`,
    } satisfies KnowledgebaseMpRouteContribution;
  });
