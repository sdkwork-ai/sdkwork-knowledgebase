import {
  knowledgebaseRouteContributions,
  type KnowledgebaseRouteContribution,
} from '@sdkwork/sdkwork-knowledgebase-harmony-mobile-knowledge';

/**
 * Route/page registry assembly.
 *
 * Route ids follow `<surface>.<domain>.<capability>.<screen>` and stay aligned
 * with the PC, H5, mini program, and Flutter roots
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7). Physical Harmony
 * page paths may differ from other platforms.
 */

export function createKnowledgebaseHarmonyRoutes(): KnowledgebaseRouteContribution[] {
  return knowledgebaseRouteContributions;
}
