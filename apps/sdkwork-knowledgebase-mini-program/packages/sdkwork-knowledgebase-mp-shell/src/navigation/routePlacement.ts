/**
 * Mini program route placement metadata and projection inputs.
 *
 * SDKWork packages are source/dependency boundaries; platform `subpackages`
 * are runtime loading and package-size boundaries
 * (`MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md` section 5).
 */
export interface MiniProgramRoutePlacement {
  readonly pagePath: string;
  readonly preload?: boolean;
  readonly rootPackage?: boolean;
  readonly subpackage?: string;
}

export interface KnowledgebaseMpRouteContribution {
  readonly auth: "public" | "required";
  readonly capability: string;
  readonly domain: string;
  readonly id: string;
  readonly miniProgram: MiniProgramRoutePlacement;
  readonly permissionHint?: string;
  readonly screen: string;
  readonly surface: "app";
  readonly titleKey: string;
}

export interface KnowledgebaseMpPageProjectionEntry {
  readonly pagePath: string;
  readonly rootPackage: boolean;
  readonly subpackage?: string;
}

/** Project route contributions into `pages` / `subPackages` descriptors. */
export function projectKnowledgebaseMpPages(
  routes: KnowledgebaseMpRouteContribution[],
): KnowledgebaseMpPageProjectionEntry[] {
  return routes.map((route) => ({
    pagePath: route.miniProgram.pagePath,
    rootPackage: route.miniProgram.rootPackage === true,
    subpackage: route.miniProgram.subpackage,
  }));
}

export function listKnowledgebaseMpRootPages(routes: KnowledgebaseMpRouteContribution[]): string[] {
  return projectKnowledgebaseMpPages(routes)
    .filter((entry) => entry.rootPackage)
    .map((entry) => entry.pagePath);
}

export function listKnowledgebaseMpSubPackages(
  routes: KnowledgebaseMpRouteContribution[],
): Array<{ name: string; pages: string[] }> {
  const grouped = new Map<string, string[]>();
  for (const entry of projectKnowledgebaseMpPages(routes)) {
    if (entry.rootPackage || !entry.subpackage) {
      continue;
    }
    grouped.set(entry.subpackage, [...(grouped.get(entry.subpackage) ?? []), entry.pagePath]);
  }
  return [...grouped].map(([name, pages]) => ({ name, pages }));
}
