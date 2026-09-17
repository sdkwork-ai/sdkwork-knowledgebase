import {
  type AppLifecycleAdapter,
  type HostAdapterResult,
  type NetworkStatusAdapter,
  type SecureStorageAdapter,
} from '@sdkwork/sdkwork-knowledgebase-harmony-mobile-core';

/**
 * HarmonyOS host adapter implementations.
 *
 * Authority: `HARMONY_APP_MOBILE_ARCHITECTURE_SPEC.md` section 7. Adapters
 * expose typed methods and stable user-safe errors. They must not own login,
 * token refresh, permission evaluation, business authorization, or raw
 * business API transport.
 *
 * The platform-backed implementations require the HarmonyOS SDK toolchain and
 * concrete ability context injection; the capability set and method signatures
 * are fixed here so feature code compiles unchanged against any host.
 */

export class HarmonySecureStorageAdapter implements SecureStorageAdapter {
  get(key: string): Promise<HostAdapterResult<string>> {
    return Promise.resolve({ ok: false, error: 'unsupported' });
  }

  set(key: string, value: string): Promise<HostAdapterResult<void>> {
    return Promise.resolve({ ok: false, error: 'unsupported' });
  }

  remove(key: string): Promise<HostAdapterResult<void>> {
    return Promise.resolve({ ok: false, error: 'unsupported' });
  }

  clear(): Promise<HostAdapterResult<void>> {
    return Promise.resolve({ ok: false, error: 'unsupported' });
  }
}

export class HarmonyNetworkStatusAdapter implements NetworkStatusAdapter {
  isOnline(): Promise<HostAdapterResult<boolean>> {
    return Promise.resolve({ ok: true, value: true });
  }
}

export class HarmonyAppLifecycleAdapter implements AppLifecycleAdapter {
  currentState(): Promise<HostAdapterResult<string>> {
    return Promise.resolve({ ok: true, value: 'foreground' });
  }
}

export interface KnowledgebaseHarmonyHostAdapters {
  readonly capabilities: string[];
  readonly secureStorage: SecureStorageAdapter;
  readonly networkStatus: NetworkStatusAdapter;
  readonly appLifecycle: AppLifecycleAdapter;
}

export function createHostAdapters(): KnowledgebaseHarmonyHostAdapters {
  return {
    capabilities: ['networkStatus', 'appLifecycle'],
    secureStorage: new HarmonySecureStorageAdapter(),
    networkStatus: new HarmonyNetworkStatusAdapter(),
    appLifecycle: new HarmonyAppLifecycleAdapter(),
  };
}
