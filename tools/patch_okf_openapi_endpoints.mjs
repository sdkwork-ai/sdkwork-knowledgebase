import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const targets = [
  "sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json",
  "sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json",
];

const backendImportPath = "/backend/v3/api/knowledge/okf/imports";
const backendImportOperation = {
  post: {
    operationId: "okf.bundle.import.create",
    tags: ["knowledge"],
    requestBody: {
      required: true,
      content: {
        "application/json": {
          schema: { $ref: "#/components/schemas/OkfBundleImportRequest" },
        },
      },
    },
    responses: {
      "201": {
        description: "Created",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/OkfBundleImportResult" },
          },
        },
      },
      "400": {
        description: "Error",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
          "application/problem+json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
        },
      },
      "404": {
        description: "Error",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
          "application/problem+json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
        },
      },
    },
    security: [{ AuthToken: [], AccessToken: [] }],
    "x-sdkwork-owner": "sdkwork-knowledgebase",
    "x-sdkwork-api-authority": "sdkwork-knowledgebase-backend-api",
    description: "Import an OKF bundle from drive staging",
    "x-sdkwork-request-context": "WebRequestContext",
    "x-sdkwork-api-surface": "backend-api",
    "x-sdkwork-source-route-crate": "sdkwork-routes-knowledgebase-backend-api",
  },
};

const appUpsertPath = "/app/v3/api/knowledge/okf/concepts/upsert";
const appUpsertOperation = {
  put: {
    operationId: "okf.concepts.update",
    tags: ["knowledge"],
    requestBody: {
      required: true,
      content: {
        "application/json": {
          schema: { $ref: "#/components/schemas/OkfConceptUpsertRequest" },
        },
      },
    },
    responses: {
      "200": {
        description: "OK",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/OkfConceptSummary" },
          },
        },
      },
      "400": {
        description: "Error",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
          "application/problem+json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
        },
      },
      "404": {
        description: "Error",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
          "application/problem+json": {
            schema: { $ref: "#/components/schemas/ProblemDetail" },
          },
        },
      },
    },
    security: [{ AuthToken: [], AccessToken: [] }],
    "x-sdkwork-owner": "sdkwork-knowledgebase",
    "x-sdkwork-api-authority": "sdkwork-knowledgebase-app-api",
    description: "Upsert an OKF concept revision",
    "x-sdkwork-request-context": "WebRequestContext",
    "x-sdkwork-api-surface": "app-api",
    "x-sdkwork-source-route-crate": "sdkwork-routes-knowledgebase-app-api",
  },
};

const backendSchemas = {
  OkfBundleImportRequest: {
    type: "object",
    required: ["spaceId", "importType"],
    properties: {
      spaceId: { type: "integer", format: "uint64" },
      importType: { type: "string" },
      importId: { type: "string" },
    },
  },
  OkfBundleImportResult: {
    type: "object",
    required: ["importedConceptCount", "skippedFiles"],
    properties: {
      importedConceptCount: { type: "integer", format: "uint32" },
      skippedFiles: { type: "array", items: { type: "string" } },
    },
  },
};

const appSurface = {
  owner: "sdkwork-knowledgebase",
  authority: "sdkwork-knowledgebase-app-api",
  surface: "app-api",
  crate: "sdkwork-routes-knowledgebase-app-api",
};

function appOperation(method, operationId, description, extra = {}) {
  return {
    [method]: {
      operationId,
      tags: ["knowledge"],
      security: [{ AuthToken: [], AccessToken: [] }],
      "x-sdkwork-owner": appSurface.owner,
      "x-sdkwork-api-authority": appSurface.authority,
      description,
      "x-sdkwork-request-context": "WebRequestContext",
      "x-sdkwork-api-surface": appSurface.surface,
      "x-sdkwork-source-route-crate": appSurface.crate,
      ...extra,
    },
  };
}

const problemResponses = {
  "400": {
    description: "Error",
    content: {
      "application/json": { schema: { $ref: "#/components/schemas/ProblemDetail" } },
      "application/problem+json": { schema: { $ref: "#/components/schemas/ProblemDetail" } },
    },
  },
  "404": {
    description: "Error",
    content: {
      "application/json": { schema: { $ref: "#/components/schemas/ProblemDetail" } },
      "application/problem+json": { schema: { $ref: "#/components/schemas/ProblemDetail" } },
    },
  },
};

const appBundlePaths = {
  "/app/v3/api/knowledge/okf/exports": appOperation("post", "okf.bundle.export.create", "Create an OKF bundle export", {
    requestBody: {
      required: true,
      content: {
        "application/json": {
          schema: { $ref: "#/components/schemas/OkfBundleExportRequest" },
        },
      },
    },
    responses: {
      "201": {
        description: "Created",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/KnowledgeOkfBundleFile" },
          },
        },
      },
      ...problemResponses,
    },
  }),
  "/app/v3/api/knowledge/okf/exports/{exportId}": appOperation(
    "get",
    "okf.bundle.export.retrieve",
    "Retrieve an OKF bundle export",
    {
      parameters: [
        {
          name: "exportId",
          in: "path",
          required: true,
          schema: { type: "integer", format: "uint64" },
        },
      ],
      responses: {
        "200": {
          description: "OK",
          content: {
            "application/json": {
              schema: { $ref: "#/components/schemas/KnowledgeOkfBundleFile" },
            },
          },
        },
        ...problemResponses,
      },
    },
  ),
  "/app/v3/api/knowledge/okf/imports": appOperation("post", "okf.bundle.import.create", "Import an OKF bundle from drive staging", {
    requestBody: {
      required: true,
      content: {
        "application/json": {
          schema: { $ref: "#/components/schemas/OkfBundleImportRequest" },
        },
      },
    },
    responses: {
      "201": {
        description: "Created",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/OkfBundleImportResult" },
          },
        },
      },
      ...problemResponses,
    },
  }),
  "/app/v3/api/knowledge/okf/lint_runs": appOperation("post", "okf.lintRuns.create", "Create an OKF bundle lint run", {
    requestBody: {
      required: true,
      content: {
        "application/json": {
          schema: { $ref: "#/components/schemas/OkfQualityRunRequest" },
        },
      },
    },
    responses: {
      "201": {
        description: "Created",
        content: {
          "application/json": {
            schema: { $ref: "#/components/schemas/OkfQualityRun" },
          },
        },
      },
      ...problemResponses,
    },
  }),
};

const appSchemas = {
  OkfConceptUpsertRequest: {
    type: "object",
    required: ["spaceId", "conceptId", "markdown", "actor", "publish"],
    properties: {
      spaceId: { type: "integer", format: "uint64" },
      conceptId: { type: "string" },
      markdown: { type: "string" },
      actor: { type: "string" },
      publish: { type: "boolean" },
    },
  },
  OkfBundleImportRequest: {
    type: "object",
    required: ["spaceId", "importType"],
    properties: {
      spaceId: { type: "integer", format: "uint64" },
      importType: { type: "string" },
      importId: { type: "string" },
    },
  },
  OkfBundleExportRequest: {
    type: "object",
    required: ["spaceId", "exportType"],
    properties: {
      spaceId: { type: "integer", format: "uint64" },
      exportType: { type: "string" },
      stageForImport: { type: "boolean", default: false },
      importId: { type: "string" },
    },
  },
  OkfBundleImportResult: {
    type: "object",
    required: ["importedConceptCount", "skippedFiles"],
    properties: {
      importedConceptCount: { type: "integer", format: "uint32" },
      skippedFiles: { type: "array", items: { type: "string" } },
    },
  },
  OkfConceptSummaryList: okfListDataSchema(
    "#/components/schemas/OkfConceptSummary",
    "One bounded cursor page of published OKF concept summaries.",
  ),
  KnowledgeOkfConceptRevisionList: okfListDataSchema(
    "#/components/schemas/KnowledgeOkfConceptRevision",
    "One bounded cursor page of OKF concept revisions.",
  ),
};

function okfListDataSchema(itemSchemaRef, description) {
  return {
    type: "object",
    additionalProperties: false,
    description,
    required: ["items", "pageInfo"],
    properties: {
      items: {
        type: "array",
        items: { $ref: itemSchemaRef },
      },
      pageInfo: { $ref: "#/components/schemas/PageInfo" },
    },
  };
}

function okfListResponseSchema(dataSchemaRef) {
  return {
    allOf: [
      { $ref: "#/components/schemas/SdkWorkApiResponse" },
      {
        type: "object",
        required: ["data"],
        properties: {
          data: { $ref: dataSchemaRef },
        },
      },
    ],
  };
}

function cursorParameter() {
  return {
    name: "cursor",
    in: "query",
    required: false,
    schema: { type: "string", maxLength: 512 },
  };
}

function pageSizeParameter() {
  return {
    name: "page_size",
    in: "query",
    required: false,
    schema: {
      type: "integer",
      format: "int32",
      minimum: 1,
      maximum: 200,
      default: 20,
    },
  };
}

const backendCandidatesPath = "/backend/v3/api/knowledge/okf/candidates";

for (const relativePath of targets) {
  const filePath = path.join(root, relativePath);
  const spec = JSON.parse(await readFile(filePath, "utf8"));
  const isBackend = relativePath.includes("backend-api");
  const isApp = relativePath.includes("app-api");

  if (isBackend) {
    spec.paths[backendImportPath] = backendImportOperation;
    Object.assign(spec.components.schemas, backendSchemas);
    const bundleFile = spec.components.schemas.KnowledgeOkfBundleFile;
    if (bundleFile?.properties) {
      bundleFile.properties.stagedImportRoot = { type: "string" };
      bundleFile.properties.importId = { type: "string" };
    }
    const exportRequest = spec.components.schemas.OkfBundleExportRequest;
    if (exportRequest?.properties) {
      exportRequest.properties.stageForImport = { type: "boolean", default: false };
      exportRequest.properties.importId = { type: "string" };
    }
    if (spec.paths[backendCandidatesPath]?.get) {
      spec.paths[backendCandidatesPath].get.parameters = [
        {
          name: "spaceId",
          in: "query",
          required: true,
          schema: { type: "integer", format: "uint64" },
        },
      ];
    }
  }

  if (isApp) {
    spec.paths[appUpsertPath] = appUpsertOperation;
    Object.assign(spec.paths, appBundlePaths);
    Object.assign(spec.components.schemas, appSchemas);
    const appConceptsListPath = "/app/v3/api/knowledge/okf/concepts";
    if (spec.paths[appConceptsListPath]?.get) {
      spec.paths[appConceptsListPath].get.parameters = [
        {
          name: "spaceId",
          in: "query",
          required: true,
          schema: {
            type: "string",
            format: "uint64",
            pattern: "^[0-9]+$",
            "x-sdkwork-int64-string": true,
          },
        },
        cursorParameter(),
        pageSizeParameter(),
      ];
      spec.paths[appConceptsListPath].get.responses["200"].content[
        "application/json"
      ].schema = okfListResponseSchema("#/components/schemas/OkfConceptSummaryList");
    }
    const appRevisionListPath =
      "/app/v3/api/knowledge/okf/concepts/{conceptId}/revisions";
    if (spec.paths[appRevisionListPath]?.get) {
      const conceptIdParameter = spec.paths[appRevisionListPath].get.parameters?.find(
        (parameter) => parameter.name === "conceptId" && parameter.in === "path",
      );
      if (!conceptIdParameter) {
        throw new Error(`Missing conceptId path parameter: ${appRevisionListPath}`);
      }
      spec.paths[appRevisionListPath].get.parameters = [
        conceptIdParameter,
        cursorParameter(),
        pageSizeParameter(),
      ];
      spec.paths[appRevisionListPath].get.responses["200"].content[
        "application/json"
      ].schema = okfListResponseSchema(
        "#/components/schemas/KnowledgeOkfConceptRevisionList",
      );
    }
    const bundleFile = spec.components.schemas.KnowledgeOkfBundleFile;
    if (bundleFile?.properties) {
      bundleFile.properties.stagedImportRoot = { type: "string" };
      bundleFile.properties.importId = { type: "string" };
    }
    const exportRequest = spec.components.schemas.OkfBundleExportRequest;
    if (exportRequest?.properties) {
      exportRequest.properties.stageForImport = { type: "boolean", default: false };
      exportRequest.properties.importId = { type: "string" };
    }
    const citation = spec.components.schemas.KnowledgeAgentChatCitation;
    if (citation?.properties?.conceptId) {
      citation.properties.conceptId = {
        anyOf: [{ type: "string" }, { type: "null" }],
      };
    }
  }

  await writeFile(filePath, `${JSON.stringify(spec, null, 2)}\n`, "utf8");
  console.log(`Patched ${relativePath}`);
}
