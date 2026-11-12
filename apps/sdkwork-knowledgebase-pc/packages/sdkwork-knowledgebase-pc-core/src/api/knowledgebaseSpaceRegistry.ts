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

/**
 * The session store every account-scoped guard reads.
 *
 * Exposed so the identity-change watch can subscribe to the same store the
 * registry keys are derived from; it never outlives the runtime that bound it.
 *
 * @returns the bound store, or `null` before the runtime binds one.
 */
export function getKnowledgebaseSessionStore(): SessionStore | null {
  return sessionStoreRef;
}

export function getKnowledgebaseTenantId(): string | undefined {
  return sessionStoreRef?.getSnapshot().context?.tenantId;
}

/**
 * Signed-in actor id of the bound session.
 *
 * A browser profile keeps its localStorage across sign-outs, so device storage
 * keyed by tenant alone replayed one account's knowledge bases to the next
 * account in the same tenant: the sidebar resolved those foreign space ids and
 * every surface answered `403 actor ... does not have access to space`,
 * including the agent-profile binding that the settings panel provisions from
 * the selected id. Account-scoped keys make each account resolve only the
 * spaces it registered, so a reader never has to configure permissions for
 * content it does not own.
 */
export function getKnowledgebaseActorId(): string | undefined {
  const context = sessionStoreRef?.getSnapshot().context;
  const actorId = (context?.userId ?? context?.actorId ?? '').trim();
  return actorId === '' ? undefined : actorId;
}

function registryStorageKey(tenantId: string, actorId?: string): string {
  return actorId === undefined
    ? `${REGISTRY_KEY_PREFIX}.${tenantId}`
    : `${REGISTRY_KEY_PREFIX}.${tenantId}.${actorId}`;
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

function parseRegisteredSpaces(raw: string | null): RegisteredKnowledgebaseSpace[] {
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
}

export function readRegisteredSpaces(
  tenantId: string,
  actorId: string | undefined = getKnowledgebaseActorId(),
): RegisteredKnowledgebaseSpace[] {
  if (typeof window === 'undefined') {
    return [];
  }

  const storageKey = registryStorageKey(tenantId, actorId);
  try {
    const stored = window.localStorage.getItem(storageKey);
    if (stored !== null) {
      return parseRegisteredSpaces(stored);
    }
    if (actorId === undefined) {
      return [];
    }
    // Adopt the tenant-only registry written before keys were account-scoped.
    // The caller validates every entry with a space read and drops the ids this
    // account cannot open, so adopting it costs one round, not lost spaces.
    const adopted = parseRegisteredSpaces(
      window.localStorage.getItem(registryStorageKey(tenantId)),
    );
    if (adopted.length > 0) {
      window.localStorage.setItem(storageKey, JSON.stringify(adopted));
    }
    return adopted;
  } catch {
    return [];
  }
}

export function writeRegisteredSpaces(
  tenantId: string,
  spaces: RegisteredKnowledgebaseSpace[],
  actorId: string | undefined = getKnowledgebaseActorId(),
): void {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(
    registryStorageKey(tenantId, actorId),
    JSON.stringify(spaces),
  );
}

export function upsertRegisteredSpace(
  tenantId: string,
  entry: RegisteredKnowledgebaseSpace,
): RegisteredKnowledgebaseSpace[] {
  // Capture the identity once: a read that recomputed it after an account
  // switch could merge one account's snapshot into the other's key.
  const actorId = getKnowledgebaseActorId();
  return withCrossTabStorageUpdate(
    registryStorageKey(tenantId, actorId),
    () => readRegisteredSpaces(tenantId, actorId),
    (spaces) => {
      const next = spaces.filter((space) => space.spaceId !== entry.spaceId);
      next.push(entry);
      return next;
    },
    (next) => writeRegisteredSpaces(tenantId, next, actorId),
  );
}

export function removeRegisteredSpace(tenantId: string, spaceId: string): RegisteredKnowledgebaseSpace[] {
  const actorId = getKnowledgebaseActorId();
  return withCrossTabStorageUpdate(
    registryStorageKey(tenantId, actorId),
    () => readRegisteredSpaces(tenantId, actorId),
    (spaces) => spaces.filter((space) => space.spaceId !== spaceId),
    (next) => writeRegisteredSpaces(tenantId, next, actorId),
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
  const actorId = getKnowledgebaseActorId();
  return withCrossTabStorageUpdate(
    registryStorageKey(tenantId, actorId),
    () => readRegisteredSpaces(tenantId, actorId),
    (spaces) =>
      spaces.map((space) => (space.spaceId === spaceId ? { ...space, ...patch } : space)),
    (next) => writeRegisteredSpaces(tenantId, next, actorId),
  );
}
