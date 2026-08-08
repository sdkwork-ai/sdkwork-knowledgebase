import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, "..");
const sdksRoot = path.resolve(workspaceRoot, "sdks");
const owner = "sdkwork-knowledgebase";
const standardVersion = "2026-06-06";
const checkOnly = process.argv.includes("--check");
const pendingChanges = [];

const httpMethods = new Set(["get", "put", "post", "delete", "options", "head", "patch", "trace"]);
const forbiddenPaginationQueryAliases = new Set(["limit", "page_no", "pageNo", "per_page", "size"]);
const currentTenantRequestSchemas = new Set([
  "KnowledgeRetrievalRequest",
  "KnowledgeContextPackRequest",
  "KnowledgeAgentChatRequest",
  "KnowledgeAgentBindingRequest",
  "KnowledgeAgentProfileRequest",
  "KnowledgeIndexRequest",
  "KnowledgeRetrievalProfileRequest",
]);

const appbasePackages = {
  typescript: "@sdkwork/iam-app-sdk",
  rust: "sdkwork-iam-app-sdk",
  java: "com.sdkwork:sdkwork-iam-app-sdk",
  python: "sdkwork-iam-app-sdk",
  go: "github.com/sdkwork/sdkwork-iam-app-sdk",
};

const appbaseBackendPackages = {
  typescript: "@sdkwork/iam-backend-sdk",
  rust: "sdkwork-iam-backend-sdk",
  java: "com.sdkwork:sdkwork-iam-backend-sdk",
  python: "sdkwork-iam-backend-sdk",
  go: "github.com/sdkwork/sdkwork-iam-backend-sdk",
};

const driveAppPackages = {
  typescript: "@sdkwork/drive-app-sdk",
  rust: "sdkwork-drive-app-sdk",
  java: "com.sdkwork:sdkwork-drive-app-sdk",
  python: "sdkwork-drive-app-sdk",
  go: "github.com/sdkwork/sdkwork-drive-app-sdk",
};

const driveBackendPackages = {
  typescript: "@sdkwork/drive-backend-sdk",
  rust: "sdkwork-drive-backend-sdk",
  java: "com.sdkwork:sdkwork-drive-backend-sdk",
  python: "sdkwork-drive-backend-sdk",
  go: "github.com/sdkwork/sdkwork-drive-backend-sdk",
};

const memoryAppPackages = {
  typescript: "@sdkwork/memory-app-sdk",
  rust: "sdkwork-memory-app-sdk",
  java: "com.sdkwork:sdkwork-memory-app-sdk",
  python: "sdkwork-memory-app-sdk",
  go: "github.com/sdkwork/sdkwork-memory-app-sdk",
};

const memoryBackendPackages = {
  typescript: "@sdkwork/memory-backend-sdk",
  rust: "sdkwork-memory-backend-sdk",
  java: "com.sdkwork:sdkwork-memory-backend-sdk",
  python: "sdkwork-memory-backend-sdk",
  go: "github.com/sdkwork/sdkwork-memory-backend-sdk",
};

const families = [
  {
    root: "sdkwork-knowledgebase-sdk",
    title: "SDKWork Knowledgebase Open API SDK",
    apiVersion: "0.1.0",
    authority: "sdkwork-knowledgebase-open-api",
    sdkTarget: "open",
    apiPrefix: "/knowledge/v3/api",
    schemaUrl: "/knowledge/v3/openapi.json",
    input: "openapi/knowledgebase-open-api.openapi.json",
    packageName: "@sdkwork/knowledgebase-sdk",
    generatedPath: "sdkwork-knowledgebase-sdk-typescript/generated/server-openapi",
    generatedWorkspace: "sdkwork-knowledgebase-sdk-typescript",
    primaryClient: "SdkworkKnowledgebaseClient",
    dependencies: [],
    materializeFrom: {
      sourceFamilyRoot: "sdkwork-knowledgebase-app-sdk",
      sourceInput: "openapi/knowledgebase-app-api.openapi.json",
      operations: [
        openOperation("post", "/app/v3/api/knowledge/retrievals", "/knowledge/v3/api/retrievals"),
        openOperation("get", "/app/v3/api/knowledge/retrievals/{retrievalId}", "/knowledge/v3/api/retrievals/{retrievalId}"),
        openOperation("post", "/app/v3/api/knowledge/context_packs", "/knowledge/v3/api/context_packs"),
        openOperation("post", "/app/v3/api/knowledge/ingests", "/knowledge/v3/api/ingests"),
        openOperation("get", "/app/v3/api/knowledge/ingests/{ingestId}", "/knowledge/v3/api/ingests/{ingestId}"),
        openOperation("get", "/app/v3/api/knowledge/documents", "/knowledge/v3/api/documents"),
        openOperation("get", "/app/v3/api/knowledge/documents/{documentId}", "/knowledge/v3/api/documents/{documentId}"),
        openOperation("get", "/app/v3/api/knowledge/spaces/{spaceId}/browser", "/knowledge/v3/api/spaces/{spaceId}/browser"),
      ],
    },
    forbiddenPathPrefixes: [
      "/app/v3/api/",
      "/backend/v3/api/",
      "/mem/v3/api/",
      "/open/v3/api/drive/",
    ],
  },
  {
    root: "sdkwork-knowledgebase-app-sdk",
    title: "SDKWork Knowledgebase App API SDK",
    apiVersion: "0.1.0",
    authority: "sdkwork-knowledgebase-app-api",
    sdkTarget: "app",
    apiPrefix: "/app/v3/api",
    schemaUrl: "/app/v3/openapi.json",
    input: "openapi/knowledgebase-app-api.openapi.json",
    packageName: "@sdkwork/knowledgebase-app-sdk",
    composedPackagePath: "sdkwork-knowledgebase-app-sdk-typescript",
    composedFactory: "createKnowledgebaseAppClient",
    generatedPath: "sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi",
    generatedWorkspace: "sdkwork-knowledgebase-app-sdk-typescript",
    primaryClient: "SdkworkKnowledgebaseAppClient",
    dependencies: [
      dependency("sdkwork-iam-app-sdk", "appbase-identity-and-session-capability", "/app/v3/api", "sdkwork-iam-app-api", appbasePackages),
      dependency("sdkwork-drive-app-sdk", "drive-file-and-media-capability", "/app/v3/api", "sdkwork-drive.app", driveAppPackages),
      dependency("sdkwork-memory-app-sdk", "memory-context-capability", "/app/v3/api", "sdkwork-memory.app", memoryAppPackages),
    ],
    forbiddenPathPrefixes: [
      "/app/v3/api/auth/",
      "/app/v3/api/iam/",
      "/app/v3/api/open_platform/",
      "/app/v3/api/system/iam/",
      "/app/v3/api/drive/",
      "/app/v3/api/memory/",
      "/mem/v3/api/",
    ],
  },
  {
    root: "sdkwork-knowledgebase-backend-sdk",
    title: "SDKWork Knowledgebase Backend API SDK",
    apiVersion: "0.1.0",
    authority: "sdkwork-knowledgebase-backend-api",
    sdkTarget: "backend",
    apiPrefix: "/backend/v3/api",
    schemaUrl: "/backend/v3/openapi.json",
    input: "openapi/knowledgebase-backend-api.openapi.json",
    packageName: "@sdkwork/knowledgebase-backend-sdk",
    generatedPath: "sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi",
    generatedWorkspace: "sdkwork-knowledgebase-backend-sdk-typescript",
    primaryClient: "SdkworkKnowledgebaseBackendClient",
    additionalLanguageTargets: [
      {
        language: "rust",
        workspace: "sdkwork-knowledgebase-backend-sdk-rust",
        generatedPath: "sdkwork-knowledgebase-backend-sdk-rust/generated/server-openapi",
        manifestPath: "sdkwork-knowledgebase-backend-sdk-rust/generated/server-openapi/Cargo.toml",
        packageName: "sdkwork-knowledgebase-backend-sdk-generated-rust",
        primaryClient: "SdkworkBackendClient",
      },
    ],
    dependencies: [
      dependency("sdkwork-iam-backend-sdk", "appbase-backend-management-capability", "/backend/v3/api", "sdkwork-iam-backend-api", appbaseBackendPackages),
      dependency("sdkwork-drive-backend-sdk", "drive-backend-management-capability", "/backend/v3/api", "sdkwork-drive.backend", driveBackendPackages),
      dependency("sdkwork-memory-backend-sdk", "memory-backend-management-capability", "/backend/v3/api", "sdkwork-memory.backend", memoryBackendPackages),
    ],
    forbiddenPathPrefixes: [
      "/backend/v3/api/auth/",
      "/backend/v3/api/iam/",
      "/backend/v3/api/open_platform/",
      "/backend/v3/api/system/iam/",
      "/backend/v3/api/drive/",
      "/backend/v3/api/memory/",
      "/mem/v3/api/",
    ],
  },
];

function routeCrateFor(family) {
  switch (family.sdkTarget) {
    case "app":
      return "sdkwork-routes-knowledgebase-app-api";
    case "backend":
      return "sdkwork-routes-knowledgebase-backend-api";
    case "open":
      return "sdkwork-routes-knowledgebase-open-api";
    default:
      return `sdkwork-routes-knowledgebase-${family.sdkTarget}-api`;
  }
}

function openOperation(method, sourcePath, targetPath) {
  return { method, sourcePath, targetPath };
}

function dependency(workspace, role, apiPrefix, apiAuthority, packageByLanguage) {
  return {
    workspace,
    role,
    required: true,
    dependencyMode: "consumer-sdk",
    apiPrefix,
    apiAuthority,
    generatedTransportImportPolicy: "forbidden",
    packageByLanguage,
  };
}

function jsonText(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

async function writeJson(filePath, value) {
  const desiredText = jsonText(value);
  let currentText = "";
  let exists = true;
  try {
    currentText = await readFile(filePath, "utf8");
  } catch {
    exists = false;
  }

  if (exists && currentText === desiredText) {
    return;
  }

  const relativePath = path.relative(workspaceRoot, filePath).replaceAll("\\", "/");
  if (checkOnly) {
    pendingChanges.push(relativePath);
    return;
  }

  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, desiredText, "utf8");
}

async function writeText(filePath, desiredText) {
  let currentText = "";
  let exists = true;
  try {
    currentText = await readFile(filePath, "utf8");
  } catch {
    exists = false;
  }

  if (exists && currentText === desiredText) {
    return;
  }

  const relativePath = path.relative(workspaceRoot, filePath).replaceAll("\\", "/");
  if (checkOnly) {
    pendingChanges.push(relativePath);
    return;
  }

  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, desiredText, "utf8");
}

function operationEntries(openapi) {
  const entries = [];
  for (const [pathKey, pathItem] of Object.entries(openapi.paths || {})) {
    for (const [method, operation] of Object.entries(pathItem || {})) {
      if (!httpMethods.has(method.toLowerCase()) || !operation || typeof operation !== "object") {
        continue;
      }
      entries.push({ pathKey, method: method.toLowerCase(), operation });
    }
  }
  return entries;
}

function containsPropertyName(value, propertyName) {
  if (!value || typeof value !== "object") {
    return false;
  }
  if (
    value.properties &&
    typeof value.properties === "object" &&
    Object.hasOwn(value.properties, propertyName)
  ) {
    return true;
  }
  if (Array.isArray(value)) {
    return value.some((item) => containsPropertyName(item, propertyName));
  }
  return Object.values(value).some((item) => containsPropertyName(item, propertyName));
}

function successResponseSchemas(operation) {
  const schemas = [];
  for (const [status, response] of Object.entries(operation.responses ?? {})) {
    if (!/^2[0-9][0-9]$/u.test(status)) {
      continue;
    }
    for (const mediaType of Object.values(response.content ?? {})) {
      if (mediaType?.schema && typeof mediaType.schema === "object") {
        schemas.push(mediaType.schema);
      }
    }
  }
  return schemas;
}

function hasPageInfoSuccessSchema(operation) {
  return successResponseSchemas(operation).some((schema) => containsPropertyName(schema, "pageInfo"));
}

function cursorQueryParameter() {
  return {
    name: "cursor",
    in: "query",
    required: false,
    schema: {
      type: "string",
      maxLength: 512,
    },
  };
}

function pageSizeQueryParameter() {
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

function ensurePaginatedGetQueryInputs(openapi) {
  for (const { method, operation } of operationEntries(openapi)) {
    if (method !== "get" || !hasPageInfoSuccessSchema(operation)) {
      continue;
    }

    const parameters = Array.isArray(operation.parameters) ? operation.parameters : [];
    const normalizedParameters = [];
    const queryParameterNames = new Set();

    for (const parameter of parameters) {
      if (!parameter || typeof parameter !== "object" || parameter.in !== "query") {
        normalizedParameters.push(parameter);
        continue;
      }

      const normalizedParameter = parameter.name === "pageSize"
        ? { ...parameter, name: "page_size" }
        : parameter;
      if (forbiddenPaginationQueryAliases.has(normalizedParameter.name)) {
        continue;
      }
      if (normalizedParameter.name === "cursor" || normalizedParameter.name === "page_size") {
        if (queryParameterNames.has(normalizedParameter.name)) {
          continue;
        }
        queryParameterNames.add(normalizedParameter.name);
      }
      normalizedParameters.push(normalizedParameter);
    }

    if (!queryParameterNames.has("cursor")) {
      normalizedParameters.push(cursorQueryParameter());
    }
    if (!queryParameterNames.has("page_size")) {
      normalizedParameters.push(pageSizeQueryParameter());
    }

    operation.parameters = normalizedParameters;
  }
}

function removeDependencyOwnedOperations(openapi, family) {
  const removed = [];
  for (const [pathKey, pathItem] of Object.entries(openapi.paths || {})) {
    if (!pathItem || typeof pathItem !== "object") {
      continue;
    }
    for (const [method, operation] of Object.entries({ ...pathItem })) {
      if (!httpMethods.has(method.toLowerCase()) || !operation || typeof operation !== "object") {
        continue;
      }
      const explicitOwner = operation["x-sdkwork-owner"];
      const forbiddenByOwner = explicitOwner && explicitOwner !== owner;
      const forbiddenByPrefix = family.forbiddenPathPrefixes.some((prefix) => pathKey.startsWith(prefix));
      if (!forbiddenByOwner && !forbiddenByPrefix) {
        continue;
      }
      removed.push({
        path: pathKey,
        method: method.toLowerCase(),
        operationId: operation.operationId || "",
        owner: explicitOwner || "",
      });
      delete pathItem[method];
    }

    const remainingMethods = Object.keys(pathItem).filter((method) => httpMethods.has(method.toLowerCase()));
    if (remainingMethods.length === 0) {
      delete openapi.paths[pathKey];
    }
  }
  return removed;
}

async function standardizeOpenApi(family) {
  await materializeFamilyOpenApi(family);

  const filePath = path.join(sdksRoot, family.root, family.input);
  const openapi = await readJson(filePath);
  const removedOperations = removeDependencyOwnedOperations(openapi, family);
  removeCurrentTenantRequestInputs(openapi);
  ensurePaginatedGetQueryInputs(openapi);

  openapi["x-sdkwork-owner"] = owner;
  openapi["x-sdkwork-api-authority"] = family.authority;
  openapi["x-sdkwork-sdk-family"] = family.root;
  openapi["x-sdkwork-owner-only-input"] = true;
  openapi["x-sdkwork-standard-version"] = standardVersion;
  openapi.info = {
    ...(openapi.info || {}),
    title: openapi.info?.title || family.title,
    version: openapi.info?.version || family.apiVersion,
  };

  if (removedOperations.length > 0) {
    openapi["x-sdkwork-dependency-exclusions"] = [
      ...(Array.isArray(openapi["x-sdkwork-dependency-exclusions"]) ? openapi["x-sdkwork-dependency-exclusions"] : []),
      {
        standardVersion,
        reason: "dependency-owned operations are consumed through sdkDependencies and excluded from owner SDK generation input",
        removedOperations,
      },
    ];
  }

  for (const { operation, pathKey } of operationEntries(openapi)) {
    operation["x-sdkwork-owner"] = owner;
    operation["x-sdkwork-api-authority"] = family.authority;
    operation["x-sdkwork-request-context"] = "WebRequestContext";
    operation["x-sdkwork-api-surface"] = `${family.sdkTarget}-api`;
    operation["x-sdkwork-source-route-crate"] = routeCrateFor(family);
  }

  await writeJson(filePath, openapi);
  return {
    openapi,
    operationCount: operationEntries(openapi).length,
    removedOperations,
  };
}

function removeCurrentTenantRequestInputs(openapi) {
  const schemas = openapi.components?.schemas || {};
  for (const schemaName of currentTenantRequestSchemas) {
    const schema = schemas[schemaName];
    if (!schema || typeof schema !== "object") {
      continue;
    }

    if (Array.isArray(schema.required)) {
      schema.required = schema.required.filter((field) => field !== "tenantId" && field !== "tenant_id");
      if (schema.required.length === 0) {
        delete schema.required;
      }
    }

    if (schema.properties && typeof schema.properties === "object") {
      delete schema.properties.tenantId;
      delete schema.properties.tenant_id;
    }
  }
}

async function materializeFamilyOpenApi(family) {
  if (!family.materializeFrom) {
    return;
  }

  const sourcePath = path.join(
    sdksRoot,
    family.materializeFrom.sourceFamilyRoot,
    family.materializeFrom.sourceInput,
  );
  const source = await readJson(sourcePath);
  const schemas = {};
  const target = {
    openapi: source.openapi || "3.1.0",
    info: {
      title: family.title,
      version: family.apiVersion,
    },
    servers: [
      {
        url: family.apiPrefix,
      },
    ],
    paths: {},
    components: {
      securitySchemes: {
        ApiKey: {
          type: "apiKey",
          in: "header",
          name: "X-API-Key",
        },
      },
      schemas,
    },
  };

  for (const operationMapping of family.materializeFrom.operations) {
    const sourceOperation =
      source.paths?.[operationMapping.sourcePath]?.[operationMapping.method];
    if (!sourceOperation) {
      throw new Error(
        `Missing source operation for ${operationMapping.method.toUpperCase()} ${operationMapping.sourcePath}`,
      );
    }

    const operation = structuredClone(sourceOperation);
    operation.security = [{ ApiKey: [] }];
    operation["x-sdkwork-auth-mode"] = "api-key";
    operation["x-sdkwork-owner"] = owner;
    operation["x-sdkwork-api-authority"] = family.authority;
    operation["x-sdkwork-request-context"] = "WebRequestContext";
    operation["x-sdkwork-api-surface"] = `${family.sdkTarget}-api`;
    operation["x-sdkwork-source"] = "sdks/standardize-knowledgebase-sdk-family.mjs";
    operation["x-sdkwork-source-route-crate"] = routeCrateFor(family);

    target.paths[operationMapping.targetPath] = {
      ...(target.paths[operationMapping.targetPath] || {}),
      [operationMapping.method]: operation,
    };

    collectReferencedSchemas(operation, source, schemas);
  }

  target["x-sdkwork-owner"] = owner;
  target["x-sdkwork-api-authority"] = family.authority;
  target["x-sdkwork-sdk-family"] = family.root;
  target["x-sdkwork-owner-only-input"] = true;
  target["x-sdkwork-standard-version"] = standardVersion;

  await writeJson(path.join(sdksRoot, family.root, family.input), target);
}

function collectReferencedSchemas(value, source, targetSchemas) {
  if (!value || typeof value !== "object") {
    return;
  }

  if (typeof value.$ref === "string") {
    const schemaName = schemaNameFromRef(value.$ref);
    if (schemaName && !targetSchemas[schemaName]) {
      const sourceSchema = source.components?.schemas?.[schemaName];
      if (!sourceSchema) {
        throw new Error(`Missing referenced schema ${schemaName}`);
      }
      targetSchemas[schemaName] = structuredClone(sourceSchema);
      collectReferencedSchemas(targetSchemas[schemaName], source, targetSchemas);
    }
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      collectReferencedSchemas(item, source, targetSchemas);
    }
    return;
  }

  for (const child of Object.values(value)) {
    collectReferencedSchemas(child, source, targetSchemas);
  }
}

function schemaNameFromRef(ref) {
  const prefix = "#/components/schemas/";
  return ref.startsWith(prefix) ? ref.slice(prefix.length) : null;
}

function typeScriptTransportPackageName(family) {
  return `${family.root}-generated-typescript`;
}

function languageEntriesFor(family) {
  const typeScriptEntry = {
    language: "typescript",
    workspace: family.generatedWorkspace,
    generationState: "materialized",
    releaseState: "not_published",
    generatedPath: family.generatedPath,
    manifestPath: `${family.generatedPath}/package.json`,
    consumerPackageName: family.packageName,
    transportPackageName: typeScriptTransportPackageName(family),
    version: family.apiVersion,
    description: `Generator-owned TypeScript transport SDK for ${family.authority}.`,
    consumerSurface: family.composedPackagePath
      ? {
          primaryClient: family.primaryClient,
          factory: family.composedFactory,
          apiPrefix: family.apiPrefix,
          composedManifestPath: `${family.composedPackagePath}/package.json`,
          composedEntryPath: `${family.composedPackagePath}/src/index.ts`,
          packageName: family.packageName,
        }
      : {
          primaryClient: family.primaryClient,
          apiPrefix: family.apiPrefix,
        },
  };

  const additionalEntries = (family.additionalLanguageTargets ?? []).map((target) => ({
    language: target.language,
    workspace: target.workspace,
    generationState: "materialized",
    releaseState: "not_published",
    generatedPath: target.generatedPath,
    manifestPath: target.manifestPath,
    name: target.packageName,
    version: family.apiVersion,
    description: `Generator-owned ${target.language === "rust" ? "Rust" : target.language} transport SDK for ${family.authority}.`,
    consumerSurface: {
      primaryClient: target.primaryClient,
      apiPrefix: family.apiPrefix,
    },
  }));

  return [typeScriptEntry, ...additionalEntries];
}

function generatedPackagesFor(family) {
  return Object.fromEntries(
    languageEntriesFor(family).map((entry) => [
      entry.language,
      {
        language: entry.language,
        packageName: entry.language === "typescript" ? entry.transportPackageName : entry.name,
        generatedOutput: entry.generatedPath,
      },
    ]),
  );
}

function sdkManifestFor(family, openapi, operationCount) {
  return {
    schemaVersion: 1,
    workspace: family.root,
    title: family.title,
    apiVersion: family.apiVersion,
    openapiVersion: openapi.openapi || "3.1.0",
    authoritySpec: family.input,
    generationInputSpec: family.input,
    derivedSpecs: {
      default: family.input,
    },
    apiAuthority: family.authority,
    sdkName: family.root,
    sdkFamily: family.root,
    sdkType: family.sdkTarget,
    sdkSurface: family.sdkTarget,
    packageName: family.packageName,
    transportPackageName: typeScriptTransportPackageName(family),
    apiPrefix: family.apiPrefix,
    standardProfile: "sdkwork-v3",
    ownerOnlyOperationCount: operationCount,
    standardVersion,
    managedBy: "sdks/standardize-knowledgebase-sdk-family.mjs",
    typescript: {
      composedRoot: family.generatedWorkspace,
      composedEntry: `${family.generatedWorkspace}/src/index.ts`,
      transportRoot: family.generatedPath,
      transportEntry: `${family.generatedPath}/src/index.ts`,
    },
    discoverySurface: {
      sdkTarget: family.sdkTarget,
      apiPrefix: family.apiPrefix,
      schemaUrl: family.schemaUrl,
      generatedProtocols: ["http-openapi"],
      manualTransports: [],
    },
    languages: languageEntriesFor(family),
    generatedPackages: generatedPackagesFor(family),
    sdkOwner: owner,
    sdkDependencies: family.dependencies,
    metadata: {
      standardVersion,
      ownerOnlyOperationCount: operationCount,
      managedBy: "sdks/standardize-knowledgebase-sdk-family.mjs",
    },
  };
}

function componentSpecFor(family) {
  const sdkClients = languageEntriesFor(family).map(
    (entry) => entry.consumerSurface.primaryClient,
  );
  const verificationCommands = [
    "node sdks/standardize-knowledgebase-sdk-family.mjs --check",
    "node sdks/test/verify-sdk-ownership-boundaries.test.mjs",
    "powershell -ExecutionPolicy Bypass -File tools/verify_openapi_operation_ids.ps1",
  ];

  if (family.root === "sdkwork-knowledgebase-app-sdk") {
    sdkClients.push(
      "SdkworkKnowledgebaseAppClient.knowledge.wikiPublications",
      "SdkworkKnowledgebaseAppClient.knowledge.wikiSourceFiles",
    );
    verificationCommands.splice(
      1,
      0,
      "node tools/knowledgebase_sdk_generate.mjs --check --family sdkwork-knowledgebase-app-sdk",
      "node sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi/bin/publish-core.mjs --language typescript --project-dir sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi --action check",
      "pnpm --dir sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript typecheck",
    );
  }

  return {
    schemaVersion: 1,
    kind: "sdkwork.component.spec",
    component: {
      name: family.root,
      displayName: family.title,
      version: family.apiVersion,
      type: "sdk-family",
      root: `sdkwork-knowledgebase/sdks/${family.root}`,
      domain: "knowledgebase",
      capability: "knowledgebase",
      status: "standardized",
      languages: languageEntriesFor(family).map((entry) => entry.language),
      generated: true,
      private: false,
      manifests: ["sdk-manifest.json"],
    },
    canonicalSpecs: [
      {
        file: "API_SPEC.md",
        path: "../sdkwork-specs/API_SPEC.md",
        purpose: "HTTP/OpenAPI and generated SDK contract rules.",
      },
      {
        file: "SDK_SPEC.md",
        path: "../sdkwork-specs/SDK_SPEC.md",
        purpose: "SDK generation, SDK dependency, and SDK integration rules.",
      },
      {
        file: "SDK_WORKSPACE_GENERATION_SPEC.md",
        path: "../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md",
        purpose: "SDK workspace, SDK family naming, API authority naming, and OpenAPI generation rules.",
      },
    ],
    contracts: {
      apiAuthority: {
        name: family.authority,
        prefix: family.apiPrefix,
        authorityOpenApi: family.input,
        derivedOpenApi: [family.input],
        owner,
        standard: "../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md",
      },
      publicExports: [],
      runtimeEntrypoints: ["sdk-manifest.json"],
      sdkDependencies: family.dependencies,
      sdkClients,
      events: [],
      configKeys: ["sdk-manifest.json"],
    },
    integration: {
      authority: "Root SDKWork specs remain authoritative. Local specs may extend but must not contradict them.",
      dependencyPolicy: "Appbase, drive, and memory capabilities are consumed through declared dependency SDKs, not copied into generated knowledgebase transports.",
      sdkPolicy: family.composedPackagePath
        ? "Consumers import the composed TypeScript facade (`createKnowledgebaseAppClient`) from the SDK family workspace root; generated transport under `generated/server-openapi` remains generator-owned and must not be imported directly."
        : "Generated SDK clients are injected through service/runtime boundaries; consumers must not create raw HTTP clients or manual auth headers.",
      languagePolicy: `Generated languages (${languageEntriesFor(family).map((entry) => entry.language).join(", ")}) use the same owner-only OpenAPI input and sdkDependencies.`,
    },
    verification: {
      commands: verificationCommands,
    },
    metadata: {
      managedBy: "sdks/standardize-knowledgebase-sdk-family.mjs",
      standardVersion,
    },
  };
}

function composedAppSdkIndexSource(family) {
  return `import {
  createClient as createGeneratedKnowledgebaseAppClient,
  SdkworkKnowledgebaseAppClient,
} from '../generated/server-openapi/src/index';
import type { SdkworkAppConfig } from '../generated/server-openapi/src/types/common';

export { SdkworkKnowledgebaseAppClient, createGeneratedKnowledgebaseAppClient };
export type { SdkworkAppConfig };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';

export function ${family.composedFactory}(config: SdkworkAppConfig): SdkworkKnowledgebaseAppClient {
  return createGeneratedKnowledgebaseAppClient(config);
}

export function createClient(config: SdkworkAppConfig): SdkworkKnowledgebaseAppClient {
  return ${family.composedFactory}(config);
}
`;
}

function composedAppSdkPackageJson(family) {
  return {
    name: family.packageName,
    version: family.apiVersion,
    description: "SDKWork Knowledgebase App SDK with composed consumer facade.",
    type: "module",
    private: false,
    main: "./src/index.ts",
    module: "./src/index.ts",
    types: "./src/index.ts",
    files: [
      "src",
      "generated/server-openapi/dist",
      "generated/server-openapi/src",
    ],
    exports: {
      ".": {
        types: "./src/index.ts",
        import: "./src/index.ts",
        default: "./src/index.ts",
      },
    },
    dependencies: {
      "@sdkwork/sdk-common": "^1.0.2",
      // Generated transport code imports shared helpers (sha256Hash/getPath) from
      // @sdkwork/utils; the workspace link must stay declared for type resolution.
      "@sdkwork/utils": "workspace:*",
    },
    peerDependencies: {
      "@sdkwork/iam-app-sdk": "workspace:*",
    },
    peerDependenciesMeta: {
      "@sdkwork/iam-app-sdk": { optional: true },
    },
    scripts: {
      typecheck: "tsc --noEmit",
    },
    devDependencies: {
      typescript: "^5.8.2",
    },
    keywords: ["sdkwork", "knowledgebase", "app-sdk"],
  };
}

function composedAppSdkTsconfigSource() {
  return `{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "declaration": true,
    "declarationMap": true,
    "emitDeclarationOnly": true,
    "outDir": "dist",
    "lib": [
      "ES2022",
      "DOM",
      "DOM.Iterable"
    ]
  },
  "include": [
    "src/**/*.ts"
  ],
  "exclude": [
    "generated/server-openapi/node_modules",
    "generated/server-openapi/dist"
  ]
}
`;
}

async function materializeComposedAppSdk(family) {
  if (!family.composedPackagePath) {
    return;
  }

  const composedRoot = path.join(sdksRoot, family.root, family.composedPackagePath);
  await writeJson(path.join(composedRoot, "package.json"), composedAppSdkPackageJson(family));
  await writeText(path.join(composedRoot, "src", "index.ts"), composedAppSdkIndexSource(family));
  await writeText(path.join(composedRoot, "tsconfig.json"), composedAppSdkTsconfigSource());
}

async function standardizeFamily(family) {
  const { openapi, operationCount, removedOperations } = await standardizeOpenApi(family);
  await writeJson(path.join(sdksRoot, family.root, "sdk-manifest.json"), sdkManifestFor(family, openapi, operationCount));
  await writeJson(path.join(sdksRoot, family.root, "specs", "component.spec.json"), componentSpecFor(family));
  await materializeComposedAppSdk(family);
  return {
    family: family.root,
    authority: family.authority,
    operationCount,
    removedDependencyOperations: removedOperations.length,
  };
}

const summary = [];
for (const family of families) {
  summary.push(await standardizeFamily(family));
}

if (checkOnly && pendingChanges.length > 0) {
  console.error(JSON.stringify({ ok: false, mode: "check", owner, standardVersion, pendingChanges, families: summary }, null, 2));
  process.exit(1);
}

console.log(JSON.stringify({ ok: true, mode: checkOnly ? "check" : "apply", owner, standardVersion, families: summary }, null, 2));
