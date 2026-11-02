import { getKnowledgebaseAppSdkClient } from 'sdkwork-knowledgebase-pc-core';

/**
 * How one knowledge space answers content queries.
 *
 * The gateway owns this fact (`KnowledgeSpace.knowledgeMode`) and fixes it when
 * the space is created; no update route changes it, so a resolved value is
 * cached for the lifetime of the page.
 */
export type KnowledgeSpaceKnowledgeMode = 'rag' | 'okf_bundle' | 'external';

/**
 * Mode reported when the space read cannot answer.
 *
 * `okf_bundle` is the wire default of the contract, and it is the safe failure
 * direction: it keeps a transient space-read failure from turning into a
 * `POST /knowledge/retrievals` call that the gateway rejects with
 * `rag_retrieval_mode_required` (40003) for every OKF or external space.
 */
const UNRESOLVED_SPACE_KNOWLEDGE_MODE: KnowledgeSpaceKnowledgeMode = 'okf_bundle';

const knowledgeModeBySpaceId = new Map<string, KnowledgeSpaceKnowledgeMode>();

function normalizeKnowledgeSpaceMode(value: unknown): KnowledgeSpaceKnowledgeMode {
  if (value === 'rag' || value === 'external') {
    return value;
  }
  // `okf_bundle` is the contract default for a missing or unrecognized value.
  return 'okf_bundle';
}

function normalizeSpaceId(spaceId: string): string {
  return spaceId.trim();
}

/**
 * Resolve one space's knowledge mode, cached after the first successful read.
 *
 * `POST /knowledge/retrievals` performs RAG chunk retrieval and the gateway
 * refuses it for any space whose mode is not `rag`: OKF-bundle spaces must use
 * the OKF query/content APIs and external spaces their own engine. Every caller
 * that is about to read or search space content therefore has to know the mode
 * first, and must not fall back to retrieval when the mode is not `rag`.
 *
 * The read itself is advisory: a permission or transport failure must not fail
 * the caller's document or search request, and it must not resurrect the
 * rejected RAG call. Such a round answers with the contract default and caches
 * nothing, so a later call retries the read.
 *
 * @param spaceId - numeric knowledge space id.
 * @returns the space's knowledge mode; a space whose mode cannot be read
 *   reports the contract default (`okf_bundle`), never `rag`.
 */
export async function resolveKnowledgeSpaceKnowledgeMode(
  spaceId: string,
): Promise<KnowledgeSpaceKnowledgeMode> {
  const normalizedSpaceId = normalizeSpaceId(spaceId);
  if (!normalizedSpaceId) {
    return UNRESOLVED_SPACE_KNOWLEDGE_MODE;
  }

  const cached = knowledgeModeBySpaceId.get(normalizedSpaceId);
  if (cached !== undefined) {
    return cached;
  }

  try {
    const space = await getKnowledgebaseAppSdkClient().client.knowledge.spaces.retrieve(
      normalizedSpaceId,
    );
    const mode = normalizeKnowledgeSpaceMode(space.knowledgeMode);
    knowledgeModeBySpaceId.set(normalizedSpaceId, mode);
    return mode;
  } catch (error) {
    console.warn(
      `[KnowledgeSpaceKnowledgeMode] knowledge mode read failed for space ${normalizedSpaceId}.`,
      error,
    );
    return UNRESOLVED_SPACE_KNOWLEDGE_MODE;
  }
}

/**
 * Resolve several spaces concurrently and group them by mode.
 *
 * @param spaceIds - space ids to classify; blank ids and duplicates collapse.
 * @returns ids grouped by mode. A space whose mode read failed answers the
 *   contract default and lands in `okfBundle`, so retrieval callers never send
 *   an unresolved space to the rejected RAG endpoint.
 */
export async function partitionKnowledgeSpaceIdsByMode(
  spaceIds: readonly string[],
): Promise<{
  rag: string[];
  okfBundle: string[];
  external: string[];
}> {
  const uniqueSpaceIds = [...new Set(spaceIds.map(normalizeSpaceId).filter(Boolean))];
  const modes = await Promise.all(
    uniqueSpaceIds.map(async (spaceId) => ({
      spaceId,
      mode: await resolveKnowledgeSpaceKnowledgeMode(spaceId),
    })),
  );

  const partition = {
    rag: [] as string[],
    okfBundle: [] as string[],
    external: [] as string[],
  };
  for (const entry of modes) {
    partition[entry.mode === 'rag' ? 'rag' : entry.mode === 'external' ? 'external' : 'okfBundle'].push(
      entry.spaceId,
    );
  }
  return partition;
}

/**
 * Drop an entry (or the whole cache) so the next call re-reads the space.
 *
 * Space deletion and tenant switches invalidate the assumption that a cached
 * mode still describes a reachable space.
 *
 * @param spaceId - entry to drop; omit to clear every entry.
 */
export function forgetKnowledgeSpaceKnowledgeMode(spaceId?: string): void {
  if (spaceId === undefined) {
    knowledgeModeBySpaceId.clear();
    return;
  }
  knowledgeModeBySpaceId.delete(normalizeSpaceId(spaceId));
}
