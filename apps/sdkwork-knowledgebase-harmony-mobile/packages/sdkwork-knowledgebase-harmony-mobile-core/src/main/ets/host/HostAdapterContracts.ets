/**
 * Host adapter contracts owned by core and implemented by the host package.
 *
 * Authority: `HARMONY_APP_MOBILE_ARCHITECTURE_SPEC.md` section 7. Host
 * adapters expose stable user-safe errors only.
 */

export type HostAdapterError =
  | 'unsupported'
  | 'permission-denied'
  | 'unavailable'
  | 'cancelled'
  | 'invalid-state';

export interface HostAdapterResult<T> {
  readonly ok: boolean;
  readonly value?: T;
  readonly error?: HostAdapterError;
}

export interface SecureStorageAdapter {
  get(key: string): Promise<HostAdapterResult<string>>;
  set(key: string, value: string): Promise<HostAdapterResult<void>>;
  remove(key: string): Promise<HostAdapterResult<void>>;
  clear(): Promise<HostAdapterResult<void>>;
}

export interface NetworkStatusAdapter {
  isOnline(): Promise<HostAdapterResult<boolean>>;
}

export interface AppLifecycleAdapter {
  currentState(): Promise<HostAdapterResult<string>>;
}
