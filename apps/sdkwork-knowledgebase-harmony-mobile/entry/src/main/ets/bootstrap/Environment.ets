/**
 * HarmonyOS runtime environment resolution.
 *
 * Authority: `config/app/runtime-env.<deploymentProfile>.<environment>.json`
 * (see `CONFIG_SPEC.md` and `ENVIRONMENT_SPEC.md`). The build projects exactly
 * one selected non-secret runtime JSON resource; this module only reads it.
 */

export type KnowledgebaseHarmonyRuntimeTarget = 'harmony-native';

export interface KnowledgebaseHarmonyEnvironment {
  readonly environment: string;
  readonly deploymentProfile: string;
  readonly profileId: string;
  readonly runtimeTarget: KnowledgebaseHarmonyRuntimeTarget;
  readonly knowledgeApiBaseUrl: string;
  readonly knowledgeBackendApiBaseUrl: string;
  readonly appbaseAppApiBaseUrl: string;
  readonly appbaseLoginUrl: string;
}

const APP_API_SUFFIX: string = '/app/v3/api';

function requireNonEmpty(value: string | undefined, key: string): string {
  const normalized: string = value === undefined ? '' : value.trim();
  if (normalized.length === 0) {
    throw new Error(key + ' is required by the HarmonyOS runtime config');
  }
  return normalized;
}

function stripTrailingSlashes(value: string): string {
  let normalized: string = value.trim();
  while (normalized.endsWith('/')) {
    normalized = normalized.slice(0, normalized.length - 1);
  }
  return normalized;
}

export function normalizeKnowledgebaseAppApiBaseUrl(value: string): string {
  const normalized: string = stripTrailingSlashes(requireNonEmpty(value, 'knowledge.apiBaseUrl'));
  if (!normalized.endsWith(APP_API_SUFFIX)) {
    throw new Error('knowledge.apiBaseUrl must end with ' + APP_API_SUFFIX);
  }
  const prefix: string = normalized.slice(0, normalized.length - APP_API_SUFFIX.length);
  if (prefix.endsWith(APP_API_SUFFIX)) {
    throw new Error('knowledge.apiBaseUrl must contain ' + APP_API_SUFFIX + ' exactly once');
  }
  return normalized;
}

export function createKnowledgebaseHarmonyEnvironment(
  runtimeConfig: Record<string, Object>,
): KnowledgebaseHarmonyEnvironment {
  const knowledge: Record<string, Object> = runtimeConfig.knowledge as Record<string, Object>;
  const appbase: Record<string, Object> = runtimeConfig.appbase as Record<string, Object>;
  const target: string = requireNonEmpty(runtimeConfig.runtimeTarget as string, 'runtimeTarget');
  if (target !== 'harmony-native') {
    throw new Error('runtimeTarget must be harmony-native for this application root');
  }
  return {
    environment: requireNonEmpty(runtimeConfig.environment as string, 'environment'),
    deploymentProfile: requireNonEmpty(runtimeConfig.deploymentProfile as string, 'deploymentProfile'),
    profileId: requireNonEmpty(runtimeConfig.profileId as string, 'profileId'),
    runtimeTarget: 'harmony-native',
    knowledgeApiBaseUrl: normalizeKnowledgebaseAppApiBaseUrl(knowledge.apiBaseUrl as string),
    knowledgeBackendApiBaseUrl: requireNonEmpty(
      knowledge.backendApiBaseUrl as string,
      'knowledge.backendApiBaseUrl',
    ),
    appbaseAppApiBaseUrl: requireNonEmpty(appbase.appApiBaseUrl as string, 'appbase.appApiBaseUrl'),
    appbaseLoginUrl: requireNonEmpty(appbase.loginUrl as string, 'appbase.loginUrl'),
  };
}
