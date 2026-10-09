import type { ReactNode } from "react";

import { ScreenState } from "@sdkwork/knowledgebase-h5-commons";
import {
  createKnowledgebaseRouteContributions,
} from "@sdkwork/knowledgebase-h5-knowledge/routes";
import {
  GroupKnowledgebaseLaunchView,
  KnowledgebaseDetailView,
  KnowledgebaseListView,
  KnowledgebaseSearchView,
} from "@sdkwork/knowledgebase-h5-knowledge/pages";

/**
 * Assemble route contributions for the H5 root.
 *
 * Route identity stays aligned with the PC workbench; physical H5 paths are
 * mapped by the shell navigation container
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7).
 *
 * The shell renders `render()` for the active contribution, so every route id
 * must resolve to a concrete screen element. Screens receive the route title
 * (the segment before the i18n key suffix) and empty params until the shell
 * wires real hash routing.
 */
export function createH5RouteContributions() {
  const contributions = createKnowledgebaseRouteContributions({
    presentation: "h5Mobile",
    surfaceRoot: "/",
  });

  return contributions.map((route) => {
    const title = route.titleKey.split(":")[0] ?? route.screen;
    return {
      id: route.id,
      titleKey: route.titleKey,
      render: () => renderRouteScreen(route.id, title),
    };
  });
}

function renderRouteScreen(routeId: string, title: string): ReactNode {
  switch (routeId) {
    case "app.intelligence.knowledgebase.list":
      return <KnowledgebaseListView title={title} />;
    case "app.intelligence.knowledgebase.detail":
      return <KnowledgebaseDetailView title={title} />;
    case "app.intelligence.knowledgebase.search":
      return <KnowledgebaseSearchView title={title} />;
    case "app.intelligence.knowledgebase.launch":
      return <GroupKnowledgebaseLaunchView title={title} />;
    default:
      // Includes the settings screen, which has no view implementation yet:
      // render the standard empty screen instead of crashing the shell.
      return <ScreenState title={title} />;
  }
}
