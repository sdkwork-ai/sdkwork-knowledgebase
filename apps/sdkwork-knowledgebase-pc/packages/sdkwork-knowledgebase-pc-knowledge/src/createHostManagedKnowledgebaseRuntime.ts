import {
  bindKnowledgebaseSessionStore,
  configureKnowledgebaseAgentsAppSdk,
  configureKnowledgebaseAppSdk,
  configureKnowledgebaseDriveAppSdk,
  createHostAdapter,
  createKnowledgebaseAgentsAppSdkClient,
  createKnowledgebaseAppSdkClient,
  createKnowledgebaseDriveAppSdkClient,
  createKnowledgebaseSessionTokenManager,
  createRuntimeConfig,
  createSessionStore,
  setKnowledgebaseApiEnabled,
  isKnowledgebaseAppApiConfigured,
  type KnowledgebasePcRuntime,
} from 'sdkwork-knowledgebase-pc-core';
import {
  configureKnowledgebaseBackendSdk,
  createKnowledgebaseBackendSdkClient,
  isKnowledgebaseBackendApiConfigured,
  setKnowledgebaseBackendApiEnabled,
} from 'sdkwork-knowledgebase-pc-admin-core';
import { installKnowledgebaseDerivedCacheCleanup } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';

import { getKnowledgebasePcSdkPorts } from './sdkPorts';
import { syncHostSessionIntoKnowledgebaseStore } from './sessionBridge';

export function createHostManagedKnowledgebaseRuntime(): KnowledgebasePcRuntime {
  const config = createRuntimeConfig({
    ...import.meta.env,
    VITE_SDKWORK_KNOWLEDGEBASE_TOKEN_STORAGE: 'memory',
    VITE_SDKWORK_KNOWLEDGEBASE_TOKEN_MANAGER_MODE: 'appbase-global',
  });
  const session = createSessionStore();
  const ports = getKnowledgebasePcSdkPorts();
  // APP_SDK_INTEGRATION_SPEC TokenManager closure rule: one TokenManager per
  // authenticated session context. Host-managed surfaces reuse the host's
  // shared instance; without the port the runtime owns exactly one of its own.
  const hostTokenManager = ports.getTokenManager?.();
  const tokenManager = hostTokenManager ?? createKnowledgebaseSessionTokenManager(session);
  if (hostTokenManager) {
    // Mirror session-store tokens into the host manager so every bound SDK
    // client reads current credentials even when the host relies on the
    // embedded session bridge instead of syncing its manager itself.
    session.subscribe((snapshot) => {
      if (snapshot.accessToken || snapshot.authToken || snapshot.refreshToken) {
        hostTokenManager.setTokens({
          accessToken: snapshot.accessToken,
          authToken: snapshot.authToken,
          refreshToken: snapshot.refreshToken,
        });
      }
    });
  }
  const appSdkClient = createKnowledgebaseAppSdkClient({
    config,
    sdkClient: ports.getKnowledgebaseClient(),
    tokenManager,
  });
  const driveSdkClient = createKnowledgebaseDriveAppSdkClient({
    config,
    sdkClient: ports.getDriveClient(),
    tokenManager,
  });
  const agentsSdkClient = createKnowledgebaseAgentsAppSdkClient({
    config,
    ...(ports.getAgentsClient ? { sdkClient: ports.getAgentsClient() } : {}),
    tokenManager,
  });

  const backendSdkClient = createKnowledgebaseBackendSdkClient({
    config,
    tokenManager,
  });

  bindKnowledgebaseSessionStore(session);
  // Installed before the host session is mirrored into the store so the first
  // identity the host publishes already invalidates the previous account's
  // derived caches (browser nodes, knowledge modes, agent-profile provisioning,
  // persisted tab rows).
  installKnowledgebaseDerivedCacheCleanup();
  configureKnowledgebaseAppSdk(appSdkClient);
  configureKnowledgebaseBackendSdk(backendSdkClient);
  configureKnowledgebaseDriveAppSdk(driveSdkClient);
  configureKnowledgebaseAgentsAppSdk(agentsSdkClient);
  setKnowledgebaseApiEnabled(
    config.auth.tokenManagerMode !== 'test'
    && isKnowledgebaseAppApiConfigured(config),
  );
  setKnowledgebaseBackendApiEnabled(
    config.auth.tokenManagerMode !== 'test'
    && isKnowledgebaseBackendApiConfigured(config),
  );
  syncHostSessionIntoKnowledgebaseStore(session);

  return {
    config,
    sdk: {
      app: appSdkClient,
      drive: driveSdkClient,
      agents: agentsSdkClient,
    },
    session,
    host: createHostAdapter(),
  };
}
