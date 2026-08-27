import type { KnowledgeAccessLevel, KnowledgeSpaceContextBinding } from 'sdkwork-knowledgebase-pc-core';
import { isBlank } from '@sdkwork/utils';
import {
  getKnowledgebaseAppSdkClient,
  requireKnowledgebaseTenantId} from 'sdkwork-knowledgebase-pc-core';

import type { KnowledgeBase } from './document';
import { normalizeSdkWorkListPage } from './sdkWorkListPage';
import {
  KNOWLEDGE_AGENT_DEFAULT_UI_PROVIDER,
  KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
  KNOWLEDGE_AGENT_RIG_IMPLEMENTATION_ID,
  KNOWLEDGE_AGENT_RIG_MODEL_PROVIDER_ID,
} from './knowledgeAgentDefaults';

const SPACE_AGENT_PROFILE_PREFIX = 'sdkwork.knowledgebase.spaceAgentProfile.v1';
const GUEST_CONTEXT_BINDING_ID = 'pc-knowledgebase-guest-link';
const GUEST_CONTEXT_BINDING_NAME = 'PC Guest Link';

interface ParsedModelParameters {
  temperature?: number;
  maxTokens?: number;
  uiProvider?: string;
  uiModelName?: string;
}

export interface EnsureSpaceAgentProfileOptions {
  persistCache?: boolean;
}

function readSpaceAgentProfileCache(tenantId: string, spaceId: string): string | null {
  if (typeof window === 'undefined') {
    return null;
  }
  return window.localStorage.getItem(`${SPACE_AGENT_PROFILE_PREFIX}.${tenantId}.${spaceId}`);
}

function writeSpaceAgentProfileCache(tenantId: string, spaceId: string, profileId: string): void {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(`${SPACE_AGENT_PROFILE_PREFIX}.${tenantId}.${spaceId}`, profileId);
}

function parseModelParameters(raw?: string | null): ParsedModelParameters {
  if (!raw) {
    return {};
  }
  try {
    return JSON.parse(raw) as ParsedModelParameters;
  } catch {
    return {};
  }
}

function buildModelParameters(
  updates: Partial<KnowledgeBase>,
  existing?: ParsedModelParameters,
): string {
  return JSON.stringify({
    temperature: updates.temperature ?? existing?.temperature,
    maxTokens: updates.maxTokens ?? existing?.maxTokens,
    uiProvider: updates.provider ?? existing?.uiProvider,
    uiModelName: updates.modelName ?? existing?.uiModelName,
  });
}

function toBackendModelId(provider?: string, modelName?: string): string {
  const trimmed = modelName?.trim();
  if (trimmed) {
    return trimmed;
  }
  if (provider === 'DeepSeek') {
    return 'deepseek-chat';
  }
  if (provider === 'OpenAI') {
    return 'gpt-4o-mini';
  }
  return KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID;
}

function mapAccessLevel(
  publicPermission?: KnowledgeBase['publicPermission'],
): KnowledgeAccessLevel {
  if (publicPermission === 'write' || publicPermission === 'admin') {
    return 'writer';
  }
  return 'reader';
}

function isLegacyContractProfile(profile: {
  agentImplementationId?: string | null;
  modelProviderId?: string | null;
}): boolean {
  return profile.agentImplementationId === 'plugin.intelligence.knowledgebase-contract'
    || profile.modelProviderId === 'provider.model.knowledgebase-contract';
}

export async function ensureSpaceAgentProfile(
  spaceId: string,
  options: EnsureSpaceAgentProfileOptions = {},
): Promise<string> {
  const tenantId = requireKnowledgebaseTenantId();
  const persistCache = options.persistCache ?? true;

  const cached = persistCache ? readSpaceAgentProfileCache(tenantId, spaceId) : null;
  const client = getKnowledgebaseAppSdkClient().client;
  if (cached && !isBlank(cached)) {
    try {
      const profile = await client.knowledge.agentProfiles.retrieve(cached);
      if (isLegacyContractProfile(profile)) {
        const existingParams = parseModelParameters(profile.modelParameters);
        await client.knowledge.agentProfiles.update(cached, {
          name: profile.name,
          description: profile.description,
          systemInstruction: profile.systemInstruction,
          modelProviderId: KNOWLEDGE_AGENT_RIG_MODEL_PROVIDER_ID,
          modelId: KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
          modelParameters: JSON.stringify({
            temperature: existingParams.temperature,
            maxTokens: existingParams.maxTokens,
            uiProvider: KNOWLEDGE_AGENT_DEFAULT_UI_PROVIDER,
            uiModelName: KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
          }),
          retrievalProfileId: profile.retrievalProfileId,
          citationPolicy: profile.citationPolicy,
          memoryPolicyRef: profile.memoryPolicyRef,
          toolPolicyRef: profile.toolPolicyRef,
          answerPolicy: profile.answerPolicy,
          knowledgeMode: profile.knowledgeMode ?? 'okf_bundle',
          agentImplementationId: KNOWLEDGE_AGENT_RIG_IMPLEMENTATION_ID,
          status: profile.status,
        });
      }
      return cached;
    } catch {
      // Stale cache entry; recreate profile.
    }
  }

  const profile = await client.knowledge.agentProfiles.create({
    name: `Knowledgebase Space ${spaceId}`,
    description: 'PC knowledge space assistant profile',
    systemInstruction:
      'You are a helpful knowledge assistant for this knowledge space. Answer accurately and cite sources when available.',
    modelProviderId: KNOWLEDGE_AGENT_RIG_MODEL_PROVIDER_ID,
    modelId: KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
    modelParameters: JSON.stringify({
      temperature: 0.7,
      maxTokens: 2048,
      uiProvider: KNOWLEDGE_AGENT_DEFAULT_UI_PROVIDER,
      uiModelName: KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
    }),
    agentImplementationId: KNOWLEDGE_AGENT_RIG_IMPLEMENTATION_ID,
    knowledgeMode: 'okf_bundle',
    status: 'active',
  });

  const profileId = String(profile.profileId ?? '').trim();
  if (isBlank(profileId)) {
    throw new Error('Agent profile create did not return profileId');
  }

  await client.knowledge.agentProfiles.bindings.create(profileId, {
    profileId,
    spaceId: String(spaceId),
    priority: 0,
    enabled: true,
  });

  if (persistCache) {
    writeSpaceAgentProfileCache(tenantId, spaceId, profileId);
  }
  return profileId;
}

export async function loadKnowledgeSpaceModelSettings(
  spaceId: string,
): Promise<Partial<KnowledgeBase>> {
  const profileId = await ensureSpaceAgentProfile(spaceId);
  const profile = await getKnowledgebaseAppSdkClient().client.knowledge.agentProfiles.retrieve(profileId);
  const params = parseModelParameters(profile.modelParameters);

  return {
    provider: params.uiProvider ?? KNOWLEDGE_AGENT_DEFAULT_UI_PROVIDER,
    modelName: params.uiModelName ?? profile.modelId ?? KNOWLEDGE_AGENT_RIG_DEFAULT_MODEL_ID,
    temperature: params.temperature ?? 0.7,
    maxTokens: params.maxTokens ?? 2048,
    systemPrompt: profile.systemInstruction,
  };
}

export async function applyKnowledgeSpaceModelSettings(
  spaceId: string,
  updates: Partial<KnowledgeBase>,
): Promise<void> {
  const profileId = await ensureSpaceAgentProfile(spaceId);
  const client = getKnowledgebaseAppSdkClient().client;
  const existing = await client.knowledge.agentProfiles.retrieve(profileId);
  const existingParams = parseModelParameters(existing.modelParameters);

  await client.knowledge.agentProfiles.update(profileId, {
    name: existing.name,
    description: existing.description,
    systemInstruction: updates.systemPrompt ?? existing.systemInstruction,
    modelProviderId: KNOWLEDGE_AGENT_RIG_MODEL_PROVIDER_ID,
    modelId: toBackendModelId(
      updates.provider ?? existingParams.uiProvider,
      updates.modelName ?? existingParams.uiModelName ?? existing.modelId,
    ),
    modelParameters: buildModelParameters(updates, existingParams),
    retrievalProfileId: existing.retrievalProfileId,
    citationPolicy: existing.citationPolicy,
    memoryPolicyRef: existing.memoryPolicyRef,
    toolPolicyRef: existing.toolPolicyRef,
    answerPolicy: existing.answerPolicy,
    knowledgeMode: existing.knowledgeMode ?? 'okf_bundle',
    agentImplementationId: KNOWLEDGE_AGENT_RIG_IMPLEMENTATION_ID,
    status: existing.status,
  });
}

async function findGuestContextBinding(spaceId: string) {
  const client = getKnowledgebaseAppSdkClient().client;
  const bindings = normalizeSdkWorkListPage<KnowledgeSpaceContextBinding>(
    await client.knowledge.spaces.contextBindings.list(String(spaceId)),
  );
  return bindings.items.find(
    (binding) => binding.contextId === GUEST_CONTEXT_BINDING_ID,
  );
}

export async function loadKnowledgeSpacePermissionSettings(
  spaceId: string,
): Promise<Pick<KnowledgeBase, 'publicPermission' | 'guestLinkEnabled'>> {
  const binding = await findGuestContextBinding(spaceId);
  if (!binding) {
    return { publicPermission: 'none', guestLinkEnabled: false };
  }

  const publicPermission =
    binding.accessLevel === 'writer' ? 'write' : 'read';
  return {
    publicPermission,
    guestLinkEnabled: true,
  };
}

export async function applyKnowledgeSpacePermissionSettings(
  spaceId: string,
  updates: Partial<KnowledgeBase>,
): Promise<void> {
  const shouldEnable =
    updates.guestLinkEnabled === true
    || (updates.publicPermission && updates.publicPermission !== 'none');

  const client = getKnowledgebaseAppSdkClient().client;
  const existing = await findGuestContextBinding(spaceId);

  if (!shouldEnable) {
    if (existing) {
      await client.knowledge.contextBindings.delete(existing.id);
    }
    return;
  }

  const accessLevel = mapAccessLevel(updates.publicPermission);
  if (existing) {
    await client.knowledge.contextBindings.update(existing.id, {
      contextName: GUEST_CONTEXT_BINDING_NAME,
      accessLevel,
    });
    return;
  }

  await client.knowledge.spaces.contextBindings.create(String(spaceId), {
    spaceId: String(spaceId),
    contextType: 'team',
    contextId: GUEST_CONTEXT_BINDING_ID,
    contextName: GUEST_CONTEXT_BINDING_NAME,
    accessLevel,
  });
}

export async function applyKnowledgeSpaceSettings(
  spaceId: string,
  updates: Partial<KnowledgeBase>,
): Promise<void> {
  const hasModelUpdates =
    updates.provider !== undefined
    || updates.modelName !== undefined
    || updates.temperature !== undefined
    || updates.maxTokens !== undefined
    || updates.systemPrompt !== undefined;

  if (hasModelUpdates) {
    await applyKnowledgeSpaceModelSettings(spaceId, updates);
  }

  if (updates.publicPermission !== undefined || updates.guestLinkEnabled !== undefined) {
    await applyKnowledgeSpacePermissionSettings(spaceId, updates);
  }
}

export async function hydrateKnowledgeBaseFromApi(
  kb: KnowledgeBase,
): Promise<KnowledgeBase> {
  const spaceId = String(kb.id);
  const numericSpaceId = Number(spaceId);
  if (!Number.isFinite(numericSpaceId) || numericSpaceId <= 0) {
    return kb;
  }

  try {
    const modelSettings = await loadKnowledgeSpaceModelSettings(spaceId);
    const permissionSettings = await loadKnowledgeSpacePermissionSettings(spaceId);
    return {
      ...kb,
      ...modelSettings,
      ...permissionSettings,
    };
  } catch {
    return kb;
  }
}
