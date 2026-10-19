import { afterEach, describe, expect, it } from 'vitest';
import {
  bindKnowledgebaseSessionStore,
  configureKnowledgebaseAppSdk,
  setKnowledgebaseApiEnabled,
  type SessionSnapshot,
  type SessionStore,
} from 'sdkwork-knowledgebase-pc-core';

import {
  buildEditorActionPrompt,
  sendKnowledgeAgentMessage,
  sendKnowledgeAgentMessageWithCitations,
  synthesizeKnowledgeSearchAnswer,
} from './knowledgeAgentChatService';

const EMPTY_SESSION_STORE: SessionStore = createStaticSessionStore({});
const MOJIBAKE_PATTERN = /[\uFFFD\u951F]/;

function createStaticSessionStore(snapshot: SessionSnapshot): SessionStore {
  return {
    getSnapshot: () => snapshot,
    refreshSession: () => snapshot,
    setSession() {
      // Test fake; session mutation is covered by pc-core session tests.
    },
    clearSession() {
      // Test fake; session clearing is covered by pc-core session tests.
    },
    subscribe() {
      return () => {
        // Test fake; listener lifecycle is not relevant for prompt generation.
      };
    },
  };
}

function bindTenantContext(tenantId: string): void {
  bindKnowledgebaseSessionStore(
    createStaticSessionStore({
      context: {
        tenantId,
        userId: 'user-1',
      },
    }),
  );
}

function installRegisteredSpace(tenantId: string, spaceId: string): void {
  const storage = new Map<string, string>();
  storage.set(
    `sdkwork.knowledgebase.spaces.v1.${tenantId}`,
    JSON.stringify([
      {
        spaceId,
        kbType: 'team',
        createdAt: '2026-07-08T00:00:00.000Z',
      },
    ]),
  );

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
}

function configureFakeKnowledgeQueryClient(
  calls: Array<{ spaceId: string; query: string }>,
): void {
  configureKnowledgebaseAppSdk({
    client: {
      knowledge: {
        okf: {
          queries: {
            create: async (body: { spaceId: string; query: string }) => {
              calls.push(body);
              return { answerMarkdown: 'answer markdown' };
            },
          },
        },
      },
    } as never,
    setTokenManager() {
      // Test fake; token propagation is covered by pc-core SDK bootstrap tests.
    },
  });
  setKnowledgebaseApiEnabled(true);
}

afterEach(() => {
  bindKnowledgebaseSessionStore(EMPTY_SESSION_STORE);
  setKnowledgebaseApiEnabled(false);
  Reflect.deleteProperty(globalThis, 'window');
});

describe('knowledge agent prompt builders', () => {
  it('uses Rig defaults when an existing profile omits optional model routing fields', async () => {
    const tenantId = 'tenant-1';
    const chatRequests: Array<Record<string, unknown>> = [];
    bindTenantContext(tenantId);
    installRegisteredSpace(tenantId, '42');
    window.localStorage.setItem(
      `sdkwork.knowledgebase.spaceAgentProfile.v1.${tenantId}.42`,
      'profile-1',
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            retrieve: async () => ({ knowledgeMode: 'okf_bundle' }),
            update: async () => ({ profileId: 'profile-1' }),
            chat: {
              create: async (_profileId: string, request: Record<string, unknown>) => {
                chatRequests.push(request);
                return { answer: 'real provider answer' };
              },
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await expect(sendKnowledgeAgentMessage('question', { spaceId: '42' })).resolves.toBe(
      'real provider answer',
    );
    expect(chatRequests[0]).toMatchObject({
      modelProviderId: 'provider.model.rig-rust',
      modelId: 'rig.default-chat',
      agentImplementationId: 'plugin.intelligence.rig',
    });
  });

  it('returns citations with the answer while the string wrapper stays compatible', async () => {
    const tenantId = 'tenant-1';
    bindTenantContext(tenantId);
    installRegisteredSpace(tenantId, '42');
    window.localStorage.setItem(
      `sdkwork.knowledgebase.spaceAgentProfile.v1.${tenantId}.42`,
      'profile-1',
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            retrieve: async () => ({ knowledgeMode: 'rag' }),
            chat: {
              create: async () => ({
                answer: 'grounded answer',
                citations: [
                  {
                    documentId: '7',
                    title: 'Refund policy',
                    snippet: 'Accepted within 30 days',
                  },
                ],
              }),
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await expect(
      sendKnowledgeAgentMessageWithCitations('question', { spaceId: '42' }),
    ).resolves.toMatchObject({
      answer: 'grounded answer',
      citations: [{ documentId: '7', title: 'Refund policy' }],
    });
    await expect(sendKnowledgeAgentMessage('question', { spaceId: '42' })).resolves.toBe(
      'grounded answer',
    );
  });

  it('submits bounded conversation history with the chat request', async () => {    const tenantId = 'tenant-1';
    const chatRequests: Array<Record<string, unknown>> = [];
    bindTenantContext(tenantId);
    installRegisteredSpace(tenantId, '42');
    window.localStorage.setItem(
      `sdkwork.knowledgebase.spaceAgentProfile.v1.${tenantId}.42`,
      'profile-1',
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            retrieve: async () => ({ knowledgeMode: 'rag' }),
            chat: {
              create: async (_profileId: string, request: Record<string, unknown>) => {
                chatRequests.push(request);
                return { answer: 'grounded answer' };
              },
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await sendKnowledgeAgentMessage('follow up', {
      spaceId: '42',
      history: [
        { role: 'user', content: 'first question' },
        { role: 'assistant', content: 'first answer' },
        { role: 'user', content: '   ' },
      ],
    });

    expect(chatRequests[0]?.history).toEqual([
      { role: 'user', content: 'first question' },
      { role: 'assistant', content: 'first answer' },
    ]);
  });

  it('omits the history key when no prior turn is supplied', async () => {
    const tenantId = 'tenant-1';
    const chatRequests: Array<Record<string, unknown>> = [];
    bindTenantContext(tenantId);
    installRegisteredSpace(tenantId, '42');
    window.localStorage.setItem(
      `sdkwork.knowledgebase.spaceAgentProfile.v1.${tenantId}.42`,
      'profile-1',
    );
    configureKnowledgebaseAppSdk({
      client: {
        knowledge: {
          agentProfiles: {
            retrieve: async () => ({ knowledgeMode: 'rag' }),
            chat: {
              create: async (_profileId: string, request: Record<string, unknown>) => {
                chatRequests.push(request);
                return { answer: 'answer' };
              },
            },
          },
        },
      } as never,
      setTokenManager() {},
    });

    await sendKnowledgeAgentMessage('question', { spaceId: '42' });

    expect(chatRequests[0]).not.toHaveProperty('history');
  });

  it('builds readable editor action prompts without mojibake', () => {    const text = 'a'.repeat(4001);

    const prompt = buildEditorActionPrompt('summary', text, 'workspace context');

    expect(prompt).toContain('请用 Markdown 总结以下选中文本的核心要点（3-5 条）：');
    expect(prompt).toContain(`${'a'.repeat(4000)}…`);
    expect(prompt).toContain('文档上下文：\nworkspace context');
    expect(prompt).not.toMatch(MOJIBAKE_PATTERN);
  });

  it('interpolates the translation target instead of leaking template syntax', () => {
    const prompt = buildEditorActionPrompt('translate', 'plain English text', '');

    expect(prompt).toContain('请将以下文本翻译为Chinese，保留格式并只输出译文：');
    expect(prompt).not.toContain('{target}');
    expect(prompt).not.toMatch(MOJIBAKE_PATTERN);
  });

  it('sends a readable search synthesis prompt with citation constraints', async () => {
    const calls: Array<{ spaceId: string; query: string }> = [];
    bindTenantContext('tenant-1');
    installRegisteredSpace('tenant-1', '42');
    configureFakeKnowledgeQueryClient(calls);

    await expect(
      synthesizeKnowledgeSearchAnswer('权限模型', '[1] 权限设计文档'),
    ).resolves.toBe('answer markdown');

    expect(calls).toHaveLength(1);
    expect(calls[0]).toEqual({
      spaceId: '42',
      query: expect.stringContaining('用户搜索问题：权限模型'),
    });
    expect(calls[0]?.query).toContain('可用引用来源（正文引用序号必须与 [n] 对齐）：');
    expect(calls[0]?.query).toContain('[1] 权限设计文档');
    expect(calls[0]?.query).toContain('请输出结构化 Markdown 中文回答');
    expect(calls[0]?.query).not.toMatch(MOJIBAKE_PATTERN);
  });
});
