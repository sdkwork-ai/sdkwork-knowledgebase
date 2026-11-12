import { isBlank, sha256Hash, uuid } from '@sdkwork/utils';
import {
  KnowledgebaseErrorCodes,
  completeAgentTurnStream,
  getKnowledgebaseAgentsAppSdkClient,
  isKnowledgebaseAgentsApiAvailable,
  throwKnowledgebaseError,
  type AgentRecord,
  type AgentSessionRecord,
  type CreateAgentSessionRequest,
  type CreateAgentTurnRequest,
  type SdkworkAgentsAppClient,
  type TurnRichToolEvent,
} from 'sdkwork-knowledgebase-pc-core';

import type { McpToolCall } from './mcpAgent';

/**
 * Catalog category owned by `sdkwork-agents` for the writing agents
 * (`agent.writing.*`). The Agents app-api list operation has no category
 * filter, so the market page is bounded and filtered client-side by this
 * stable catalog value.
 */
export const WRITING_AGENT_CATEGORY_ID = 'writing';

/** `API_SPEC.md` §14 page-size ceiling for the Agents app-api list. */
const WRITING_AGENT_PAGE_SIZE = 200;
/** Defensive bound so a misbehaving pageInfo cannot loop forever. */
const WRITING_AGENT_MAX_PAGES = 4;

/** Catalog order keeps the picker stable across tenants and locales. */
const WRITING_AGENT_CODE_ORDER = [
  'writing-novelist',
  'writing-book-author',
  'writing-news-reporter',
  'writing-article-writer',
  'writing-copywriter',
  'writing-editor',
];

export interface WritingAgent {
  agentId: string;
  code: string;
  name: string;
  description: string;
  iconName?: string;
  color?: string;
  suggestedPrompts: string[];
  welcomeMessage?: string;
  tags: string[];
}

export interface WritingAgentDocumentContext {
  title?: string;
  content?: string;
  selectedText?: string;
}

export interface WritingAgentTurnResult {
  content: string;
  toolCalls: McpToolCall[];
  citations: Array<{
    documentId?: string | null;
    conceptId?: string | null;
    title: string;
    snippet?: string | null;
  }>;
}

export interface WritingAgentTurnHandlers {
  onDelta?: (delta: string) => void;
  onReasoning?: (reasoning: string) => void;
  onToolEvent?: (event: TurnRichToolEvent) => void;
}

export interface WritingAgentSessionInput {
  agentId: string;
  sessionId?: string;
  title?: string;
  sourceContextKind?: string;
  sourceContextId?: string;
}

function agentsClient(): SdkworkAgentsAppClient {
  if (!isKnowledgebaseAgentsApiAvailable()) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_AGENTS);
  }
  return getKnowledgebaseAgentsAppSdkClient().client;
}

function categoryOf(record: AgentRecord): string | undefined {
  const profileCategory = record.managementProfile?.categoryId;
  if (typeof profileCategory === 'string' && profileCategory.trim().length > 0) {
    return profileCategory.trim();
  }
  const manifestDomain = record.manifest?.['domain'];
  return typeof manifestDomain === 'string' ? manifestDomain.trim() : undefined;
}

function toWritingAgent(record: AgentRecord): WritingAgent {
  const profile = record.managementProfile;
  return {
    agentId: record.agentId,
    code: record.code,
    name: record.displayName,
    description: record.description ?? '',
    ...(profile?.iconName ? { iconName: profile.iconName } : {}),
    ...(profile?.color ? { color: profile.color } : {}),
    suggestedPrompts: profile?.suggestedPrompts ?? [],
    ...(profile?.welcomeMessage ? { welcomeMessage: profile.welcomeMessage } : {}),
    tags: record.tags ?? [],
  };
}

function compareWritingAgents(left: WritingAgent, right: WritingAgent): number {
  const leftIndex = WRITING_AGENT_CODE_ORDER.indexOf(left.code);
  const rightIndex = WRITING_AGENT_CODE_ORDER.indexOf(right.code);
  if (leftIndex !== -1 || rightIndex !== -1) {
    if (leftIndex === -1) return 1;
    if (rightIndex === -1) return -1;
    return leftIndex - rightIndex;
  }
  return left.name.localeCompare(right.name);
}

/**
 * Lists the public writing agents from `sdkwork-agents` through the generated
 * app SDK. No mock catalog: the picker only ever renders agents the backend
 * actually exposes.
 */
export async function listWritingAgents(
  client: SdkworkAgentsAppClient = agentsClient(),
): Promise<WritingAgent[]> {
  const records: AgentRecord[] = [];
  for (let page = 1; page <= WRITING_AGENT_MAX_PAGES; page += 1) {
    const response = await client.ai.agents.list({
      scope: 'market',
      page,
      pageSize: WRITING_AGENT_PAGE_SIZE,
    });
    records.push(...response.items);
    if (!response.pageInfo?.hasMore) {
      break;
    }
  }

  const seen = new Set<string>();
  return records
    .filter((record) => categoryOf(record) === WRITING_AGENT_CATEGORY_ID)
    .filter((record) => {
      if (seen.has(record.agentId)) {
        return false;
      }
      seen.add(record.agentId);
      return true;
    })
    .map(toWritingAgent)
    .sort(compareWritingAgents);
}

function isReusableSession(record: AgentSessionRecord): boolean {
  return record.status !== 'closed' && record.status !== 'archived' && !record.archivedAt;
}

function buildSessionCreateRequest(input: WritingAgentSessionInput): CreateAgentSessionRequest {
  const title = input.title?.trim() || 'Writing session';
  const canonical = {
    agentId: input.agentId,
    sessionKind: 'assistant',
    entrySurface: 'pc',
    sourceModule: 'knowledgebase',
    sourceContextKind: input.sourceContextKind ?? 'note',
    sourceContextId: input.sourceContextId ?? '',
    title,
  };
  return {
    sessionKind: 'assistant',
    entrySurface: 'pc',
    sourceModule: 'knowledgebase',
    ...(input.sourceContextKind ? { sourceContextKind: input.sourceContextKind } : {}),
    ...(input.sourceContextId ? { sourceContextId: input.sourceContextId } : {}),
    title,
    idempotencyKey: `kb-writing-session-${uuid()}`,
    payloadHash: `sha256:${sha256Hash(JSON.stringify(canonical))}`,
    requestedAt: new Date().toISOString(),
  };
}

/**
 * Reuses the caller's live session for this agent/document pair, otherwise
 * creates one through the Agents app SDK. The Agents app-api rejects
 * client-chosen session ids, so the returned id is always server-generated.
 */
export async function ensureWritingAgentSession(
  input: WritingAgentSessionInput,
  client: SdkworkAgentsAppClient = agentsClient(),
): Promise<string> {
  if (input.sessionId) {
    try {
      const existing = await client.ai.agents.sessions.retrieve(input.agentId, input.sessionId);
      if (isReusableSession(existing)) {
        return existing.sessionId;
      }
    } catch {
      // A missing or foreign session falls through to a fresh session.
    }
  }
  const created = await client.ai.agents.sessions.create(
    input.agentId,
    buildSessionCreateRequest(input),
  );
  const sessionId = created.sessionId;
  if (isBlank(sessionId)) {
    throw new Error('Agent session create did not return a sessionId.');
  }
  return sessionId;
}

const MAX_CONTEXT_CHARS = 6_000;

function buildSystemPrompt(context?: WritingAgentDocumentContext): string | undefined {
  if (!context) {
    return undefined;
  }
  const sections: string[] = [
    '你正在 SDKWork 知识库的写作工作台中与用户协作。',
    '用户当前打开的是知识库文档，你的回答要能直接用于该文档：保持与文档一致的语言、人称和文风。',
  ];
  if (context.title && context.title.trim().length > 0) {
    sections.push(`当前文档标题：${context.title.trim()}`);
  }
  if (context.selectedText && context.selectedText.trim().length > 0) {
    sections.push(
      `用户选中的文本（优先处理这段内容）：\n${context.selectedText.trim().slice(0, MAX_CONTEXT_CHARS)}`,
    );
  }
  if (context.content && context.content.trim().length > 0) {
    sections.push(
      `当前文档全文（节选，仅作上下文参考，不要全文复述）：\n${context.content.trim().slice(0, MAX_CONTEXT_CHARS)}`,
    );
  }
  sections.push('需要写入或修改知识库笔记时，调用已授权的 MCP 工具，并在回答中说明改动位置。');
  return sections.join('\n\n');
}

export function buildWritingAgentTurnBody(
  message: string,
  context?: WritingAgentDocumentContext,
): CreateAgentTurnRequest {
  const content = message.trim();
  const systemPrompt = buildSystemPrompt(context);
  const requestId = `kb-writing-turn-${uuid()}`;
  const payloadHash = `sha256:${sha256Hash(JSON.stringify({
    content,
    contentType: 'text/markdown',
    systemPrompt: systemPrompt ?? '',
  }))}`;
  return {
    content,
    contentType: 'text/markdown',
    turnMode: 'interactive',
    ...(systemPrompt ? { systemPrompt } : {}),
    idempotencyKey: requestId,
    payloadHash,
    clientRequestId: requestId,
    requestedAt: new Date().toISOString(),
  };
}

/**
 * Folds one kernel-v1 tool event into the chat transcript's tool cards. The
 * reducer is pure so the panel can update tool state while text is still
 * streaming.
 */
export function reduceWritingAgentToolEvent(
  toolCalls: McpToolCall[],
  event: TurnRichToolEvent,
): McpToolCall[] {
  const name = event.toolName ?? 'mcp_tool';
  const key = event.toolCallId ?? name;
  const index = toolCalls.findIndex((call, position) => {
    const callKey = (call as McpToolCall & { toolCallId?: string }).toolCallId ?? call.name;
    return callKey === key || (event.toolName !== undefined && call.name === event.toolName && position === toolCalls.length - 1);
  });
  const current = index >= 0
    ? toolCalls[index]
    : { name, arguments: {}, status: 'running' as const };
  const next: McpToolCall & { toolCallId?: string } = { ...current, toolCallId: key, name };
  if (event.phase === 'start') {
    next.status = 'running';
  } else if (event.phase === 'delta') {
    next.status = 'running';
  } else {
    next.status = event.isError ? 'failed' : 'success';
    if (event.result) {
      next.result = event.result;
    }
  }
  if (index < 0) {
    return [...toolCalls, next];
  }
  return toolCalls.map((call, position) => (position === index ? next : call));
}

function findAssistantOutput(
  items: Array<{ kind?: string; content?: string | null }>,
): { content: string } | undefined {
  for (let index = items.length - 1; index >= 0; index -= 1) {
    const item = items[index];
    if (item.kind === 'assistant_output') {
      return { content: item.content ?? '' };
    }
  }
  return undefined;
}

/**
 * Streams one writing-agent turn and returns the final assistant text plus the
 * MCP tool cards collected from the kernel-v1 stream. Streaming is fail-closed:
 * a terminal `error` frame throws the SDK problem, which the panel renders
 * through `resolveUserFacingErrorMessage`.
 */
export async function streamWritingAgentTurn(
  input: {
    agentId: string;
    sessionId: string;
    message: string;
    context?: WritingAgentDocumentContext;
  },
  handlers: WritingAgentTurnHandlers = {},
  client: SdkworkAgentsAppClient = agentsClient(),
): Promise<WritingAgentTurnResult> {
  if (isBlank(input.message)) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.MESSAGE_REQUIRED);
  }
  const toolCalls: McpToolCall[] = [];
  const completion = await completeAgentTurnStream(
    client,
    input.agentId,
    input.sessionId,
    buildWritingAgentTurnBody(input.message, input.context),
    {
      onDelta: handlers.onDelta,
      onReasoning: handlers.onReasoning,
      onToolEvent: (event) => {
        const next = reduceWritingAgentToolEvent(toolCalls, event);
        toolCalls.length = 0;
        toolCalls.push(...next);
        handlers.onToolEvent?.(event);
      },
    },
  );
  const assistant = findAssistantOutput(completion.items);
  if (!assistant) {
    throw new Error('Writing agent turn did not return an assistant_output item.');
  }
  return {
    content: assistant.content.trim(),
    toolCalls: [...toolCalls],
    citations: completion.citations ?? [],
  };
}
