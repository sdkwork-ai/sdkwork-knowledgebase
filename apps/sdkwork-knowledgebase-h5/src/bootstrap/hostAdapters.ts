import { createKnowledgebaseH5BrowserHostAdapter } from "@sdkwork/knowledgebase-h5-core/host";

import { resolveKnowledgebaseH5Environment } from "./environment";

/**
 * Register the H5 browser host adapter set.
 *
 * Capacitor iOS/Android profiles register their own native host
 * implementations behind the same adapter interface
 * (`APP_H5_ARCHITECTURE_SPEC.md` host profile rules). Feature packages consume
 * host contracts only and never platform globals.
 */
export function registerHostAdapters() {
  const environment = resolveKnowledgebaseH5Environment();
  return createKnowledgebaseH5BrowserHostAdapter({
    capabilities: ["clipboard", "deepLinks", "networkStatus", "shareSheet"],
    runtimeTarget: environment.lifecycleEnvironment === "development" ? "development" : "browser",
  });
}
