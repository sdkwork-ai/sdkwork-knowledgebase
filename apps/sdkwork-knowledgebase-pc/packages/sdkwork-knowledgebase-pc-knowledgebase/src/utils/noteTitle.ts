/**
 * Default-title allocation for knowledge notes.
 *
 * A note body is saved through the knowledge ingest surface, which resolves the
 * target document by `(space, title)`: the first API-owned document carrying
 * that exact title receives the new version. Two notes that share a title
 * therefore overwrite each other, so a created note needs a title that is
 * unique inside its space.
 */

/**
 * Build a note title that does not collide with any existing title.
 *
 * @param baseTitle - localized default title (`未命名笔记`, `Untitled note`, …).
 * @param existingTitles - every title currently visible in the same space.
 * @returns `baseTitle` when free, otherwise `"<baseTitle> 2"`, `"<baseTitle> 3"`, …
 */
export function buildUniqueNoteTitle(
  baseTitle: string,
  existingTitles: Iterable<string>,
): string {
  const base = baseTitle.trim();
  if (!base) {
    return baseTitle;
  }

  const taken = new Set<string>();
  for (const title of existingTitles) {
    const normalized = title.trim();
    if (normalized) taken.add(normalized);
  }
  if (!taken.has(base)) return base;

  for (let suffix = 2; suffix < 10_000; suffix += 1) {
    const candidate = `${base} ${suffix}`;
    if (!taken.has(candidate)) return candidate;
  }

  // An absurdly large space still gets a unique title rather than a duplicate.
  return `${base} ${Date.now().toString(36)}`;
}
