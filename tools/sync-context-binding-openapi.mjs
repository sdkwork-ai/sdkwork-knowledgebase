import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  jsonResponse,
  listEnvelope,
  listPaginationQueryParams,
  resourceEnvelope,
} from "./lib/openapi-envelope.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, "..");

const openApiPath = path.join(
  workspaceRoot,
  "sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json",
);
const routeManifestPath = path.join(
  workspaceRoot,
  "sdks/_route-manifests/app-api/sdkwork-routes-knowledgebase-app-api.route-manifest.json",
);

const problemResponse = {
  description: "Error",
  content: {
    "application/json": {
      schema: { $ref: "#/components/schemas/ProblemDetails" },
    },
    "application/problem+json": {
      schema: { $ref: "#/components/schemas/ProblemDetails" },
    },
  },
};

const security = [{ AuthToken: [], AccessToken: [] }];
const extensions = {
  "x-sdkwork-owner": "sdkwork-knowledgebase",
  "x-sdkwork-api-authority": "sdkwork-knowledgebase-app-api",
  "x-sdkwork-request-context": "WebRequestContext",
  "x-sdkwork-api-surface": "app-api",
  "x-sdkwork-source-route-crate": "sdkwork-routes-knowledgebase-app-api",
};

const int64PathParam = (name) => ({
  name,
  in: "path",
  required: true,
  schema: {
    type: "string",
    pattern: "^[0-9]+$",
    "x-sdkwork-int64-string": true,
  },
});

const schemas = {
  KnowledgeContextType: {
    type: "string",
    enum: [
      "chat_group",
      "organization",
      "circle",
      "channel",
      "team",
      "project",
    ],
  },
  KnowledgeAccessLevel: {
    type: "string",
    enum: ["reader", "writer"],
  },
  KnowledgeContextBindingStatus: {
    type: "string",
    enum: ["active", "deleted"],
  },
  KnowledgeSpaceContextBinding: {
    type: "object",
    required: [
      "id",
      "tenantId",
      "spaceId",
      "contextType",
      "contextId",
      "accessLevel",
      "status",
      "createdBy",
      "createdAt",
      "updatedAt",
    ],
    properties: {
      id: {
        type: "string",
        pattern: "^[0-9]+$",
        "x-sdkwork-int64-string": true,
      },
      tenantId: {
        type: "string",
        pattern: "^[0-9]+$",
        "x-sdkwork-int64-string": true,
      },
      spaceId: {
        type: "string",
        pattern: "^[0-9]+$",
        "x-sdkwork-int64-string": true,
      },
      contextType: { $ref: "#/components/schemas/KnowledgeContextType" },
      contextId: { type: "string" },
      contextName: { type: ["string", "null"] },
      accessLevel: { $ref: "#/components/schemas/KnowledgeAccessLevel" },
      status: { $ref: "#/components/schemas/KnowledgeContextBindingStatus" },
      createdBy: { type: "string" },
      createdAt: { type: "string", format: "date-time" },
      updatedAt: { type: "string", format: "date-time" },
    },
  },
  CreateKnowledgeSpaceContextBindingRequest: {
    type: "object",
    required: ["spaceId", "contextType", "contextId"],
    properties: {
      spaceId: {
        type: "string",
        pattern: "^[0-9]+$",
        "x-sdkwork-int64-string": true,
      },
      contextType: { $ref: "#/components/schemas/KnowledgeContextType" },
      contextId: { type: "string" },
      contextName: { type: ["string", "null"] },
      accessLevel: { $ref: "#/components/schemas/KnowledgeAccessLevel" },
    },
  },
  UpdateKnowledgeSpaceContextBindingRequest: {
    type: "object",
    properties: {
      contextName: { type: ["string", "null"] },
      accessLevel: { $ref: "#/components/schemas/KnowledgeAccessLevel" },
    },
  },
};

const newPaths = {
  "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings": {
    get: {
      operationId: "spaces.contextBindings.list",
      tags: ["knowledge"],
      summary: "List knowledge space context bindings",
      parameters: [int64PathParam("spaceId"), ...listPaginationQueryParams],
      security,
      responses: {
        200: jsonResponse(
          listEnvelope("#/components/schemas/KnowledgeSpaceContextBinding"),
        ),
        400: problemResponse,
        404: problemResponse,
      },
      ...extensions,
    },
    post: {
      operationId: "spaces.contextBindings.create",
      tags: ["knowledge"],
      summary: "Create a knowledge space context binding",
      parameters: [int64PathParam("spaceId")],
      security,
      requestBody: {
        required: true,
        content: {
          "application/json": {
            schema: {
              $ref: "#/components/schemas/CreateKnowledgeSpaceContextBindingRequest",
            },
          },
        },
      },
      responses: {
        200: jsonResponse(
          resourceEnvelope("#/components/schemas/KnowledgeSpaceContextBinding"),
        ),
        400: problemResponse,
        404: problemResponse,
        409: problemResponse,
      },
      ...extensions,
    },
  },
  "/app/v3/api/knowledge/context_bindings/{bindingId}": {
    get: {
      operationId: "contextBindings.retrieve",
      tags: ["knowledge"],
      summary: "Retrieve a knowledge space context binding",
      parameters: [int64PathParam("bindingId")],
      security,
      responses: {
        200: jsonResponse(
          resourceEnvelope("#/components/schemas/KnowledgeSpaceContextBinding"),
        ),
        404: problemResponse,
      },
      ...extensions,
    },
    patch: {
      operationId: "contextBindings.update",
      tags: ["knowledge"],
      summary: "Update a knowledge space context binding",
      parameters: [int64PathParam("bindingId")],
      security,
      requestBody: {
        required: true,
        content: {
          "application/json": {
            schema: {
              $ref: "#/components/schemas/UpdateKnowledgeSpaceContextBindingRequest",
            },
          },
        },
      },
      responses: {
        200: jsonResponse(
          resourceEnvelope("#/components/schemas/KnowledgeSpaceContextBinding"),
        ),
        400: problemResponse,
        404: problemResponse,
      },
      ...extensions,
    },
    delete: {
      operationId: "contextBindings.delete",
      tags: ["knowledge"],
      summary: "Delete a knowledge space context binding",
      parameters: [int64PathParam("bindingId")],
      security,
      responses: {
        204: { description: "No Content" },
        404: problemResponse,
      },
      ...extensions,
    },
  },
};

const newRoutes = [
  {
    method: "GET",
    path: "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings",
    operationId: "spaces.contextBindings.list",
  },
  {
    method: "POST",
    path: "/app/v3/api/knowledge/spaces/{spaceId}/context_bindings",
    operationId: "spaces.contextBindings.create",
  },
  {
    method: "GET",
    path: "/app/v3/api/knowledge/context_bindings/{bindingId}",
    operationId: "contextBindings.retrieve",
  },
  {
    method: "PATCH",
    path: "/app/v3/api/knowledge/context_bindings/{bindingId}",
    operationId: "contextBindings.update",
  },
  {
    method: "DELETE",
    path: "/app/v3/api/knowledge/context_bindings/{bindingId}",
    operationId: "contextBindings.delete",
  },
];

const spec = JSON.parse(await readFile(openApiPath, "utf8"));
Object.assign(spec.paths, newPaths);
Object.assign(spec.components.schemas, schemas);
await writeFile(openApiPath, `${JSON.stringify(spec, null, 2)}\n`, "utf8");

const manifest = JSON.parse(await readFile(routeManifestPath, "utf8"));
const existing = new Set(
  (manifest.routes || []).map((route) => `${route.method} ${route.path}`),
);
for (const route of newRoutes) {
  const key = `${route.method} ${route.path}`;
  if (existing.has(key)) {
    continue;
  }
  manifest.routes.push({
    method: route.method,
    path: route.path,
    operationId: route.operationId,
    tags: ["knowledge"],
    auth: { mode: "dual-token", required: true },
    handler: { module: "crate::routes", name: null },
    ownership: {
      owner: "sdkwork-knowledgebase",
      apiAuthority: "sdkwork-knowledgebase-app-api",
    },
    requestContext: "WebRequestContext",
    apiSurface: "app-api",
  });
}
manifest.routes.sort((left, right) => {
  const byMethod = left.method.localeCompare(right.method);
  if (byMethod !== 0) {
    return byMethod;
  }
  return left.path.localeCompare(right.path);
});
await writeFile(routeManifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

console.log("Synced context binding OpenAPI paths, schemas, and route manifest entries.");
