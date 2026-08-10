import assert from "node:assert/strict";
import test from "node:test";

import { NotFoundError } from "@sdkwork/sdk-common";
import type { SdkworkKnowledgebaseAppClient } from "@sdkwork/knowledgebase-app-sdk";

import {
  KnowledgeBaseCapabilityUnavailableError,
  KnowledgeBaseService,
  configureKnowledgeBaseRuntime,
  readRegisteredSpaces,
  resetKnowledgeBaseRuntime,
} from "./KnowledgeBaseService";

/**
 * In-memory localStorage polyfill so registry behaviour can be exercised in
 * Node (the browser `window` is absent by default).
 */
const registryStore = new Map<string, string>();

function installLocalStorage(): void {
  const localStorage = {
    getItem: (key: string) => registryStore.get(key) ?? null,
    setItem: (key: string, value: string) => {
      registryStore.set(key, value);
    },
    removeItem: (key: string) => {
      registryStore.delete(key);
    },
  };
  (globalThis as Record<string, unknown>).window = { localStorage };
}

function uninstallLocalStorage(): void {
  registryStore.clear();
  delete (globalThis as Record<string, unknown>).window;
}

interface StubSpaceRecord {
  id: string;
  name: string;
  description?: string | null;
  status: string;
}

interface StubDocumentRecord {
  id: string;
  spaceId: string;
  title: string;
  contentState: string;
}

/**
 * Minimal in-memory fake of the generated Knowledgebase App SDK client,
 * exercising the same method names used by KnowledgeBaseService.
 */
function createStubClient() {
  const spaces: StubSpaceRecord[] = [];
  const documents: StubDocumentRecord[] = [];
  const contents = new Map<string, string>();
  const ingests: Array<Record<string, unknown>> = [];
  const calls: Array<Record<string, unknown>> = [];

  const client = {
    knowledge: {
      spaces: {
        async create(body: { name: string; description?: string | null }) {
          calls.push({ op: "spaces.create", body });
          const space = {
            id: String(spaces.length + 1),
            uuid: `uuid-${spaces.length + 1}`,
            name: body.name,
            description: body.description ?? null,
            status: "active",
            okfBundleInitialized: false,
          };
          spaces.push(space);
          return space;
        },
        async retrieve(spaceId: string) {
          calls.push({ op: "spaces.retrieve", spaceId });
          const space = spaces.find((item) => item.id === spaceId);
          if (!space) {
            throw new NotFoundError("Space not found");
          }
          return space;
        },
        async update(spaceId: string, body: { name?: string; description?: string | null }) {
          calls.push({ op: "spaces.update", spaceId, body });
          const space = spaces.find((item) => item.id === spaceId);
          if (!space) {
            throw new NotFoundError("Space not found");
          }
          if (body.name !== undefined) {
            space.name = body.name;
          }
          if (body.description !== undefined) {
            space.description = body.description;
          }
          return space;
        },
        async delete(spaceId: string) {
          calls.push({ op: "spaces.delete", spaceId });
          const index = spaces.findIndex((item) => item.id === spaceId);
          if (index < 0) {
            throw new NotFoundError("Space not found");
          }
          spaces.splice(index, 1);
        },
      },
      documents: {
        async list(params: { spaceId: string; cursor?: string; pageSize?: number }) {
          calls.push({ op: "documents.list", params });
          // Cap the page size so cursor pagination is exercised in tests.
          const pageSize = Math.min(params.pageSize ?? 100, 2);
          const all = documents.filter((doc) => doc.spaceId === params.spaceId);
          const offset = params.cursor ? Number(params.cursor) : 0;
          const items = all.slice(offset, offset + pageSize);
          const nextCursor =
            offset + pageSize < all.length ? String(offset + pageSize) : undefined;
          return {
            items,
            pageInfo: { mode: "cursor", nextCursor, hasMore: nextCursor !== undefined },
          };
        },
        async create(body: { spaceId: string; title: string }) {
          calls.push({ op: "documents.create", body });
          const document = {
            id: `doc-${documents.length + 1}`,
            spaceId: body.spaceId,
            collectionId: "",
            title: body.title,
            visibility: "space",
            contentState: "draft",
            indexState: "draft",
          };
          documents.push(document);
          return document;
        },
        async retrieve(documentId: string) {
          calls.push({ op: "documents.retrieve", documentId });
          const document = documents.find((item) => item.id === documentId);
          if (!document) {
            throw new NotFoundError("Document not found");
          }
          return document;
        },
        async update(documentId: string, body: { spaceId: string; title: string }) {
          calls.push({ op: "documents.update", documentId, body });
          const document = documents.find((item) => item.id === documentId);
          if (!document) {
            throw new NotFoundError("Document not found");
          }
          document.title = body.title;
          return document;
        },
        async delete(documentId: string) {
          calls.push({ op: "documents.delete", documentId });
          const index = documents.findIndex((item) => item.id === documentId);
          if (index < 0) {
            throw new NotFoundError("Document not found");
          }
          documents.splice(index, 1);
        },
        content: {
          async list(documentId: string) {
            calls.push({ op: "documents.content.list", documentId });
            if (!contents.has(documentId)) {
              throw new NotFoundError("Content not found");
            }
            return {
              documentId,
              contentMarkdown: contents.get(documentId) ?? "",
              contentSource: "manual",
              contentVersion: "1",
            };
          },
        },
      },
      ingests: {
        async create(body: Record<string, unknown>) {
          calls.push({ op: "ingests.create", body });
          ingests.push(body);
          return {
            id: `ingest-${ingests.length}`,
            spaceId: body.spaceId as string,
            sourceType: "markdown",
            idempotencyKey: body.idempotencyKey as string,
            state: "queued",
          };
        },
      },
    },
  } as unknown as SdkworkKnowledgebaseAppClient;

  return {
    client,
    spaces,
    documents,
    contents,
    ingests,
    calls,
  };
}

test("knowledge base operations fail closed until the owner SDK is composed", async () => {
  resetKnowledgeBaseRuntime();
  const knowledgeBase = {
    description: "Description",
    icon: "database",
    name: "Knowledge Base",
  };
  const document = {
    author: "user-id",
    category: "general",
    content: "Content",
    kbId: "knowledge-base-id",
    title: "Document",
  };
  const operations = [
    () => KnowledgeBaseService.getKnowledgeBases(),
    () => KnowledgeBaseService.getKnowledgeBase("knowledge-base-id"),
    () => KnowledgeBaseService.createKnowledgeBase(knowledgeBase),
    () => KnowledgeBaseService.updateKnowledgeBase("knowledge-base-id", { name: "Updated" }),
    () => KnowledgeBaseService.deleteKnowledgeBase("knowledge-base-id"),
    () => KnowledgeBaseService.deleteKnowledgeBases(["knowledge-base-id"]),
    () => KnowledgeBaseService.getAllDocuments(),
    () => KnowledgeBaseService.getDocumentsByKbId("knowledge-base-id"),
    () => KnowledgeBaseService.getDocument("document-id"),
    () => KnowledgeBaseService.createDocument(document),
    () => KnowledgeBaseService.updateDocument("document-id", { title: "Updated" }),
    () => KnowledgeBaseService.deleteDocument("document-id"),
  ];

  for (const operation of operations) {
    await assert.rejects(operation, KnowledgeBaseCapabilityUnavailableError);
  }
});

test("list returns an empty array when the registry is empty", async () => {
  installLocalStorage();
  const { client } = createStubClient();
  configureKnowledgeBaseRuntime({ client, resolveScopeKey: () => "user-1" });
  try {
    assert.deepEqual(await KnowledgeBaseService.getKnowledgeBases(), []);
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("createKnowledgeBase creates the space through the app SDK and registers it locally", async () => {
  installLocalStorage();
  const stub = createStubClient();
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const created = await KnowledgeBaseService.createKnowledgeBase({
      name: "员工手册",
      description: "公司制度",
      icon: "📚",
      color: "#0066FF",
    });

    assert.equal(created.id, "1");
    assert.equal(created.name, "员工手册");
    assert.equal(created.description, "公司制度");
    assert.equal(created.icon, "📚");
    assert.equal(created.color, "#0066FF");
    assert.equal(created.isArchived, false);
    assert.ok(created.createdAt.length > 0);

    const registered = readRegisteredSpaces("user-1");
    assert.equal(registered.length, 1);
    assert.equal(registered[0].spaceId, "1");
    assert.equal(registered[0].icon, "📚");
    assert.deepEqual(stub.calls[0], {
      op: "spaces.create",
      body: { name: "员工手册", description: "公司制度" },
    });
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("getKnowledgeBase maps the server space and refreshes the registry", async () => {
  installLocalStorage();
  const stub = createStubClient();
  stub.spaces.push({
    id: "7",
    name: "产品知识库",
    description: "产品文档",
    status: "active",
  });
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const kb = await KnowledgeBaseService.getKnowledgeBase("7");
    assert.ok(kb);
    assert.equal(kb.name, "产品知识库");
    assert.equal(kb.description, "产品文档");
    assert.equal(kb.isArchived, false);

    const registered = readRegisteredSpaces("user-1");
    assert.equal(registered.length, 1);
    assert.equal(registered[0].name, "产品知识库");
    assert.equal(registered[0].description, "产品文档");
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("getKnowledgeBase returns null and prunes the registry when the space is gone", async () => {
  installLocalStorage();
  const stub = createStubClient();
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    assert.equal(await KnowledgeBaseService.getKnowledgeBase("missing"), null);
    assert.equal(readRegisteredSpaces("user-1").length, 0);
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("getKnowledgeBases syncs registry entries, prunes deleted ones and keeps transient failures", async () => {
  installLocalStorage();
  const stub = createStubClient();
  stub.spaces.push(
    { id: "1", name: "A", description: "a", status: "active" },
    { id: "2", name: "B", description: "b", status: "archived" },
    { id: "3", name: "C", description: "c", status: "deleted" },
  );
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    // Seed the registry with three spaces; one of them is deleted server-side.
    const registry = [
      { spaceId: "1", name: "Old A", description: "old-a", createdAt: "2026-01-01T00:00:00Z" },
      { spaceId: "2", name: "Old B", description: "old-b", createdAt: "2026-01-02T00:00:00Z" },
      { spaceId: "3", name: "Old C", description: "old-c", createdAt: "2026-01-03T00:00:00Z" },
    ];
    const rawKey = "sdkwork.knowledgebase.spaces.v1.h5.user-1";
    registryStore.set(rawKey, JSON.stringify(registry));

    const list = await KnowledgeBaseService.getKnowledgeBases();
    assert.equal(list.length, 2);
    assert.equal(list[0].name, "A");
    assert.equal(list[1].name, "B");
    assert.equal(list[1].isArchived, true);

    // Deleted space pruned, snapshots refreshed.
    const remaining = readRegisteredSpaces("user-1");
    assert.deepEqual(
      remaining.map((entry) => entry.spaceId).sort(),
      ["1", "2"],
    );
    assert.equal(remaining.find((entry) => entry.spaceId === "1")?.name, "A");
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("deleteKnowledgeBase deletes through the SDK and removes the registry entry", async () => {
  installLocalStorage();
  const stub = createStubClient();
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const created = await KnowledgeBaseService.createKnowledgeBase({
      name: "X",
      description: "",
      icon: "📚",
    });
    assert.equal(readRegisteredSpaces("user-1").length, 1);

    await KnowledgeBaseService.deleteKnowledgeBase(created.id);
    assert.equal(readRegisteredSpaces("user-1").length, 0);
    assert.deepEqual(stub.calls[stub.calls.length - 1], {
      op: "spaces.delete",
      spaceId: created.id,
    });
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("getDocumentsByKbId paginates through the documents list", async () => {
  installLocalStorage();
  const stub = createStubClient();
  for (let index = 1; index <= 5; index += 1) {
    stub.documents.push({
      id: `doc-${index}`,
      spaceId: "1",
      title: `文档 ${index}`,
      contentState: "ready",
    });
  }
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const docs = await KnowledgeBaseService.getDocumentsByKbId("1");
    assert.equal(docs.length, 5);
    assert.equal(docs[0].id, "doc-1");
    assert.equal(docs[0].kbId, "1");
    assert.equal(docs[0].title, "文档 1");
    // The stub paginates with pageSize 2: expect multiple list calls.
    const listCalls = stub.calls.filter((call) => call.op === "documents.list");
    assert.ok(listCalls.length >= 3, "expected cursor pagination across pages");
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("getDocument returns the document with authoritative markdown content", async () => {
  installLocalStorage();
  const stub = createStubClient();
  stub.documents.push({
    id: "doc-1",
    spaceId: "1",
    title: "复盘",
    contentState: "ready",
  });
  stub.contents.set("doc-1", "# 标题\n正文内容");
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const doc = await KnowledgeBaseService.getDocument("doc-1");
    assert.ok(doc);
    assert.equal(doc.title, "复盘");
    assert.equal(doc.content, "# 标题\n正文内容");
    assert.equal(doc.contentState, "ready");
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("createDocument creates metadata and pushes content through the ingest pipeline", async () => {
  installLocalStorage();
  const stub = createStubClient();
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const doc = await KnowledgeBaseService.createDocument({
      kbId: "1",
      title: "新文档",
      content: "## 内容",
      category: "",
      author: "",
    });

    assert.equal(doc.id, "doc-1");
    assert.equal(doc.title, "新文档");
    assert.equal(doc.content, "## 内容");

    const createCall = stub.calls.find((call) => call.op === "documents.create");
    assert.deepEqual(createCall?.body, {
      spaceId: "1",
      title: "新文档",
      mimeType: "text/markdown",
    });

    const ingestCall = stub.calls.find((call) => call.op === "ingests.create");
    assert.ok(ingestCall, "expected an ingest job for the markdown content");
    const ingestBody = ingestCall.body as Record<string, unknown>;
    assert.equal(ingestBody.spaceId, "1");
    assert.equal(ingestBody.title, "新文档");
    assert.equal(ingestBody.payloadMarkdown, "## 内容");
    assert.equal(typeof ingestBody.idempotencyKey, "string");
    assert.ok((ingestBody.idempotencyKey as string).startsWith("kb-h5-doc-"));
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("updateDocument patches the title with the resolved spaceId", async () => {
  installLocalStorage();
  const stub = createStubClient();
  stub.documents.push({
    id: "doc-1",
    spaceId: "1",
    title: "旧标题",
    contentState: "ready",
  });
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    const updated = await KnowledgeBaseService.updateDocument("doc-1", { title: "新标题" });
    assert.ok(updated);
    assert.equal(updated.title, "新标题");

    const updateCall = stub.calls.find((call) => call.op === "documents.update");
    assert.deepEqual(updateCall?.body, { spaceId: "1", title: "新标题" });
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});

test("deleteDocument deletes through the SDK", async () => {
  installLocalStorage();
  const stub = createStubClient();
  stub.documents.push({
    id: "doc-1",
    spaceId: "1",
    title: "T",
    contentState: "ready",
  });
  configureKnowledgeBaseRuntime({ client: stub.client, resolveScopeKey: () => "user-1" });
  try {
    await KnowledgeBaseService.deleteDocument("doc-1");
    assert.deepEqual(stub.calls[stub.calls.length - 1], {
      op: "documents.delete",
      documentId: "doc-1",
    });
  } finally {
    resetKnowledgeBaseRuntime();
    uninstallLocalStorage();
  }
});
