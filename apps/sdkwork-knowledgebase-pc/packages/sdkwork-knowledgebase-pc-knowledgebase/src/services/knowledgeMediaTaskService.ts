import { isBlank, trim } from '@sdkwork/utils';
import {
  KnowledgebaseErrorCodes,
  parseKnowledgeSpaceId,
  requireKnowledgebaseAppSdkHttpClient,
  requireKnowledgebaseTenantId,
  requireNonEmptyString,
  requirePrimaryRegisteredSpaceId,
  throwKnowledgebaseError} from 'sdkwork-knowledgebase-pc-core';
import { getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId } from '../workspaceMode';

export interface MediaTaskContext {
  spaceId?: string;
  documentId?: string;
}

export interface ImageGenerationResult {
  url: string;
  resolution: string;
  suggestions: string[];
  similars: string[];
}

function parseDocumentId(documentId?: string): string | undefined {
  if (isBlank(documentId)) {
    return undefined;
  }
  const trimmed = trim(documentId ?? '');
  if (!/^[0-9]+$/u.test(trimmed) || /^0+$/u.test(trimmed)) {
    return undefined;
  }
  return trimmed;
}

function resolveSpaceId(context?: MediaTaskContext): string {
  if (getActiveEphemeralFixedKnowledgebaseWorkspaceSpaceId() !== null) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.API_UNAVAILABLE_AI);
  }

  const spaceId = context?.spaceId;
  if (!isBlank(spaceId)) {
    return parseKnowledgeSpaceId(spaceId ?? '');
  }

  requireKnowledgebaseTenantId();
  return requirePrimaryRegisteredSpaceId();
}

export async function runSpeechToTextTask(
  audioUrl: string,
  context?: MediaTaskContext,
): Promise<string> {
  const client = requireKnowledgebaseAppSdkHttpClient();
  const result = await client.knowledge.mediaTasks.create({
    spaceId: resolveSpaceId(context),
    taskType: 'speech_to_text',
    sourceUrl: trim(audioUrl) || undefined,
    documentId: parseDocumentId(context?.documentId)});

  if (result.accepted !== true || isBlank(result.text)) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.SPEECH_NO_RESULT);
  }

  return result.text ?? '';
}

export async function runImageGenerationTask(
  prompt: string,
  aspectMode: string,
  styleMode: string,
  context?: MediaTaskContext,
): Promise<ImageGenerationResult> {
  const trimmedPrompt = requireNonEmptyString(prompt, KnowledgebaseErrorCodes.PROMPT_REQUIRED);

  const client = requireKnowledgebaseAppSdkHttpClient();
  const result = await client.knowledge.mediaTasks.create({
    spaceId: resolveSpaceId(context),
    taskType: 'generate_image',
    prompt: trimmedPrompt,
    aspectMode: trim(aspectMode) || '1:1',
    styleMode: trim(styleMode) || 'default'});

  if (result.accepted !== true || isBlank(result.url)) {
    throwKnowledgebaseError(KnowledgebaseErrorCodes.IMAGE_URL_MISSING);
  }

  return {
    url: result.url ?? '',
    resolution: result.resolution || '1024x1024',
    suggestions: result.suggestions,
    similars: result.similars};
}
