import { AbilityContext } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';

import {
  createKnowledgebaseHarmonyEnvironment,
  type KnowledgebaseHarmonyEnvironment,
} from './Environment';
import { registerKnowledgebaseHarmonyHostAdapters } from './HostAdapters';
import { createKnowledgebaseHarmonyIamRuntime } from './IamRuntime';
import { createKnowledgebaseHarmonySdkClients } from './SdkClients';
import { createKnowledgebaseHarmonyRoutes } from './Routes';

const DOMAIN: number = 0x0000;
const TAG: string = 'SdkworkKnowledgebaseBootstrap';

/**
 * Root composition entry.
 *
 * This module assembles the environment, SDK clients, IAM runtime, host
 * adapters, and route registry only. It must not own product business logic.
 */
export interface KnowledgebaseHarmonyRuntime {
  readonly environment: KnowledgebaseHarmonyEnvironment;
  readonly routeCount: number;
}

export function bootstrapKnowledgebaseHarmonyApp(
  context: AbilityContext,
): KnowledgebaseHarmonyRuntime | null {
  try {
    const environment: KnowledgebaseHarmonyEnvironment = createKnowledgebaseHarmonyEnvironment(
      createKnowledgebaseHarmonyRuntimeConfig(),
    );
    createKnowledgebaseHarmonySdkClients().initialize(environment.knowledgeApiBaseUrl);
    createKnowledgebaseHarmonyIamRuntime();
    registerKnowledgebaseHarmonyHostAdapters();
    const routes = createKnowledgebaseHarmonyRoutes();
    hilog.info(DOMAIN, TAG, 'bootstrap completed for profile %{public}s', environment.profileId);
    return { environment, routeCount: routes.length };
  } catch (error) {
    hilog.error(DOMAIN, TAG, 'bootstrap failed: %{public}s', JSON.stringify(error));
    return null;
  }
}

/**
 * Runtime config projection.
 *
 * The build projects exactly one non-secret
 * `config/app/runtime-env.<deploymentProfile>.<environment>.json` resource
 * into the HAP. Until the resource projection task is wired into the hvigor
 * pipeline, bootstrap fails fast instead of falling back to a hand-written
 * environment.
 */
function createKnowledgebaseHarmonyRuntimeConfig(): Record<string, Object> {
  throw new Error(
    'Harmony runtime config projection is not wired yet: provide config/app/runtime-env.<profile-id>.json as an ArkTS resource module',
  );
}
