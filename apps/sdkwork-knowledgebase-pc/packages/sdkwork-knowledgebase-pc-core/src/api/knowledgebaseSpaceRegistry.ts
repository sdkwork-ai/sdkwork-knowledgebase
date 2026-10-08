import type { SessionStore } from '../session/sessionStore';
import { withCrossTabStorageUpdate } from './knowledgebaseCrossTabStorage';

export type KnowledgebaseSpaceKbType = 'team' | 'personal' | 'public';

export type KnowledgebaseSpaceId = string;

export interface RegisteredKnowledgebaseSpace {
  spaceId: KnowledgebaseSpaceId;
  kbType: KnowledgebaseSpaceKbType;
  icon?: string;
  avatar?: string;
  createdAt: string;
}

const REGISTRY_KEY_PREFIX = 'sdkwork.knowledgebase.spaces.v1';

let sessionStoreRef: SessionStore | null = null;

export function bindKnowledgebaseSessionStore(session: SessionStore): void {
  sessionStoreRef = session;
}

export function getKnowledgebaseTenantId(): string | undefined {
  return sessionStoreRef?.getSnapshot().context?.tenantId;
}

function registryStorageKey(tenantId: string): string {
  return `${REGISTRY_KEY_PREFIX}.${tenantId}`;
}

function normalizeRegisteredSpaceId(value: unknown): string {
  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (/^[0-9]+$/.test(trimmed)) {
      return trimmed;
    }
  }
  if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
    if (!Number.isSafeInteger(value)) {
      return '';
    }
    return String(Math.trunc(value));
  }
  return '';
}

export function readRegisteredSpaces(tenantId: string): RegisteredKnowledgebaseSpace[] {
  if (typeof window === 'undefined') {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(registryStorageKey(tenantId));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw) as RegisteredKnowledgebaseSpace[];
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.map((space) => ({
      ...space,
      spaceId: normalizeRegisteredSpaceId(space.spaceId),
    }));
  } catch {
    return [];
  }
}

export function writeRegisteredSpaces(
  tenantId: string,
  spaces: RegisteredKnowledgebaseSpace[],
): void {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(registryStorageKey(tenantId), JSON.stringify(spaces));
}

export function upsertRegisteredSpace(
  tenantId: string,
  entry: RegisteredKnowledgebaseSpace,
): RegisteredKnowledgebaseSpace[] {
  return withCrossTabStorageUpdate(
    registryStorageKey(tenantId),
    () => readRegisteredSpaces(tenantId),
    (spaces) => {
      const next = spaces.filter((space) => space.spaceId !== entry.spaceId);
      next.push(entry);
      return next;
    },
    (next) => writeRegisteredSpaces(tenantId, next),
  );
}

export function removeRegisteredSpace(tenantId: string, spaceId: string): RegisteredKnowledgebaseSpace[] {
  return withCrossTabStorageUpdate(
    registryStorageKey(tenantId),
    () => readRegisteredSpaces(tenantId),
    (spaces) => spaces.filter((space) => space.spaceId !== spaceId),
    (next) => writeRegisteredSpaces(tenantId, next),
  );
}

export function updateRegisteredSpace(
  tenantId: string,
  spaceId: string,
  patch: Partial<
    Pick<
      RegisteredKnowledgebaseSpace,
      | 'kbType'
      | 'icon'
      | 'avatar'
    >
  >,
): RegisteredKnowledgebaseSpace[] {
  return withCrossTabStorageUpdate(
    registryStorageKey(tenantId),
    () => readRegisteredSpaces(tenantId),
    (spaces) =>
      spaces.map((space) => (space.spaceId === spaceId ? { ...space, ...patch } : space)),
    (next) => writeRegisteredSpaces(tenantId, next),
  );
}
