import {
  KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS,
  type KnowledgeRouteParams,
} from "../models/knowledgeModels";

/**
 * Standard route contribution shape
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7).
 *
 * Route metadata must not declare HTTP API paths, SDK methods, raw URL
 * constants, or transport details.
 */
export interface SdkworkKnowledgebaseRouteContribution {
  auth: "public" | "required";
  capability: string;
  domain: string;
  id: string;
  params?: Array<{ name: string; required: boolean }>;
  path?: string;
  permissionHint?: string;
  presentation?: {
    h5Mobile?: "modal" | "sheet" | "stack" | "tab";
    pc?: "dialog" | "drawer" | "page";
  };
  screen: string;
  surface: "admin" | "app" | "console";
  titleKey: string;
}

const H5_PATHS: Record<(typeof KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS)[number], string> = {
  "app.intelligence.knowledgebase.list": "/knowledgebase",
  "app.intelligence.knowledgebase.detail": "/knowledgebase/:spaceId",
  "app.intelligence.knowledgebase.search": "/knowledgebase/search",
  "app.intelligence.knowledgebase.settings": "/knowledgebase/:spaceId/settings",
  "app.intelligence.knowledgebase.launch": "/group-launch",
};

export interface CreateKnowledgebaseRouteContributionsOptions {
  presentation?: "h5Mobile" | "pc";
  surfaceRoot?: string;
}

/** Assemble the aligned route contribution set for one physical surface. */
export function createKnowledgebaseRouteContributions(
  options: CreateKnowledgebaseRouteContributionsOptions = {},
): SdkworkKnowledgebaseRouteContribution[] {
  const presentation = options.presentation ?? "h5Mobile";
  return KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS.map((id) => {
    const [, , , screen] = id.split(".");
    return {
      auth: "required",
      capability: "knowledgebase",
      domain: "intelligence",
      id,
      ...(screen === "detail" || screen === "settings"
        ? { params: [{ name: "spaceId", required: true }] }
        : {}),
      path: H5_PATHS[id],
      permissionHint: "knowledge.read",
      presentation: { [presentation]: presentation === "pc" ? "page" : "stack" },
      screen,
      surface: "app",
      titleKey: `${screen}:title`,
    } satisfies SdkworkKnowledgebaseRouteContribution;
  });
}

export function resolveKnowledgebaseRouteParams(params: KnowledgeRouteParams): KnowledgeRouteParams {
  return {
    ...(params.documentId ? { documentId: params.documentId } : {}),
    ...(params.spaceId ? { spaceId: params.spaceId } : {}),
  };
}
