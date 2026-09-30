import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  bindKnowledgebaseSessionStore,
  configureKnowledgebaseAppSdk,
  type SessionSnapshot,
  type SessionStore,
} from 'sdkwork-knowledgebase-pc-core';

import { ensureSpaceAgentProfile } from './knowledgeSpaceSettingsService';

const EMPTY_SESSION_STORE = createStaticSessionStore({});

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {},
    clearSession() {},
    subscribe: () => () => {},
  };
}

afterEach(() => {
  bindKnowledgebaseSessionStore(EMPTY_SESSION_STORE);
  Reflect.deleteProperty(globalThis, 'window');
});

describe('knowledge space agent profile defaults', () => {
  it('creates a production Rig profile without advertising a different provider', async () => {
    const storage = new Map<string, string>();
    const createRequests: Array<Record<string, unknown>> = [];
    const bindingRequests: Array<Record<string, unknown>> = [];
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        localStorage: {
          getItem: (key: string) => storage.get(key) ?? null,
          setItem: (key: string, value: string) => storage.set(key, value),
          removeItem: (key: string) => storage.delete(key),
        },
      },
    });
    bindKnowledgebaseSessionStore(
      createStaticSessionStore({ context: { tenantId: 'tenant-1', userId: 'user-1' } }),
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            create: async (request: Record<string, unknown>) => {
              createRequests.push(request);
              return { profileId: 'profile-1' };
            },
            bindings: {
              create: async (_profileId: string, request: Record<string, unknown>) => {
                bindingRequests.push(request);
                return { accepted: true };
              },
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await expect(ensureSpaceAgentProfile('42')).resolves.toBe('profile-1');

    expect(createRequests).toHaveLength(1);
    expect(createRequests[0]).toMatchObject({
      modelProviderId: 'provider.model.rig-rust',
      modelId: 'rig.default-chat',
      agentImplementationId: 'plugin.intelligence.rig',
    });
    const modelParameters = JSON.parse(String(createRequests[0]?.modelParameters));
    expect(modelParameters).toMatchObject({
      uiProvider: 'SDKWork AI',
      uiModelName: 'rig.default-chat',
    });
    expect(bindingRequests).toHaveLength(1);
  });

  it('upgrades a cached contract profile to the production Rig runtime', async () => {
    const storage = new Map<string, string>();
    storage.set(
      'sdkwork.knowledgebase.spaceAgentProfile.v1.tenant-1.42',
      'legacy-profile',
    );
    const updateRequests: Array<Record<string, unknown>> = [];
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        localStorage: {
          getItem: (key: string) => storage.get(key) ?? null,
          setItem: (key: string, value: string) => storage.set(key, value),
          removeItem: (key: string) => storage.delete(key),
        },
      },
    });
    bindKnowledgebaseSessionStore(
      createStaticSessionStore({ context: { tenantId: 'tenant-1', userId: 'user-1' } }),
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            retrieve: async () => ({
              profileId: 'legacy-profile',
              name: 'Legacy profile',
              description: null,
              systemInstruction: 'Cite sources.',
              modelProviderId: 'provider.model.knowledgebase-contract',
              modelId: 'contract',
              modelParameters: JSON.stringify({
                temperature: 0.3,
                maxTokens: 1024,
                uiProvider: 'Google',
                uiModelName: 'gemini-3.5-flash',
              }),
              knowledgeMode: 'okf_bundle',
              agentImplementationId: 'plugin.intelligence.knowledgebase-contract',
              status: 'active',
            }),
            update: async (_profileId: string, request: Record<string, unknown>) => {
              updateRequests.push(request);
              return { profileId: 'legacy-profile' };
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await expect(ensureSpaceAgentProfile('42')).resolves.toBe('legacy-profile');
    expect(updateRequests).toHaveLength(1);
    expect(updateRequests[0]).toMatchObject({
      modelProviderId: 'provider.model.rig-rust',
      modelId: 'rig.default-chat',
      agentImplementationId: 'plugin.intelligence.rig',
    });
    expect(JSON.parse(String(updateRequests[0]?.modelParameters))).toMatchObject({
      temperature: 0.3,
      maxTokens: 1024,
      uiProvider: 'SDKWork AI',
      uiModelName: 'rig.default-chat',
    });
  });

  it('does not persist an agent profile cache for an ephemeral group workspace', async () => {
    const getItem = vi.fn(() => null);
    const setItem = vi.fn();
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        localStorage: {
          getItem,
          setItem,
          removeItem: vi.fn(),
        },
      },
    });
    bindKnowledgebaseSessionStore(
      createStaticSessionStore({ context: { tenantId: 'tenant-1', userId: 'user-1' } }),
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            create: async () => ({ profileId: 'group-profile-1' }),
            bindings: {
              create: async () => ({ accepted: true }),
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await expect(
      ensureSpaceAgentProfile('group-space-42', { persistCache: false }),
    ).resolves.toBe('group-profile-1');

    expect(getItem).not.toHaveBeenCalled();
    expect(setItem).not.toHaveBeenCalled();
  });
});
