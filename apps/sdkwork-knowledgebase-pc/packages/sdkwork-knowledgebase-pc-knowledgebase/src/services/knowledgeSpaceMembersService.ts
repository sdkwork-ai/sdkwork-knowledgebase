import type { KnowledgeSpaceMember, KnowledgeSpaceMemberRole } from 'sdkwork-knowledgebase-pc-core';
import { getKnowledgebaseAppSdkClient } from 'sdkwork-knowledgebase-pc-core';
import { normalizeSdkWorkListPage } from './sdkWorkListPage';

export interface KnowledgeSpaceMemberUi {
  name: string;
  email: string;
  role: 'admin' | 'editor' | 'viewer';
  avatar: string;
  inherited?: boolean;
}

function toUiRole(role: KnowledgeSpaceMemberRole): KnowledgeSpaceMemberUi['role'] {
  if (role === 'owner') {
    return 'admin';
  }
  if (role === 'writer') {
    return 'editor';
  }
  return 'viewer';
}

function toApiRole(role: KnowledgeSpaceMemberUi['role']): KnowledgeSpaceMemberRole {
  if (role === 'admin') {
    return 'owner';
  }
  if (role === 'editor') {
    return 'writer';
  }
  return 'reader';
}

function displayNameFromEmail(email: string): string {
  const localPart = email.split('@')[0] ?? email;
  return localPart.charAt(0).toUpperCase() + localPart.slice(1);
}

function avatarFromEmail(email: string): string {
  const name = displayNameFromEmail(email);
  return `https://ui-avatars.com/api/?name=${encodeURIComponent(name)}&background=random`;
}

/**
 * Frontend service facade for knowledge space member management.
 */
export class KnowledgeSpaceMembersService {
  static loadMembers(spaceId: string): Promise<KnowledgeSpaceMemberUi[]> {
    return loadKnowledgeSpaceMembers(spaceId);
  }

  static loadMembersPage(
    spaceId: string,
    cursor: string | null = null,
    pageSize = 20,
  ): Promise<KnowledgeSpaceMembersPage> {
    return loadKnowledgeSpaceMembersPage(spaceId, cursor, pageSize);
  }

  static syncMembers(
    spaceId: string,
    desired: KnowledgeSpaceMemberUi[],
    previous: KnowledgeSpaceMemberUi[],
  ): Promise<void> {
    return syncKnowledgeSpaceMembers(spaceId, desired, previous);
  }

  static syncMembersPartial(
    spaceId: string,
    uiMembers: KnowledgeSpaceMemberUi[],
    baselineMembers: KnowledgeSpaceMemberUi[],
    loadedEmails: ReadonlySet<string>,
  ): Promise<void> {
    const { desired, previous } = buildPartialMemberSyncPayload(
      uiMembers,
      baselineMembers,
      loadedEmails,
    );
    return syncKnowledgeSpaceMembers(spaceId, desired, previous);
  }
}

export async function loadKnowledgeSpaceMembers(
  spaceId: string,
): Promise<KnowledgeSpaceMemberUi[]> {
  const members: KnowledgeSpaceMemberUi[] = [];
  let cursor: string | null = null;

  do {
    const page = await loadKnowledgeSpaceMembersPage(spaceId, cursor, 100);
    members.push(...page.items);
    cursor = page.hasMore ? page.nextCursor : null;
  } while (cursor);

  return members;
}

export interface KnowledgeSpaceMembersPage {
  items: KnowledgeSpaceMemberUi[];
  nextCursor: string | null;
  hasMore: boolean;
}

/**
 * Builds a safe sync payload when the UI only shows a paginated subset of members.
 * Baseline members that were never loaded in the UI are preserved; removals apply
 * only to emails the user had loaded and then deleted from the list.
 */
export function buildPartialMemberSyncPayload(
  uiMembers: KnowledgeSpaceMemberUi[],
  baselineMembers: KnowledgeSpaceMemberUi[],
  loadedEmails: ReadonlySet<string>,
): { desired: KnowledgeSpaceMemberUi[]; previous: KnowledgeSpaceMemberUi[] } {
  const normalizeEmail = (email: string) => email.toLowerCase();
  const uiByEmail = new Map(uiMembers.map((member) => [normalizeEmail(member.email), member]));
  const baselineByEmail = new Map(
    baselineMembers.map((member) => [normalizeEmail(member.email), member]),
  );
  const desiredByEmail = new Map(baselineByEmail);

  for (const email of loadedEmails) {
    const normalized = normalizeEmail(email);
    const uiMember = uiByEmail.get(normalized);
    if (uiMember) {
      desiredByEmail.set(normalized, uiMember);
    } else if (baselineByEmail.has(normalized)) {
      desiredByEmail.delete(normalized);
    }
  }

  for (const [normalized, member] of uiByEmail) {
    if (!baselineByEmail.has(normalized)) {
      desiredByEmail.set(normalized, member);
    }
  }

  return {
    desired: Array.from(desiredByEmail.values()),
    previous: baselineMembers,
  };
}

export async function loadKnowledgeSpaceMembersPage(
  spaceId: string,
  cursor: string | null = null,
  pageSize = 20,
): Promise<KnowledgeSpaceMembersPage> {
  const spaceKey = String(spaceId);
  const client = getKnowledgebaseAppSdkClient().client;
  const page = normalizeSdkWorkListPage<KnowledgeSpaceMember>(
    await client.knowledge.spaces.members.list(spaceKey, { cursor: cursor ?? undefined, pageSize }),
  );
  const items: KnowledgeSpaceMemberUi[] = [];
  for (const member of page.items) {
    if (member.subjectType !== 'user') {
      continue;
    }
    items.push({
      name: displayNameFromEmail(member.subjectId),
      email: member.subjectId,
      role: toUiRole(member.role),
      avatar: avatarFromEmail(member.subjectId),
      inherited: member.inherited,
    });
  }
  return {
    items,
    nextCursor: page.nextCursor,
    hasMore: page.hasMore,
  };
}

export async function syncKnowledgeSpaceMembers(
  spaceId: string,
  desired: KnowledgeSpaceMemberUi[],
  previous: KnowledgeSpaceMemberUi[],
): Promise<void> {
  const client = getKnowledgebaseAppSdkClient().client;
  const spaceKey = String(spaceId);
  const previousByEmail = new Map(previous.map((member) => [member.email.toLowerCase(), member]));
  const desiredByEmail = new Map(desired.map((member) => [member.email.toLowerCase(), member]));

  for (const [email, member] of desiredByEmail) {
    const existing = previousByEmail.get(email);
    if (!existing || existing.role !== member.role) {
      await client.knowledge.spaces.members.create(spaceKey, {
        subjectType: 'user',
        subjectId: member.email,
        role: toApiRole(member.role),
      });
    }
  }

  for (const [email] of previousByEmail) {
    if (!desiredByEmail.has(email)) {
      await client.knowledge.spaces.members.delete(spaceKey, {
        subjectType: 'user',
        subjectId: email,
      });
    }
  }
}
