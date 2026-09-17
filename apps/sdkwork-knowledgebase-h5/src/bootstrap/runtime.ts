import { resolveKnowledgebaseH5Environment } from "./environment";
import { registerHostAdapters } from "./hostAdapters";
import { createKnowledgebaseH5IamRuntime } from "./iamRuntime";
import { bootstrapSdkClients } from "./sdkClients";

/**
 * Application bootstrap.
 *
 * Assembles providers, routes, and shell wiring only. Business screens,
 * services, and state live in `packages/`.
 */
export function bootstrap() {
  const environment = resolveKnowledgebaseH5Environment();
  const iamRuntime = createKnowledgebaseH5IamRuntime();
  const sdkClients = bootstrapSdkClients();
  const host = registerHostAdapters();

  return { environment, host, iamRuntime, sdkClients };
}
