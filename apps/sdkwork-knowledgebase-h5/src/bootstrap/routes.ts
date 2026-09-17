import { createKnowledgebaseRouteContributions } from "@sdkwork/knowledgebase-h5-knowledge/routes";

/**
 * Assemble route contributions for the H5 root.
 *
 * Route identity stays aligned with the PC workbench; physical H5 paths are
 * mapped by the shell navigation container
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7).
 */
export function createH5RouteContributions() {
  return createKnowledgebaseRouteContributions({
    presentation: "h5Mobile",
    surfaceRoot: "/",
  });
}
