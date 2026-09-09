import { isBlank, trim } from '@sdkwork/utils';
import {resolveBaseUrlWithAlignProtocol} from '@sdkwork/sdk-common';

import { buildDependencySdkBaseUrls } from '../composition/dependency-runtime.js';
import { normalizeKnowledgebaseBrowserBasePath } from './browserBasePath';
import {
  resolveKnowledgebaseFeatureFlags,
  type KnowledgebaseFeatureFlags,
} from './knowledgebaseFeatureFlags';
import { isTauriDesktopRuntime } from '../host/tauriBridge';
export type SdkworkEnvironment = 'development' | 'test' | 'staging' | 'production';
export type SdkworkConfigProfile = 'dev' | 'test' | 'staging' | 'prod';
export type SdkworkBuildMode = 'development' | 'test' | 'staging' | 'production';
export type SdkworkDeploymentProfile = 'standalone' | 'cloud';
export type SdkworkRuntimeTarget =
  | 'browser'
  | 'desktop'
  | 'tablet-ipados'
  | 'tablet-android'
  | 'server'
  | 'container'
  | 'test-runner';

export interface SdkworkDependencySdkBaseUrls {
  openApiBaseUrl?: string;
  appApiBaseUrl?: string;
}

export interface SdkworkSdkBaseUrlConfig {
  defaultApiBaseUrl?: string;
  openApiBaseUrl?: string;
  appApiBaseUrl: string;
  dependencySdkBaseUrls: Record<string, SdkworkDependencySdkBaseUrls>;
}

export interface SdkworkAuthRuntimeConfig {
  tokenManagerMode: 'appbase-global' | 'service-context' | 'test';
  tokenStorage:
    | 'memory'
    | 'browser-session'
    | 'browser-local'
    | 'os-secure-storage'
    | 'server-context';
  accessTokenHeader: 'Access-Token';
  authTokenHeader: 'Authorization';
  refreshEnabled: boolean;
}

export interface KnowledgebaseRuntimeConfig {
  deploymentProfile: SdkworkDeploymentProfile;
  environment: SdkworkEnvironment;
  configProfile: SdkworkConfigProfile;
  buildMode: SdkworkBuildMode;
  runtimeTarget: SdkworkRuntimeTarget;
  browserBasePath: string;
  appKey: 'sdkwork-knowledgebase-pc';
  appApiBaseUrl: string;
  backendApiBaseUrl: string;
  openApiBaseUrl: string;
  platformApiGatewayBaseUrl: string;
  sdkBaseUrls: SdkworkSdkBaseUrlConfig;
  auth: SdkworkAuthRuntimeConfig;
  featureFlags: KnowledgebaseFeatureFlags;
}

export interface RuntimeEnv {
  VITE_SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_ENVIRONMENT?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_CONFIG_PROFILE?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_BUILD_MODE?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_RUNTIME_TARGET?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_BROWSER_BASE_PATH?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_URL?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_PLATFORM_API_GATEWAY_HTTP_URL?: string;
  VITE_SDKWORK_APPBASE_APP_API_BASE_URL?: string;
  VITE_SDKWORK_DRIVE_APP_API_BASE_URL?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_DEV_SAME_ORIGIN_API?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_TOKEN_MANAGER_MODE?: string;
  VITE_SDKWORK_KNOWLEDGEBASE_TOKEN_STORAGE?: string;
  DEV?: boolean;
  MODE?: string;
  PROD?: boolean;
}

const APP_KEY = 'sdkwork-knowledgebase-pc';
// base-url-check: exempt (standalone/test local-stack listeners and
// no-window SSR last-ditch fallbacks; browser cloud resolution goes through
// @sdkwork/sdk-common resolveBaseUrl below, ENVIRONMENT_SPEC §6.3)
const LOCAL_APP_API_BASE_URL = 'http://127.0.0.1:18081';
const LOCAL_OPEN_API_BASE_URL = 'http://127.0.0.1:18081';
const LOCAL_PLATFORM_API_GATEWAY_BASE_URL = 'http://127.0.0.1:3900';

const CLOUD_OPEN_API_BASE_URL = 'https://knowledge.sdkwork.com/knowledge/v3/api';

const VALID_ENVIRONMENTS: SdkworkEnvironment[] = [
  'development',
  'test',
  'staging',
  'production',
];
const VALID_CONFIG_PROFILES: SdkworkConfigProfile[] = ['dev', 'test', 'staging', 'prod'];
const VALID_BUILD_MODES: SdkworkBuildMode[] = [
  'development',
  'test',
  'staging',
  'production',
];
const VALID_DEPLOYMENT_PROFILES: SdkworkDeploymentProfile[] = ['standalone', 'cloud'];
const VALID_RUNTIME_TARGETS: SdkworkRuntimeTarget[] = [
  'browser',
  'desktop',
  'tablet-ipados',
  'tablet-android',
  'server',
  'container',
  'test-runner',
];

function normalized(value: string | undefined): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  const next = trim(value).toLowerCase();
  return isBlank(next) ? undefined : next;
}

function parseOneOf<T extends string>(
  value: string | undefined,
  validValues: readonly T[],
  fallback: T,
): T {
  const nextValue = normalized(value);
  if (nextValue && validValues.includes(nextValue as T)) {
    return nextValue as T;
  }
  return fallback;
}

function normalizeEnvironment(value: string | undefined, env: RuntimeEnv): SdkworkEnvironment {
  const nextValue = normalized(value);
  if (nextValue === 'dev') {
    return 'development';
  }
  if (nextValue === 'prod') {
    return 'production';
  }
  if (nextValue && VALID_ENVIRONMENTS.includes(nextValue as SdkworkEnvironment)) {
    return nextValue as SdkworkEnvironment;
  }
  if (env.PROD) {
    return 'production';
  }
  return 'development';
}

function normalizeProfile(
  value: string | undefined,
  environment: SdkworkEnvironment,
): SdkworkConfigProfile {
  const nextValue = normalized(value);
  if (nextValue && VALID_CONFIG_PROFILES.includes(nextValue as SdkworkConfigProfile)) {
    return nextValue as SdkworkConfigProfile;
  }
  if (environment === 'production') {
    return 'prod';
  }
  if (environment === 'development') {
    return 'dev';
  }
  return environment;
}

function normalizeBuildMode(
  value: string | undefined,
  env: RuntimeEnv,
  environment: SdkworkEnvironment,
): SdkworkBuildMode {
  const nextValue = normalized(value ?? env.MODE);
  if (nextValue === 'dev') {
    return 'development';
  }
  if (nextValue === 'prod') {
    return 'production';
  }
  return parseOneOf(nextValue, VALID_BUILD_MODES, environment);
}

function normalizeDeploymentProfile(
  value: string | undefined,
  env: RuntimeEnv,
  runtimeTarget: SdkworkRuntimeTarget,
  environment: SdkworkEnvironment,
): SdkworkDeploymentProfile {
  const explicit = normalized(value);
  if (explicit && VALID_DEPLOYMENT_PROFILES.includes(explicit as SdkworkDeploymentProfile)) {
    return explicit as SdkworkDeploymentProfile;
  }
  if (runtimeTarget === 'desktop' || environment === 'test' || env.DEV) {
    return 'standalone';
  }
  return 'cloud';
}

/**
 * Cloud platform-gateway default: resolved through the shared
 * `resolveBaseUrl` contract (ENVIRONMENT_SPEC.md §6.3) so the built cloud
 * page derives `api[-<env>].<brand>` from the current host and `pnpm dev`
 * cloud pages resolve to the local cloud-gateway dev port.
 */
function defaultPlatformApiGatewayBaseUrl(
  deploymentProfile: SdkworkDeploymentProfile,
  environment: SdkworkEnvironment,
): string {
  if (deploymentProfile === 'standalone' || environment === 'test') {
    return LOCAL_PLATFORM_API_GATEWAY_BASE_URL;
  }
  return resolveBaseUrlWithAlignProtocol({ mode: 'cloud' }).url;
}

function defaultAppApiBaseUrl(
  deploymentProfile: SdkworkDeploymentProfile,
  environment: SdkworkEnvironment,
): string {
  if (deploymentProfile === 'standalone' || environment === 'test') {
    return LOCAL_APP_API_BASE_URL;
  }
  return `${resolveBaseUrlWithAlignProtocol({ mode: 'cloud' }).url}/app/v3/api`;
}

function defaultOpenApiBaseUrl(
  deploymentProfile: SdkworkDeploymentProfile,
  environment: SdkworkEnvironment,
): string {
  if (deploymentProfile === 'standalone' || environment === 'test') {
    return LOCAL_OPEN_API_BASE_URL;
  }
  return CLOUD_OPEN_API_BASE_URL;
}

function normalizeTokenManagerMode(
  value: string | undefined,
  environment: SdkworkEnvironment,
): SdkworkAuthRuntimeConfig['tokenManagerMode'] {
  if (value === 'service-context' || value === 'test') {
    return value;
  }
  return environment === 'test' ? 'test' : 'appbase-global';
}

function normalizeTokenStorage(
  value: string | undefined,
  runtimeTarget: SdkworkRuntimeTarget,
  environment: SdkworkEnvironment,
): SdkworkAuthRuntimeConfig['tokenStorage'] {
  if (
    value === 'memory'
    || value === 'browser-session'
    || value === 'browser-local'
    || value === 'os-secure-storage'
    || value === 'server-context'
  ) {
    return value;
  }
  if (environment === 'test') {
    return 'memory';
  }
  return runtimeTarget === 'desktop' ? 'os-secure-storage' : 'browser-local';
}

function shouldUseDevSameOriginApi(
  env: RuntimeEnv,
  deploymentProfile: SdkworkDeploymentProfile,
  runtimeTarget: SdkworkRuntimeTarget,
  environment: SdkworkEnvironment,
): boolean {
  const explicit = normalized(env.VITE_SDKWORK_KNOWLEDGEBASE_DEV_SAME_ORIGIN_API);
  if (explicit === 'true' || explicit === '1') {
    return true;
  }
  if (explicit === 'false' || explicit === '0') {
    return false;
  }
  return Boolean(env.DEV)
    && normalized(env.MODE) === 'development'
    && deploymentProfile === 'standalone'
    && (runtimeTarget === 'desktop' || environment === 'development' || environment === 'test');
}

function applyDevSameOriginApiBaseUrl(
  env: RuntimeEnv,
  deploymentProfile: SdkworkDeploymentProfile,
  runtimeTarget: SdkworkRuntimeTarget,
  environment: SdkworkEnvironment,
  baseUrl: string,
): string {
  return shouldUseDevSameOriginApi(env, deploymentProfile, runtimeTarget, environment)
    ? ''
    : baseUrl;
}

export function isDevSameOriginApiEnabled(
  config: Pick<KnowledgebaseRuntimeConfig, 'deploymentProfile' | 'runtimeTarget' | 'environment'>,
  env: RuntimeEnv = import.meta.env,
): boolean {
  return shouldUseDevSameOriginApi(
    env,
    config.deploymentProfile,
    config.runtimeTarget,
    config.environment,
  );
}

export function isKnowledgebaseAppApiConfigured(
  config: KnowledgebaseRuntimeConfig,
  env: RuntimeEnv = import.meta.env,
): boolean {
  return Boolean(config.appApiBaseUrl || config.sdkBaseUrls.appApiBaseUrl)
    || isDevSameOriginApiEnabled(config, env);
}

export function detectRuntimeTargetFromEnv(env: RuntimeEnv = import.meta.env): SdkworkRuntimeTarget {
  const explicit = normalized(env.VITE_SDKWORK_KNOWLEDGEBASE_RUNTIME_TARGET);
  if (explicit && VALID_RUNTIME_TARGETS.includes(explicit as SdkworkRuntimeTarget)) {
    return explicit as SdkworkRuntimeTarget;
  }

  if (isTauriDesktopRuntime()) {
    return 'desktop';
  }

  return 'browser';
}

export function createRuntimeConfig(env: RuntimeEnv = import.meta.env): KnowledgebaseRuntimeConfig {
  const environment = normalizeEnvironment(env.VITE_SDKWORK_KNOWLEDGEBASE_ENVIRONMENT, env);
  const runtimeTarget = detectRuntimeTargetFromEnv(env);
  const browserBasePath = normalizeKnowledgebaseBrowserBasePath(
    env.VITE_SDKWORK_KNOWLEDGEBASE_BROWSER_BASE_PATH,
  );
  const deploymentProfile = normalizeDeploymentProfile(
    env.VITE_SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE,
    env,
    runtimeTarget,
    environment,
  );
  const configProfile = normalizeProfile(env.VITE_SDKWORK_KNOWLEDGEBASE_CONFIG_PROFILE, environment);
  const buildMode = normalizeBuildMode(env.VITE_SDKWORK_KNOWLEDGEBASE_BUILD_MODE, env, environment);

  const platformApiGatewayBaseUrl =
    env.VITE_SDKWORK_KNOWLEDGEBASE_PLATFORM_API_GATEWAY_HTTP_URL
    || defaultPlatformApiGatewayBaseUrl(deploymentProfile, environment);

  const appApiBaseUrl =
    env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_PUBLIC_HTTP_URL
    || defaultAppApiBaseUrl(deploymentProfile, environment);
  const backendApiBaseUrl =
    env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_BACKEND_HTTP_URL
    || appApiBaseUrl;
  const openApiBaseUrl =
    env.VITE_SDKWORK_KNOWLEDGEBASE_APPLICATION_OPEN_HTTP_URL
    || defaultOpenApiBaseUrl(deploymentProfile, environment);
  const appbaseAppApiBaseUrl = applyDevSameOriginApiBaseUrl(
    env,
    deploymentProfile,
    runtimeTarget,
    environment,
    env.VITE_SDKWORK_APPBASE_APP_API_BASE_URL
    || (deploymentProfile === 'standalone' ? appApiBaseUrl : platformApiGatewayBaseUrl),
  );
  const driveAppApiBaseUrl = applyDevSameOriginApiBaseUrl(
    env,
    deploymentProfile,
    runtimeTarget,
    environment,
    env.VITE_SDKWORK_DRIVE_APP_API_BASE_URL
    || platformApiGatewayBaseUrl,
  );

  const resolvedAppApiBaseUrl = applyDevSameOriginApiBaseUrl(
    env,
    deploymentProfile,
    runtimeTarget,
    environment,
    appApiBaseUrl,
  );
  const resolvedBackendApiBaseUrl = applyDevSameOriginApiBaseUrl(
    env,
    deploymentProfile,
    runtimeTarget,
    environment,
    backendApiBaseUrl,
  );
  const resolvedOpenApiBaseUrl = applyDevSameOriginApiBaseUrl(
    env,
    deploymentProfile,
    runtimeTarget,
    environment,
    openApiBaseUrl,
  );

  return {
    deploymentProfile,
    environment,
    configProfile,
    buildMode,
    runtimeTarget,
    browserBasePath,
    appKey: APP_KEY,
    appApiBaseUrl: resolvedAppApiBaseUrl,
    backendApiBaseUrl: resolvedBackendApiBaseUrl,
    openApiBaseUrl: resolvedOpenApiBaseUrl,
    platformApiGatewayBaseUrl,
    sdkBaseUrls: {
      defaultApiBaseUrl: resolvedAppApiBaseUrl,
      appApiBaseUrl: resolvedAppApiBaseUrl,
      openApiBaseUrl: resolvedOpenApiBaseUrl,
      dependencySdkBaseUrls: buildDependencySdkBaseUrls({
        appApiBaseUrl: resolvedAppApiBaseUrl,
        backendApiBaseUrl: resolvedBackendApiBaseUrl,
        openApiBaseUrl: resolvedOpenApiBaseUrl,
        iamAppApiBaseUrl: appbaseAppApiBaseUrl,
        driveAppApiBaseUrl,
      }),
    },
    auth: {
      tokenManagerMode: normalizeTokenManagerMode(
        env.VITE_SDKWORK_KNOWLEDGEBASE_TOKEN_MANAGER_MODE,
        environment,
      ),
      tokenStorage: normalizeTokenStorage(
        env.VITE_SDKWORK_KNOWLEDGEBASE_TOKEN_STORAGE,
        runtimeTarget,
        environment,
      ),
      accessTokenHeader: 'Access-Token',
      authTokenHeader: 'Authorization',
      refreshEnabled: environment !== 'test',
    },
    featureFlags: resolveKnowledgebaseFeatureFlags(
      environment,
      env as Record<string, string | undefined>,
    ),
  };
}
