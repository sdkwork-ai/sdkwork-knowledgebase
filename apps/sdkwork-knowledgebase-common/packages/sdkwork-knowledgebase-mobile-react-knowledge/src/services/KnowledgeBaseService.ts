import { NotFoundError } from '@sdkwork/sdk-common';
import type {
  CreateKnowledgeDocumentRequest,
  KnowledgeDocument as ServerKnowledgeDocument,
  KnowledgeSpace,
  SdkworkKnowledgebaseAppClient,
  UpdateKnowledgeSpaceRequest,
} from '@sdkwork/knowledgebase-app-sdk';

export interface KnowledgeBase {
  id: string;
  name: string;
  description: string;
  icon: string;
  color?: string;
  isArchived?: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface KnowledgeDocument {
  id: string;
  kbId: string;
  title: string;
  content: string;
  category: string;
  author: string;
  createdAt: string;
  updatedAt: string;
  /** Server-side content pipeline state (`draft` | `ready` | ...), when known. */
  contentState?: string;
}

export class KnowledgeBaseCapabilityUnavailableError extends Error {
  constructor() {
    super("Knowledge Base is unavailable because its owner SDK is not composed.");
    this.name = "KnowledgeBaseCapabilityUnavailableError";
  }
}

/**
 * Runtime injected by the host application (sdkwork-im-h5) through
 * `configureKnowledgeBaseRuntime`. The generated Knowledgebase App SDK client
 * is constructed by the host bootstrap; this package never builds raw HTTP.
 */
export interface KnowledgeBaseRuntime {
  readonly client: SdkworkKnowledgebaseAppClient;
  /**
   * Resolves the local registry scope key (e.g. current user id). The app-api
   * has no end-user "list my spaces" endpoint, so the space list is a local
   * registry of spaces this device created or opened, verified against the
   * server on load — the same launch-driven model the PC client uses.
   */
  readonly resolveScopeKey?: () => string | undefined;
}

let runtime: KnowledgeBaseRuntime | null = null;

function requireRuntime(): KnowledgeBaseRuntime {
  if (!runtime) {
    throw new KnowledgeBaseCapabilityUnavailableError();
  }
  return runtime;
}

export function configureKnowledgeBaseRuntime(nextRuntime: KnowledgeBaseRuntime): void {
  runtime = nextRuntime;
}

export function resetKnowledgeBaseRuntime(): void {
  runtime = null;
}

/**
 * Local knowledge space registry (mirrors the PC `knowledgebaseSpaceRegistry`).
 * Stores display-only metadata (icon/color) plus device-local timestamps that
 * the server model does not carry.
 */
export interface RegisteredKnowledgebaseSpace {
  spaceId: string;
  /** Last known server name/description snapshot, kept for offline list rendering. */
  name?: string;
  description?: string;
  icon?: string;
  color?: string;
  createdAt: string;
  lastOpenedAt?: string;
}

const REGISTRY_KEY_PREFIX = 'sdkwork.knowledgebase.spaces.v1.h5';

function resolveScopeKey(): string {
  const scoped = runtime?.resolveScopeKey?.();
  if (typeof scoped === 'string' && scoped.trim().length > 0) {
    return scoped.trim();
  }
  return 'default';
}

function registryStorageKey(scopeKey: string): string {
  return `${REGISTRY_KEY_PREFIX}.${scopeKey}`;
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

export function readRegisteredSpaces(scopeKey = resolveScopeKey()): RegisteredKnowledgebaseSpace[] {
  if (typeof window === 'undefined') {
    return [];
  }
  try {
    const raw = window.localStorage.getItem(registryStorageKey(scopeKey));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw) as RegisteredKnowledgebaseSpace[];
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .map((space) => ({ ...space, spaceId: normalizeRegisteredSpaceId(space.spaceId) }))
      .filter((space) => space.spaceId.length > 0);
  } catch {
    return [];
  }
}

function writeRegisteredSpaces(scopeKey: string, spaces: RegisteredKnowledgebaseSpace[]): void {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(registryStorageKey(scopeKey), JSON.stringify(spaces));
}

export function upsertRegisteredSpace(
  scopeKey: string,
  entry: RegisteredKnowledgebaseSpace,
): RegisteredKnowledgebaseSpace[] {
  const spaces = readRegisteredSpaces(scopeKey);
  const next = spaces.filter((space) => space.spaceId !== entry.spaceId);
  next.push(entry);
  writeRegisteredSpaces(scopeKey, next);
  return next;
}

export function removeRegisteredSpace(scopeKey: string, spaceId: string): RegisteredKnowledgebaseSpace[] {
  const next = readRegisteredSpaces(scopeKey).filter((space) => space.spaceId !== spaceId);
  writeRegisteredSpaces(scopeKey, next);
  return next;
}

function createIdempotencyKey(): string {
  const randomUuid = globalThis.crypto?.randomUUID;
  if (!randomUuid) {
    throw new Error("Secure UUID generation is unavailable in this browser.");
  }
  return `kb-h5-doc-${randomUuid.call(globalThis.crypto)}`;
}

function mapServerSpace(
  space: KnowledgeSpace,
  registered?: RegisteredKnowledgebaseSpace,
): KnowledgeBase {
  const now = new Date().toISOString();
  return {
    id: space.id,
    name: space.name,
    description: space.description ?? '',
    icon: registered?.icon ?? '📚',
    color: registered?.color,
    isArchived: space.status === 'archived',
    createdAt: registered?.createdAt ?? now,
    updatedAt: registered?.lastOpenedAt ?? registered?.createdAt ?? now,
  };
}

function mapServerDocument(
  document: ServerKnowledgeDocument,
): KnowledgeDocument {
  return {
    id: document.id,
    kbId: document.spaceId,
    title: document.title,
    content: '',
    category: '',
    author: '',
    createdAt: '',
    updatedAt: '',
    contentState: document.contentState,
  };
}

const DOCUMENTS_PAGE_SIZE = 100;

export class KnowledgeBaseService {
  /** Lists knowledge bases from the local registry, verified against the server. */
  static async getKnowledgeBases(): Promise<KnowledgeBase[]> {
    const { client } = requireRuntime();
    const scopeKey = resolveScopeKey();
    const registered = readRegisteredSpaces(scopeKey);
    if (registered.length === 0) {
      return [];
    }

    const results = await Promise.allSettled(
      registered.map((entry) => client.knowledge.spaces.retrieve(entry.spaceId)),
    );

    const kept: RegisteredKnowledgebaseSpace[] = [];
    const knowledgeBases: KnowledgeBase[] = [];
    results.forEach((result, index) => {
      const entry = registered[index];
      if (result.status === 'fulfilled') {
        const space = result.value;
        if (space.status === 'deleted') {
          // Space was deleted elsewhere: prune it from the registry.
          return;
        }
        const refreshed = {
          ...entry,
          name: space.name,
          description: space.description ?? '',
        };
        kept.push(refreshed);
        knowledgeBases.push(mapServerSpace(space, refreshed));
      } else {
        if (result.reason instanceof NotFoundError) {
          // Space no longer exists: prune it from the registry.
          return;
        }
        // Transient/network errors keep the local entry so the list stays usable.
        kept.push(entry);
        knowledgeBases.push(mapServerSpace({
          id: entry.spaceId,
          uuid: '',
          name: entry.name ?? '',
          description: entry.description ?? null,
          status: 'active',
          okfBundleInitialized: false,
        }, entry));
      }
    });

    writeRegisteredSpaces(scopeKey, kept);
    return knowledgeBases;
  }

  static async getKnowledgeBase(id: string): Promise<KnowledgeBase | null> {
    const { client } = requireRuntime();
    const scopeKey = resolveScopeKey();
    try {
      const space = await client.knowledge.spaces.retrieve(id);
      if (space.status === 'deleted') {
        removeRegisteredSpace(scopeKey, id);
        return null;
      }
      const existing = readRegisteredSpaces(scopeKey).find((entry) => entry.spaceId === id);
      const entry = {
        spaceId: id,
        name: space.name,
        description: space.description ?? '',
        icon: existing?.icon,
        color: existing?.color,
        createdAt: existing?.createdAt ?? new Date().toISOString(),
        lastOpenedAt: new Date().toISOString(),
      };
      upsertRegisteredSpace(scopeKey, entry);
      return mapServerSpace(space, entry);
    } catch (error) {
      if (error instanceof NotFoundError) {
        removeRegisteredSpace(scopeKey, id);
        return null;
      }
      throw error;
    }
  }

  static async createKnowledgeBase(
    knowledgeBase: Omit<KnowledgeBase, "id" | "createdAt" | "updatedAt">,
  ): Promise<KnowledgeBase> {
    const { client } = requireRuntime();
    const scopeKey = resolveScopeKey();
    const space = await client.knowledge.spaces.create({
      name: knowledgeBase.name,
      description: knowledgeBase.description || null,
    });
    const now = new Date().toISOString();
    upsertRegisteredSpace(scopeKey, {
      spaceId: space.id,
      name: space.name,
      description: space.description ?? '',
      icon: knowledgeBase.icon,
      color: knowledgeBase.color,
      createdAt: now,
      lastOpenedAt: now,
    });
    return mapServerSpace(space, {
      spaceId: space.id,
      name: space.name,
      description: space.description ?? '',
      icon: knowledgeBase.icon,
      color: knowledgeBase.color,
      createdAt: now,
      lastOpenedAt: now,
    });
  }

  static async updateKnowledgeBase(
    id: string,
    updates: Partial<KnowledgeBase>,
  ): Promise<KnowledgeBase | null> {
    const { client } = requireRuntime();
    const scopeKey = resolveScopeKey();
    const body: UpdateKnowledgeSpaceRequest = {};
    if (updates.name !== undefined) {
      body.name = updates.name;
    }
    if (updates.description !== undefined) {
      body.description = updates.description;
    }
    try {
      const space = await client.knowledge.spaces.update(id, body);
      const registered = readRegisteredSpaces(scopeKey).find((entry) => entry.spaceId === id);
      const refreshed = registered
        ? {
            ...registered,
            name: space.name,
            description: space.description ?? '',
            lastOpenedAt: new Date().toISOString(),
          }
        : undefined;
      if (refreshed) {
        upsertRegisteredSpace(scopeKey, refreshed);
      }
      return mapServerSpace(space, refreshed);
    } catch (error) {
      if (error instanceof NotFoundError) {
        removeRegisteredSpace(scopeKey, id);
        return null;
      }
      throw error;
    }
  }

  static async deleteKnowledgeBase(id: string): Promise<void> {
    const { client } = requireRuntime();
    const scopeKey = resolveScopeKey();
    await client.knowledge.spaces.delete(id);
    removeRegisteredSpace(scopeKey, id);
  }

  static async deleteKnowledgeBases(ids: string[]): Promise<void> {
    for (const id of ids) {
      await this.deleteKnowledgeBase(id);
    }
  }

  static async getAllDocuments(): Promise<KnowledgeDocument[]> {
    requireRuntime();
    const registered = readRegisteredSpaces();
    const results = await Promise.allSettled(
      registered.map((entry) => this.getDocumentsByKbId(entry.spaceId)),
    );
    return results.flatMap((result) => (result.status === 'fulfilled' ? result.value : []));
  }

  static async getDocumentsByKbId(knowledgeBaseId: string): Promise<KnowledgeDocument[]> {
    const { client } = requireRuntime();
    const documents: KnowledgeDocument[] = [];
    let cursor: string | undefined;
    do {
      const page = await client.knowledge.documents.list({
        spaceId: knowledgeBaseId,
        cursor,
        pageSize: DOCUMENTS_PAGE_SIZE,
      });
      documents.push(...page.items.map(mapServerDocument));
      cursor = page.pageInfo.nextCursor ?? undefined;
    } while (cursor);
    return documents;
  }

  static async getDocument(id: string): Promise<KnowledgeDocument | null> {
    const { client } = requireRuntime();
    try {
      const document = await client.knowledge.documents.retrieve(id);
      let content = '';
      try {
        const documentContent = await client.knowledge.documents.content.list(id);
        content = documentContent.contentMarkdown;
      } catch {
        // Content may not be materialized yet (async ingest pipeline).
      }
      return { ...mapServerDocument(document), content };
    } catch (error) {
      if (error instanceof NotFoundError) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Creates the document metadata through `documents.create` and, when the form
   * carries markdown, pushes the content through the async `ingests.create`
   * pipeline. The content becomes readable once the ingest job succeeds.
   */
  static async createDocument(
    document: Omit<KnowledgeDocument, "id" | "createdAt" | "updatedAt">,
  ): Promise<KnowledgeDocument> {
    const { client } = requireRuntime();
    const created = await client.knowledge.documents.create({
      spaceId: document.kbId,
      title: document.title,
      mimeType: 'text/markdown',
    });
    if (document.content && document.content.trim().length > 0) {
      await client.knowledge.ingests.create({
        spaceId: document.kbId,
        title: document.title,
        payloadMarkdown: document.content,
        idempotencyKey: createIdempotencyKey(),
      });
    }
    return { ...mapServerDocument(created), content: document.content };
  }

  static async updateDocument(
    id: string,
    updates: Partial<KnowledgeDocument>,
  ): Promise<KnowledgeDocument | null> {
    const { client } = requireRuntime();
    try {
      const existing = await client.knowledge.documents.retrieve(id);
      const body: CreateKnowledgeDocumentRequest = {
        spaceId: existing.spaceId,
        title: updates.title !== undefined ? updates.title : existing.title,
      };
      const updated = await client.knowledge.documents.update(id, body);
      return mapServerDocument(updated);
    } catch (error) {
      if (error instanceof NotFoundError) {
        return null;
      }
      throw error;
    }
  }

  static async deleteDocument(id: string): Promise<void> {
    await requireRuntime().client.knowledge.documents.delete(id);
  }
}
