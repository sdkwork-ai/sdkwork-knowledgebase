import { resolveBaseUrl } from "@sdkwork/sdk-common";

export type KnowledgebaseH5DeploymentProfile = "standalone" | "cloud";
export type KnowledgebaseH5Environment = "development" | "test" | "staging" | "production";

export interface KnowledgebaseH5RuntimeConfig {
  appApiBaseUrl: string;
  appKey: "sdkwork-knowledgebase-h5";
  browserBasePath: string;
  deploymentProfile: KnowledgebaseH5DeploymentProfile;
  environment: KnowledgebaseH5Environment;
  featureFlags: {
    enableDemoMode: boolean;
    enableGroupLaunchTicket: boolean;
    enableSearchMedia: boolean;
  };
}

export const KNOWLEDGEBASE_H5_APP_KEY = "sdkwork-knowledgebase-h5" as const;

export interface CreateKnowledgebaseH5RuntimeConfigOptions {
  appApiBaseUrl: string;
  appKey?: string;
  browserBasePath?: string;
  deploymentProfile?: string;
  environment?: string;
  featureFlags?: Partial<KnowledgebaseH5RuntimeConfig["featureFlags"]>;
}

/**
 * Build the typed runtime config for the H5 root.
 *
 * Typed runtime config is resolved before SDK client construction
 * (`CONFIG_SPEC.md`; alignment spec section 8). The shared
 * `SDKWORK_API_BASE_URL` key wins over the config-derived value so protocol
 * adaptation stays in `@sdkwork/sdk-common`.
 */
export function createKnowledgebaseH5RuntimeConfig(
  options: CreateKnowledgebaseH5RuntimeConfigOptions,
): KnowledgebaseH5RuntimeConfig {
  if (options.appKey && options.appKey !== KNOWLEDGEBASE_H5_APP_KEY) {
    throw new Error(`unexpected H5 app key: ${options.appKey}`);
  }
  const shared = resolveBaseUrl({ envKey: "SDKWORK_API_BASE_URL" });
  const browserBasePath = options.browserBasePath ?? "/";

  return {
    appApiBaseUrl: shared.url || options.appApiBaseUrl,
    appKey: KNOWLEDGEBASE_H5_APP_KEY,
    browserBasePath: browserBasePath.endsWith("/") ? browserBasePath : `${browserBasePath}/`,
    deploymentProfile: options.deploymentProfile === "cloud" ? "cloud" : "standalone",
    environment: (["development", "test", "staging", "production"] as const).includes(
      options.environment as KnowledgebaseH5Environment,
    )
      ? (options.environment as KnowledgebaseH5Environment)
      : "development",
    featureFlags: {
      enableDemoMode: false,
      enableGroupLaunchTicket: true,
      enableSearchMedia: true,
      ...options.featureFlags,
    },
  };
}
