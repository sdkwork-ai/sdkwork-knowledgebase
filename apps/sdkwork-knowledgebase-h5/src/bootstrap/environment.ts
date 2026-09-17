export type KnowledgebaseLifecycleEnvironment =
  | "development"
  | "test"
  | "staging"
  | "production";

export type KnowledgebaseRuntimeEnvironment = Readonly<Record<string, unknown>>;

export interface KnowledgebaseH5Environment {
  appApiBaseUrl: string;
  appbaseAppApiBaseUrl: string;
  appbaseLoginUrl: string;
  browserBasePath: string;
  lifecycleEnvironment: KnowledgebaseLifecycleEnvironment;
  platformApiGatewayBaseUrl: string;
}

const APP_API_SUFFIX = "/app/v3/api";
const DEVELOPMENT_APPLICATION_PUBLIC_HTTP_URL = "http://127.0.0.1:18081";
const LIFECYCLE_ENVIRONMENTS = new Set<KnowledgebaseLifecycleEnvironment>([
  "development",
  "test",
  "staging",
  "production",
]);

function readEnv(environment: KnowledgebaseRuntimeEnvironment, key: string): string | undefined {
  const value = environment[key];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

/** Normalize a gateway base URL and reject a duplicated `/app/v3/api` suffix. */
export function normalizeAppbaseGatewayBaseUrl(value: string): string {
  const normalized = value.trim().replace(/\/+$/u, "");
  if (!normalized) {
    throw new Error("Appbase IAM gateway URL must not be empty.");
  }
  let parsed: URL;
  try {
    parsed = new URL(normalized);
  } catch {
    throw new Error("Appbase IAM gateway URL must be an absolute HTTP(S) URL.");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("Appbase IAM gateway URL must use HTTP or HTTPS.");
  }
  if (parsed.search || parsed.hash) {
    throw new Error("Appbase IAM gateway URL must not include a query string or fragment.");
  }
  if (normalized.endsWith(`${APP_API_SUFFIX}${APP_API_SUFFIX}`)) {
    throw new Error(`Appbase IAM gateway URL must not include ${APP_API_SUFFIX} more than once.`);
  }
  return normalized.endsWith(APP_API_SUFFIX)
    ? normalized.slice(0, -APP_API_SUFFIX.length)
    : normalized;
}

function resolveLifecycleEnvironment(
  environment: KnowledgebaseRuntimeEnvironment,
): KnowledgebaseLifecycleEnvironment {
  const configured = readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_ENVIRONMENT")
    ?? readEnv(environment, "MODE");
  if (configured) {
    const normalized = configured.toLowerCase();
    if (LIFECYCLE_ENVIRONMENTS.has(normalized as KnowledgebaseLifecycleEnvironment)) {
      return normalized as KnowledgebaseLifecycleEnvironment;
    }
    throw new Error(
      "VITE_SDKWORK_KNOWLEDGEBASE_H5_ENVIRONMENT must be development, test, staging, or production.",
    );
  }
  return environment.PROD === true || environment.PROD === "true"
    ? "production"
    : "development";
}

function resolveApplicationPublicHttpUrl(environment: KnowledgebaseRuntimeEnvironment): string {
  return readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_APPLICATION_PUBLIC_HTTP_URL")
    ?? (typeof window === "undefined" ? DEVELOPMENT_APPLICATION_PUBLIC_HTTP_URL : window.location.origin);
}

function resolveBrowserBasePath(environment: KnowledgebaseRuntimeEnvironment): string {
  const configured = readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_BROWSER_BASE_PATH") ?? "/";
  return configured.endsWith("/") ? configured : `${configured}/`;
}

export function createKnowledgebaseH5Environment(
  environment: KnowledgebaseRuntimeEnvironment,
): KnowledgebaseH5Environment {
  const lifecycleEnvironment = resolveLifecycleEnvironment(environment);
  const applicationPublicHttpUrl = resolveApplicationPublicHttpUrl(environment)
    .replace(/\/+$/u, "");
  const appbaseAppApiBaseUrl = normalizeAppbaseGatewayBaseUrl(
    readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_PLATFORM_API_GATEWAY_HTTP_URL")
      ?? readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_APPBASE_APP_API_BASE_URL")
      ?? applicationPublicHttpUrl,
  );

  return {
    appApiBaseUrl: readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_APP_API_BASE_URL")
      ?? `${applicationPublicHttpUrl}${APP_API_SUFFIX}`,
    appbaseAppApiBaseUrl,
    appbaseLoginUrl: readEnv(environment, "VITE_SDKWORK_KNOWLEDGEBASE_H5_APPBASE_LOGIN_URL")
      ?? appbaseAppApiBaseUrl,
    browserBasePath: resolveBrowserBasePath(environment),
    lifecycleEnvironment,
    platformApiGatewayBaseUrl: appbaseAppApiBaseUrl,
  };
}

export function resolveKnowledgebaseH5Environment(): KnowledgebaseH5Environment {
  return createKnowledgebaseH5Environment(import.meta.env as KnowledgebaseRuntimeEnvironment);
}
