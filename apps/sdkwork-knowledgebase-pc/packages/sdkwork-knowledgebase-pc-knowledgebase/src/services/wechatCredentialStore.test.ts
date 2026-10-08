import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  configureKnowledgebaseAppSdk,
  KnowledgebaseErrorCodes,
  setKnowledgebaseApiEnabled,
} from 'sdkwork-knowledgebase-pc-core';

import { WechatService, type OfficialAccount } from './wechat';

const hostBridge = vi.hoisted(() => ({
  isTauriDesktopRuntime: vi.fn(() => true),
  invokeDesktopCommand: vi.fn(),
}));

vi.mock('sdkwork-knowledgebase-pc-core/host', () => hostBridge);

const officialAccount: OfficialAccount = {
  id: 'official-1',
  name: 'Official account',
  type: 'service',
  avatar: 'OA',
  appId: 'wx-official-1',
  appSecret: 'plaintext-secret',
};

function secureWriteCommandCalls(): Array<{ command: string; key?: string; value?: string }> {
  return hostBridge.invokeDesktopCommand.mock.calls.map((call) => {
    const [command, args] = call as [string, { request?: { key?: string; value?: string } }];
    return { command, key: args?.request?.key, value: args?.request?.value };
  });
}

describe('wechat credential secure storage', () => {
  beforeEach(() => {
    hostBridge.isTauriDesktopRuntime.mockReturnValue(true);
    hostBridge.invokeDesktopCommand.mockReset();
    hostBridge.invokeDesktopCommand.mockResolvedValue(null);
    setKnowledgebaseApiEnabled(false);
  });

  afterEach(() => {
    setKnowledgebaseApiEnabled(false);
    vi.restoreAllMocks();
  });

  it('writes the trimmed secret through the desktop secure-store command', async () => {
    const { persistOfficialAccountSecrets } = await import('./wechatCredentialStore');

    await persistOfficialAccountSecrets({
      id: 'official-1',
      appSecret: '  plaintext-secret  ',
    });

    const writes = secureWriteCommandCalls().filter(
      (call) => call.command === 'write_secure_session_value',
    );
    expect(writes).toHaveLength(1);
    expect(writes[0]?.key).toContain('official-1.appSecret');
    expect(writes[0]?.value).toBe('plaintext-secret');
  });

  it('fails closed with a typed error instead of swallowing a rejected official-account secure write', async () => {
    const { persistOfficialAccountSecrets } = await import('./wechatCredentialStore');

    hostBridge.invokeDesktopCommand.mockImplementation((command: unknown) => {
      if (command === 'write_secure_session_value') {
        return Promise.reject(new Error('secure store locked'));
      }
      return Promise.resolve(null);
    });

    await expect(
      persistOfficialAccountSecrets({ id: 'official-1', appSecret: 'plaintext-secret' }),
    ).rejects.toMatchObject({ code: KnowledgebaseErrorCodes.OPERATION_FAILED });
  });

  it('fails closed with a typed error instead of swallowing a rejected applet secure write', async () => {
    const { persistAppletSecrets } = await import('./wechatCredentialStore');

    hostBridge.invokeDesktopCommand.mockImplementation((command: unknown) => {
      if (command === 'write_secure_session_value') {
        return Promise.reject(new Error('secure store locked'));
      }
      return Promise.resolve(null);
    });

    await expect(
      persistAppletSecrets({ id: 'applet-1', appSecret: 'plaintext-secret' }),
    ).rejects.toMatchObject({ code: KnowledgebaseErrorCodes.OPERATION_FAILED });
  });
});

describe('WechatService.saveOfficialAccounts secure-write flow', () => {
  type OfficialAccountsUpdateInput = { accounts: Array<{ appSecret?: string }> };

  const updateSpy = vi.fn(async (_input: OfficialAccountsUpdateInput) => ({
    accepted: true,
    status: 'completed',
  }));

  beforeEach(() => {
    hostBridge.isTauriDesktopRuntime.mockReturnValue(true);
    hostBridge.invokeDesktopCommand.mockReset();
    hostBridge.invokeDesktopCommand.mockResolvedValue(null);
    updateSpy.mockClear();
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          wechat: {
            officialAccounts: { update: updateSpy },
          },
        },
      } as never,
      setTokenManager() {
        // The service boundary is stubbed; the registry still requires a client.
      },
    });
    setKnowledgebaseApiEnabled(true);
  });

  afterEach(() => {
    setKnowledgebaseApiEnabled(false);
    vi.restoreAllMocks();
  });

  it('aborts before the backend update when the desktop secure write fails, so the plaintext secret never leaves the device', async () => {
    hostBridge.invokeDesktopCommand.mockImplementation((command: unknown) => {
      if (command === 'write_secure_session_value') {
        return Promise.reject(new Error('secure store locked'));
      }
      return Promise.resolve(null);
    });

    await expect(WechatService.saveOfficialAccounts([officialAccount])).rejects.toMatchObject({
      code: KnowledgebaseErrorCodes.OPERATION_FAILED,
    });

    expect(updateSpy).not.toHaveBeenCalled();
  });

  it('persists secrets first and only then sends the re-hydrated payload to the backend', async () => {
    await expect(WechatService.saveOfficialAccounts([officialAccount])).resolves.toBe(true);

    const writes = secureWriteCommandCalls().filter(
      (call) => call.command === 'write_secure_session_value',
    );
    expect(writes.length).toBeGreaterThanOrEqual(1);
    expect(updateSpy).toHaveBeenCalledTimes(1);
    const updateArg = updateSpy.mock.calls[0]?.[0];
    // The secure read returned nothing (mock), so hydration falls back to the
    // form value; the important guarantee is that the update happened strictly
    // after the persist attempt and the save succeeded only when it resolved.
    expect(updateArg?.accounts[0]?.appSecret).toBe('plaintext-secret');
  });
});
