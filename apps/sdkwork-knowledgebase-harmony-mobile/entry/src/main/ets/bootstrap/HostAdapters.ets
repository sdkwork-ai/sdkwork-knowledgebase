import {
  KnowledgebaseHarmonyHostAdapters,
  createHostAdapters,
} from '@sdkwork/sdkwork-knowledgebase-harmony-mobile-host';

/**
 * Host adapter registration.
 *
 * Authority: `HARMONY_APP_MOBILE_ARCHITECTURE_SPEC.md` section 7. Feature
 * packages depend on these typed interfaces and never call HarmonyOS system
 * APIs directly.
 */

let hostAdapters: KnowledgebaseHarmonyHostAdapters | null = null;

export function registerKnowledgebaseHarmonyHostAdapters(): KnowledgebaseHarmonyHostAdapters {
  hostAdapters = createHostAdapters();
  return hostAdapters;
}

export function getKnowledgebaseHarmonyHostAdapters(): KnowledgebaseHarmonyHostAdapters {
  if (hostAdapters === null) {
    return registerKnowledgebaseHarmonyHostAdapters();
  }
  return hostAdapters;
}
