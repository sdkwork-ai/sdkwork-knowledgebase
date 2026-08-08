import { getPath } from '@sdkwork/utils';

export function extractSdkWorkListItems(
  payload: unknown,
): Record<string, unknown>[] {
  const items = getPath(payload, 'items');
  if (!Array.isArray(items)) {
    return [];
  }
  return items.filter(
    (item): item is Record<string, unknown> =>
      typeof item === 'object' && item !== null && !Array.isArray(item),
  );
}

/**
 * Returns the opaque next-cursor of a standard `{ items, pageInfo }` list payload, or
 * `undefined` when the page is exhausted. Consumers drive interactive pagination with
 * this cursor — they must never aggregate full collections client-side.
 */
export function extractNextCursor(payload: unknown): string | undefined {
  const pageInfo = getPath(payload, 'pageInfo');
  if (typeof pageInfo !== 'object' || pageInfo === null) {
    return undefined;
  }
  const nextCursor = (pageInfo as Record<string, unknown>).nextCursor;
  if (typeof nextCursor === 'string' && nextCursor.length > 0) {
    return nextCursor;
  }
  return undefined;
}

export function extractSdkWorkMemberItems(
  payload: unknown,
): Record<string, unknown>[] {
  const items = getPath(payload, 'items');
  if (Array.isArray(items)) {
    return items.filter(
      (item): item is Record<string, unknown> =>
        typeof item === 'object' && item !== null && !Array.isArray(item),
    );
  }

  const members = getPath(payload, 'members');
  if (!Array.isArray(members)) {
    return [];
  }
  return members.filter(
    (item): item is Record<string, unknown> =>
      typeof item === 'object' && item !== null && !Array.isArray(item),
  );
}

export interface AdminSpaceMemberRow {
  spaceId: string;
  spaceName: string;
  subjectType: string;
  subjectId: string;
  role: string;
  inherited: boolean;
}

export interface AdminSpaceRow {
  id: string;
  name: string;
  knowledgeMode: string;
  driveBound: boolean;
}

export async function loadAdminSpaceMembers(
  listMembers: (spaceId: string) => Promise<unknown>,
  spaces: AdminSpaceRow[],
): Promise<AdminSpaceMemberRow[]> {
  const memberRows = await Promise.all(
    spaces.map(async (space) => {
      if (!space.driveBound) {
        return [] as AdminSpaceMemberRow[];
      }
      try {
        const payload = await listMembers(space.id);
        return extractSdkWorkMemberItems(payload).map((member) => ({
          spaceId: space.id,
          spaceName: space.name,
          subjectType: readStringField(member, 'subjectType'),
          subjectId: readStringField(member, 'subjectId'),
          role: readStringField(member, 'role'),
          inherited: member.inherited === true,
        }));
      } catch {
        return [] as AdminSpaceMemberRow[];
      }
    }),
  );
  return memberRows.flat();
}

export function readStringField(payload: Record<string, unknown>, key: string): string {
  const value = payload[key];
  return value == null ? '' : String(value);
}

export function readOptionalStringField(
  payload: Record<string, unknown>,
  key: string,
): string | undefined {
  const value = payload[key];
  if (value == null) {
    return undefined;
  }
  const text = String(value);
  return text.length > 0 ? text : undefined;
}

export function readNumberField(payload: Record<string, unknown>, key: string): number {
  const value = payload[key];
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}
