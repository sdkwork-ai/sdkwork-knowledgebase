import { describe, expect, it } from 'vitest';

import { createHostManagedKnowledgebaseRuntime } from './createHostManagedKnowledgebaseRuntime';
import { configureKnowledgebasePcSdkPorts } from './sdkPorts';

interface ManagerRecorder {
  managers: unknown[];
  setTokenManager(manager: unknown): void;
}

function stubSdkClient(): ManagerRecorder {
  const recorder: ManagerRecorder = {
    managers: [],
    setTokenManager(manager: unknown) {
      recorder.managers.push(manager);
    },
  };
  return recorder;
}

function configurePorts(options: { hostTokenManager?: unknown }): void {
  const app = stubSdkClient();
  const drive = stubSdkClient();
  configureKnowledgebasePcSdkPorts({
    getKnowledgebaseClient: () => app as never,
    getDriveClient: () => drive as never,
    ...(options.hostTokenManager === undefined
      ? {}
      : { getTokenManager: () => options.hostTokenManager as never }),
    readHostSession: () => null,
  });
}

describe('createHostManagedKnowledgebaseRuntime', () => {
  it('binds the host token manager to every embedded sdk client', () => {
    const hostTokenManager = { getAccessToken: () => undefined };
    configurePorts({ hostTokenManager });

    const runtime = createHostManagedKnowledgebaseRuntime();

    const boundManagers = [
      ...((runtime.sdk.app.client as unknown as ManagerRecorder).managers),
      ...((runtime.sdk.drive.client as unknown as ManagerRecorder).managers),
    ];
    expect(boundManagers.length).toBeGreaterThan(0);
    for (const manager of boundManagers) {
      expect(manager).toBe(hostTokenManager);
    }
  });

  it('mirrors session-store tokens into the host token manager', () => {
    const mirrored: unknown[] = [];
    const hostTokenManager = {
      getAccessToken: () => undefined,
      setTokens: (tokens: unknown) => { mirrored.push(tokens); },
    };
    configurePorts({ hostTokenManager });

    const runtime = createHostManagedKnowledgebaseRuntime();
    runtime.session.setSession({ accessToken: 'access-1', authToken: 'auth-1' });

    expect(mirrored.at(-1)).toMatchObject({ accessToken: 'access-1', authToken: 'auth-1' });
  });

  it('mints exactly one own session-backed token manager without the host port', () => {
    configurePorts({});

    const runtime = createHostManagedKnowledgebaseRuntime();

    const appManager = (runtime.sdk.app.client as unknown as ManagerRecorder).managers[0];
    const driveManager = (runtime.sdk.drive.client as unknown as ManagerRecorder).managers[0];
    expect(appManager).toBeDefined();
    expect(appManager).toBe(driveManager);
    expect(appManager).not.toBeUndefined();
    expect(typeof (appManager as { getAccessToken?: unknown }).getAccessToken).toBe('function');
  });
});
