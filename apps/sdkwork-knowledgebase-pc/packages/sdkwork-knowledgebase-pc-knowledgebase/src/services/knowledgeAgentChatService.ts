import type { KnowledgeAgentKnowledgeMode } from 'sdkwork-knowledgebase-pc-core';
import { isBlank } from '@sdkwork/utils';
import {
  KnowledgebaseErrorCodes,
  requireKnowledgebaseAppSdkHttpClient,
  requireKnowledgebaseTenantId,
  requireNonEmptyString,
  requirePrimaryRegisteredSpaceId,
  throwKnowledgebaseError,
} from 'sdkwork-knowledgebase-pc-core';

import { ensureSpaceAgentProfile } from './knowledgeSpaceSettingsService';
import {
  KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
  KNOWLEDGE_AGENT_RIG_IMPLEMENTATION_ID,
  KNOWLEDGE_AGENT_RIG_MODEL_PROVIDER_ID,
} from './knowledgeAgentDefaults';
import { getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId } from '../workspaceMode';


function resolvePrimarySpaceId(): string {
  requireKnowledgebaseTenantId();
  return requirePrimaryRegisteredSpaceId();
}

async function ensureDefaultAgentProfile(
  spaceId: string,
  persistSpaceProfileCache: boolean | undefined,
): Promise<string> {
  return ensureSpaceAgentProfile(spaceId, { persistCache: persistSpaceProfileCache });
}

export interface KnowledgeAgentMessageOptions {
  mode?: KnowledgeAgentKnowledgeMode;
  persistSpaceProfileCache?: boolean;
  sessionId?: string;
  spaceId?: string;
}

export async function sendKnowledgeAgentMessage(
  message: string,
  options?: KnowledgeAgentMessageOptions,
): Promise<string> {
  requireNonEmptyString(message, KnowledgebaseErrorCodes.MESSAGE_REQUIRED);

  if (getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_CHAT);
  }

  const spaceId = options?.spaceId ?? resolvePrimarySpaceId();
  const profileId = await ensureDefaultAgentProfile(spaceId, options?.persistSpaceProfileCache);
  const client = requireKnowledgebaseAppSdkHttpClient();
  const profile = await client.knowledge.agentProfiles.retrieve(profileId);

  const response = await client.knowledge.agentProfiles.chat.create(profileId, {
    message: message.trim(),
    mode: options?.mode ?? profile.knowledgeMode ?? 'okf_bundle',
    sessionId: options?.sessionId ?? null,
    modelProviderId: profile.modelProviderId ?? KNOWLEDGE_AGENT_RIG_MODEL_PROVIDER_ID,
    modelId: profile.modelId ?? KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
    agentImplementationId:
      profile.agentImplementationId ?? KNOWLEDGE_AGENT_RIG_IMPLEMENTATION_ID,
  });

  return response.answer.trim();
}

export async function queryKnowledgeSpace(
  spaceId: string,
  query: string,
): Promise<string> {
  if (getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_SEARCH);
  }

  if (isBlank(query)) {
    return '';
  }

  const client = requireKnowledgebaseAppSdkHttpClient();
  const result = await client.knowledge.okf.queries.create({
    spaceId,
    query: query.trim(),
  });
  return result.answerMarkdown.trim();
}

export async function synthesizeKnowledgeSearchAnswer(
  query: string,
  sourcesText: string,
): Promise<string> {
  if (getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_SEARCH);
  }

  requireKnowledgebaseTenantId();
  const spaceId = requirePrimaryRegisteredSpaceId();

  const prompt = `用户搜索问题：${query.trim()}

可用引用来源（正文引用序号必须与 [n] 对齐）：
${sourcesText || '（无可用引用来源）'}

请输出结构化 Markdown 中文回答：语气专业、引用序号准确、结尾给出 2-3 个追问方向。`;

  try {
    return await queryKnowledgeSpace(spaceId, prompt);
  } catch {
    return sendKnowledgeAgentMessage(prompt, { spaceId });
  }
}

export function buildEditorActionPrompt(
  action: string,
  text: string,
  context: string,
  customPrompt?: string,
): string {
  const snippet = text.length > 4000 ? `${text.slice(0, 4000)}…` : text;
  const contextBlock = isBlank(context) ? '' : `\n\n文档上下文：\n${context}`;

  switch (action) {
    case 'summary':
      return `请用 Markdown 总结以下选中文本的核心要点（3-5 条）：\n\n${snippet}${contextBlock}`;
    case 'translate': {
      const target = /[\u4e00-\u9fa5]/.test(snippet) ? 'English' : 'Chinese';
      return `请将以下文本翻译为${target}，保留格式并只输出译文：\n\n${snippet}${contextBlock}`;
    }
    case 'expand':
      return `请在保持原意的前提下扩写以下文本，输出 Markdown：\n\n${snippet}${contextBlock}`;
    case 'polish':
      return `请润色以下文本，提升可读性与专业度，输出 Markdown：\n\n${snippet}${contextBlock}`;
    case 'continue':
      return `请自然续写以下文本，保持风格一致，直接输出续写内容：\n\n${snippet}${contextBlock}`;
    case 'custom':
      return `${customPrompt?.trim() || '请按要求处理以下文本'}\n\n${snippet}${contextBlock}`;
    case 'chat':
      return `${snippet}${contextBlock}`;
    default:
      return `请处理以下文本并输出 Markdown：\n\n${snippet}${contextBlock}`;
  }
}
