import { describe, expect, it, vi } from 'vitest';
import {
  completeAgentTurnStream,
  type AgentRecord,
  type AgentSessionRecord,
  type SdkworkAgentsAppClient,
} from 'sdkwork-knowledgebase-pc-core';

import {
  buildWritingAgentTurnBody,
  ensureWritingAgentSession,
  listWritingAgents,
  reduceWritingAgentToolEvent,
  streamWritingAgentTurn,
} from './writingAgentService';

vi.mock('@sdkwork/agents-app-sdk', () => ({
  completeAgentTurnStream: vi.fn(),
}));

function agentRecord(overrides: Partial<AgentRecord> & Pick<AgentRecord, 'agentId' | 'code' | 'displayName'>): AgentRecord {
  return {
    id: `row-${overrides.agentId}`,
    tenantId: '100001',
    organizationId: '0',
    ownerUserId: '1',
    description: null,
    manifest: {},
    implementationType: 'sdkwork-native',
    status: 'active',
    visibility: 'public',
    tags: [],
    allowedRoleCodes: [],
    version: '0',
    createdAt: '2026-11-01T00:00:00.000Z',
    updatedAt: '2026-11-01T00:00:00.000Z',
    ...overrides,
  } as AgentRecord;
}

function listingClient(pages: AgentRecord[][]): SdkworkAgentsAppClient {
  return {
    ai: {
      agents: {
        list: vi.fn(async ({ page = 1 }: { page?: number }) => ({
          items: pages[page - 1] ?? [],
          pageInfo: { hasMore: page < pages.length },
        })),
      },
    },
  } as unknown as SdkworkAgentsAppClient;
}

describe('writing agent catalog', () => {
  it('keeps only writing agents, de-duplicates pages, and uses catalog order', async () => {
    const client = listingClient([
      [
        agentRecord({
          agentId: 'agent.writing.editor',
          code: 'writing-editor',
          displayName: '责任编辑',
          managementProfile: { categoryId: 'writing' },
        }),
        agentRecord({
          agentId: 'agent.programmer.fullstack',
          code: 'programmer-fullstack',
          displayName: '全栈开发工程师',
          managementProfile: { categoryId: 'engineering' },
        }),
      ],
      [
        agentRecord({
          agentId: 'agent.writing.novelist',
          code: 'writing-novelist',
          displayName: '小说创作家',
          managementProfile: { categoryId: 'writing', suggestedPrompts: ['续写这一章'] },
        }),
        agentRecord({
          agentId: 'agent.writing.editor',
          code: 'writing-editor',
          displayName: '责任编辑',
          managementProfile: { categoryId: 'writing' },
        }),
      ],
    ]);

    const agents = await listWritingAgents(client);

    expect(agents.map((agent) => agent.code)).toEqual(['writing-novelist', 'writing-editor']);
    expect(agents[0].suggestedPrompts).toEqual(['续写这一章']);
  });

  it('falls back to the manifest domain when the management profile omits a category', async () => {
    const client = listingClient([
      [
        agentRecord({
          agentId: 'agent.writing.news',
          code: 'writing-news-reporter',
          displayName: '新闻记者',
          manifest: { domain: 'writing' },
        }),
      ],
    ]);

    const agents = await listWritingAgents(client);

    expect(agents).toHaveLength(1);
    expect(agents[0].agentId).toBe('agent.writing.news');
  });
});

function sessionRecord(sessionId: string, status: AgentSessionRecord['status']): AgentSessionRecord {
  return {
    id: 'row-session',
    sessionId,
    tenantId: '100001',
    organizationId: '0',
    agentId: 'agent.writing.novelist',
    ownerUserId: '1',
    sessionKind: 'assistant',
    entrySurface: 'pc',
    titleSource: 'system',
    status,
    itemCount: '0',
    lastItemSequence: '0',
    totalInputTokens: '0',
    totalOutputTokens: '0',
    createdBy: '1',
    updatedBy: '1',
    version: '0',
    createdAt: '2026-11-01T00:00:00.000Z',
    updatedAt: '2026-11-01T00:00:00.000Z',
  } as AgentSessionRecord;
}

describe('writing agent sessions', () => {
  it('reuses a live session for the same agent', async () => {
    const retrieve = vi.fn(async () => sessionRecord('session-live', 'active'));
    const create = vi.fn();
    const client = {
      ai: { agents: { sessions: { retrieve, create } } },
    } as unknown as SdkworkAgentsAppClient;

    await expect(ensureWritingAgentSession({
      agentId: 'agent.writing.novelist',
      sessionId: 'session-live',
      sourceContextId: 'note-1',
    }, client)).resolves.toBe('session-live');
    expect(create).not.toHaveBeenCalled();
  });

  it('creates a server-generated session when the stored id is stale', async () => {
    const retrieve = vi.fn(async () => {
      throw new Error('not found');
    });
    const create = vi.fn(async (_agentId: string, body: unknown) => {
      expect(body).toMatchObject({
        sessionKind: 'assistant',
        entrySurface: 'pc',
        sourceModule: 'knowledgebase',
        sourceContextId: 'note-1',
      });
      expect((body as { payloadHash: string }).payloadHash).toMatch(/^sha256:/);
      return sessionRecord('session-new', 'active');
    });
    const client = {
      ai: { agents: { sessions: { retrieve, create } } },
    } as unknown as SdkworkAgentsAppClient;

    await expect(ensureWritingAgentSession({
      agentId: 'agent.writing.novelist',
      sessionId: 'session-stale',
      sourceContextId: 'note-1',
    }, client)).resolves.toBe('session-new');
    expect(create).toHaveBeenCalledTimes(1);
  });
});

describe('writing agent turn body', () => {
  it('carries the open note into the system prompt and hashes the payload', () => {
    const body = buildWritingAgentTurnBody('续写这一段', {
      title: '第一章',
      content: '夜色落在城墙上。',
    });

    expect(body.turnMode).toBe('interactive');
    expect(body.content).toBe('续写这一段');
    expect(body.systemPrompt ?? '').toContain('第一章');
    expect(body.systemPrompt ?? '').toContain('夜色落在城墙上。');
    expect(body.payloadHash).toMatch(/^sha256:/);
    expect(body.idempotencyKey).toMatch(/^kb-writing-turn-/);
    expect(body.clientRequestId).toBe(body.idempotencyKey);
  });
});

describe('writing agent MCP tool events', () => {
  it('creates, updates, and settles a tool card across start and result phases', () => {
    const started = reduceWritingAgentToolEvent([], {
      phase: 'start',
      toolCallId: 'call-1',
      toolName: 'read_note',
    });
    expect(started).toEqual([
      { name: 'read_note', arguments: {}, status: 'running', toolCallId: 'call-1' },
    ]);

    const settled = reduceWritingAgentToolEvent(started, {
      phase: 'stop',
      toolCallId: 'call-1',
      toolName: 'read_note',
      result: '{"title":"第一章"}',
    });
    expect(settled[0]).toMatchObject({
      name: 'read_note',
      status: 'success',
      result: '{"title":"第一章"}',
    });
  });

  it('marks a failed tool result as failed instead of success', () => {
    const started = reduceWritingAgentToolEvent([], {
      phase: 'start',
      toolCallId: 'call-2',
      toolName: 'update_note',
    });
    const failed = reduceWritingAgentToolEvent(started, {
      phase: 'stop',
      toolCallId: 'call-2',
      toolName: 'update_note',
      isError: true,
      result: 'permission denied',
    });
    expect(failed[0].status).toBe('failed');
  });
});

describe('streaming a writing agent turn', () => {
  it('forwards deltas, folds tool events, and returns the persisted assistant text', async () => {
    vi.mocked(completeAgentTurnStream).mockImplementation(async (_client, _agentId, _sessionId, _body, handlers) => {
      const resolved = handlers as {
        onDelta?: (delta: string) => void;
        onToolEvent?: (event: unknown) => void;
      };
      resolved.onDelta?.('第一');
      resolved.onToolEvent?.({ phase: 'start', toolCallId: 'c1', toolName: 'read_note' });
      resolved.onDelta?.('章');
      return {
        items: [
          { itemId: 'i1', kind: 'user_input', content: '写第一章' },
          { itemId: 'i2', kind: 'assistant_output', content: '第一章完成' },
        ],
        citations: [],
      } as never;
    });

    const deltas: string[] = [];
    const result = await streamWritingAgentTurn({
      agentId: 'agent.writing.novelist',
      sessionId: 'session-1',
      message: '写第一章',
    }, {
      onDelta: (delta) => deltas.push(delta),
    }, {} as SdkworkAgentsAppClient);

    expect(deltas).toEqual(['第一', '章']);
    expect(result.content).toBe('第一章完成');
    expect(result.toolCalls[0]).toMatchObject({ name: 'read_note', status: 'running' });
  });
});
