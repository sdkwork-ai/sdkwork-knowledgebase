/**
 * Workspace selection rules shared by the knowledge-base view and the notes
 * workspace.
 *
 * Both decisions used to live inline in the components, where they were neither
 * testable nor reused: a persisted tab row or a persisted selection was trusted
 * before the server answered, so an account switch could render a previous
 * account's document titles or open a space this account cannot read.
 */

/**
 * Keep only the persisted tab rows this round's tree still contains.
 *
 * The result is ordered like the cache and carries the tree's metadata: a cached
 * row can hold a stale title, or a title written by another account on the same
 * profile. Duplicate cache rows collapse to one.
 *
 * @param cachedTabs - persisted rows, in tab order.
 * @param treeDocuments - documents the server listed for this space this round.
 * @returns the rows worth rendering, in tab order.
 */
export function selectRenderableTabDocuments<T extends { id: string }>(
  cachedTabs: readonly T[],
  treeDocuments: readonly T[],
): T[] {
  const byId = new Map(treeDocuments.map((document) => [document.id, document]));
  const rendered: T[] = [];
  const seen = new Set<string>();
  for (const cachedTab of cachedTabs) {
    if (seen.has(cachedTab.id)) {
      continue;
    }
    const document = byId.get(cachedTab.id);
    if (!document) {
      continue;
    }
    seen.add(cachedTab.id);
    rendered.push(document);
  }
  return rendered;
}

/**
 * Decide whether a persisted knowledge-base selection may be reopened.
 *
 * Both the account's own registry and the current listing have to name the id:
 * the registry proves the space was registered for this account on this device,
 * the listing proves the server still returns it. A selection that fails either
 * check belongs to another account or to a deleted space and must be dropped
 * instead of retried on every mount.
 *
 * @param storedId - id from the persisted selection, if any.
 * @param registeredSpaceIds - ids registered for the signed-in account.
 * @param availableSpaceIds - ids this round's listing returned.
 * @returns whether the stored selection is still this account's.
 */
export function isStoredKnowledgeBaseCurrent(
  storedId: string | null | undefined,
  registeredSpaceIds: readonly string[],
  availableSpaceIds: readonly string[],
): boolean {
  const id = storedId?.trim();
  if (!id) {
    return false;
  }
  return registeredSpaceIds.includes(id) && availableSpaceIds.includes(id);
}
