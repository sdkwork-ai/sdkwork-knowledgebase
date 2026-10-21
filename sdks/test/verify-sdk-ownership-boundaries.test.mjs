import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const sdksRoot = path.resolve(testDir, "..");
const workspaceRoot = path.resolve(sdksRoot, "..");

const families = [
  {
    root: "sdkwork-knowledgebase-sdk",
    owner: "sdkwork-knowledgebase",
    authority: "sdkwork-knowledgebase-open-api",
    input: "openapi/knowledgebase-open-api.openapi.json",
    manifest: "sdk-manifest.json",
    generatedMetadata:
      "sdkwork-knowledgebase-sdk-typescript/generated/server-openapi/sdkwork-sdk.json",
    generatedPackage:
      "sdkwork-knowledgebase-sdk-typescript/generated/server-openapi/package.json",
    languages: ["typescript"],
    dependencies: [],
    forbiddenPathPrefixes: [
      "/app/v3/api/",
      "/backend/v3/api/",
      "/mem/v3/api/",
      "/open/v3/api/drive/",
    ],
  },
  {
    root: "sdkwork-knowledgebase-app-sdk",
    owner: "sdkwork-knowledgebase",
    authority: "sdkwork-knowledgebase-app-api",
    input: "openapi/knowledgebase-app-api.openapi.json",
    manifest: "sdk-manifest.json",
    generatedMetadata:
      "sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi/sdkwork-sdk.json",
    generatedPackage:
      "sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi/package.json",
    languages: ["typescript", "rust"],
    dependencies: [
      ["sdkwork-iam-app-sdk", "sdkwork-iam-app-api"],
      ["sdkwork-drive-app-sdk", "sdkwork-drive.app"],
      ["sdkwork-memory-app-sdk", "sdkwork-memory.app"],
    ],
    forbiddenPathPrefixes: [
      "/app/v3/api/auth/",
      "/app/v3/api/iam/",
      "/app/v3/api/open_platform/",
      "/app/v3/api/system/iam/",
      "/app/v3/api/drive/",
      "/app/v3/api/memory/",
      "/backend/v3/api/",
      "/mem/v3/api/",
    ],
  },
  {
    root: "sdkwork-knowledgebase-backend-sdk",
    owner: "sdkwork-knowledgebase",
    authority: "sdkwork-knowledgebase-backend-api",
    input: "openapi/knowledgebase-backend-api.openapi.json",
    manifest: "sdk-manifest.json",
    generatedMetadata:
      "sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi/sdkwork-sdk.json",
    generatedPackage:
      "sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi/package.json",
    languages: ["typescript", "rust"],
    dependencies: [
      ["sdkwork-iam-backend-sdk", "sdkwork-iam-backend-api"],
      ["sdkwork-drive-backend-sdk", "sdkwork-drive.backend"],
      ["sdkwork-memory-backend-sdk", "sdkwork-memory.backend"],
    ],
    forbiddenPathPrefixes: [
      "/backend/v3/api/auth/",
      "/backend/v3/api/iam/",
      "/backend/v3/api/open_platform/",
      "/backend/v3/api/system/iam/",
      "/backend/v3/api/drive/",
      "/backend/v3/api/memory/",
      "/app/v3/api/",
      "/mem/v3/api/",
    ],
  },
];

function readJson(relativePath) {
  return JSON.parse(readFileSync(path.join(workspaceRoot, relativePath), "utf8"));
}

function operationEntries(openapi) {
  const entries = [];
  for (const [pathKey, pathItem] of Object.entries(openapi.paths || {})) {
    for (const [method, operation] of Object.entries(pathItem || {})) {
      if (!["get", "put", "post", "patch", "delete", "head", "options", "trace"].includes(method)) {
        continue;
      }
      entries.push({ pathKey, method, operation });
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

function queryParameterNames(operation) {
  return (operation.parameters ?? [])
    .filter((parameter) => parameter?.in === "query")
    .map((parameter) => parameter.name);
}

test("knowledgebase SDK family manifests declare owner-only authority metadata", () => {
  for (const family of families) {
    const manifestPath = path.join("sdks", family.root, family.manifest);
    assert.ok(existsSync(path.join(workspaceRoot, manifestPath)), `${family.root} must have ${manifestPath}`);

    const manifest = readJson(manifestPath);
    assert.equal(manifest.sdkOwner, family.owner, `${family.root} must declare sdkOwner`);
    assert.equal(manifest.apiAuthority, family.authority, `${family.root} must declare apiAuthority`);
    assert.equal(manifest.generationInputSpec, family.input, `${family.root} must generate from owner-only OpenAPI input`);

    assert.deepEqual(
      manifest.sdkDependencies?.map((dependency) => ({
        workspace: dependency.workspace,
        apiAuthority: dependency.apiAuthority,
        dependencyMode: dependency.dependencyMode,
        generatedTransportImportPolicy: dependency.generatedTransportImportPolicy,
      })),
      family.dependencies.map(([workspace, apiAuthority]) => ({
        workspace,
        apiAuthority,
        dependencyMode: "consumer-sdk",
        generatedTransportImportPolicy: "forbidden",
      })),
      `${family.root} must declare appbase, drive, and memory as consumer SDK dependencies`,
    );
    assert.equal(
      existsSync(path.join(workspaceRoot, "sdks", family.root, "sdk-assembly.json")),
      false,
      `${family.root} must use sdk-manifest.json rather than a retired per-family assembly`,
    );
  }
});

test("knowledgebase component specs mirror SDK dependency boundaries", () => {
  for (const family of families) {
    const componentSpecPath = path.join("sdks", family.root, "specs", "component.spec.json");
    assert.ok(
      existsSync(path.join(workspaceRoot, componentSpecPath)),
      `${family.root} must have ${componentSpecPath}`,
    );

    const componentSpec = readJson(componentSpecPath);
    assert.equal(componentSpec.component?.name, family.root, `${family.root} component name must match SDK family`);
    assert.equal(
      componentSpec.contracts?.apiAuthority?.name,
      family.authority,
      `${family.root} component spec must declare the owner API authority`,
    );
    assert.deepEqual(
      componentSpec.contracts?.sdkDependencies?.map((dependency) => ({
        workspace: dependency.workspace,
        apiAuthority: dependency.apiAuthority,
        dependencyMode: dependency.dependencyMode,
        generatedTransportImportPolicy: dependency.generatedTransportImportPolicy,
      })),
      family.dependencies.map(([workspace, apiAuthority]) => ({
        workspace,
        apiAuthority,
        dependencyMode: "consumer-sdk",
        generatedTransportImportPolicy: "forbidden",
      })),
      `${family.root} component spec must mirror appbase, drive, and memory SDK dependencies`,
    );
  }
});

test("knowledgebase SDK manifests record owner and dependency boundaries outside generated metadata", () => {
  for (const family of families) {
    const manifest = readJson(path.join("sdks", family.root, family.manifest));

    assert.equal(manifest.sdkOwner, family.owner, `${family.root} manifest must declare sdkOwner`);
    assert.equal(manifest.apiAuthority, family.authority, `${family.root} manifest must declare apiAuthority`);
    assert.equal(
      manifest.generationInputSpec,
      family.input,
      `${family.root} manifest must point at owner-only OpenAPI input`,
    );
    assert.equal(
      manifest.transportPackageName,
      `${family.root}-generated-typescript`,
      `${family.root} manifest must declare the generated TypeScript transport package`,
    );
    assert.deepEqual(
      manifest.languages?.map((entry) => entry.language),
      family.languages,
      `${family.root} manifest must declare every generated language`,
    );
    assert.deepEqual(
      manifest.sdkDependencies?.map((dependency) => ({
        workspace: dependency.workspace,
        apiAuthority: dependency.apiAuthority,
        dependencyMode: dependency.dependencyMode,
        generatedTransportImportPolicy: dependency.generatedTransportImportPolicy,
      })),
      family.dependencies.map(([workspace, apiAuthority]) => ({
        workspace,
        apiAuthority,
        dependencyMode: "consumer-sdk",
        generatedTransportImportPolicy: "forbidden",
      })),
      `${family.root} manifest must mirror appbase, drive, and memory SDK dependencies`,
    );

    const generatedMetadataPath = path.join("sdks", family.root, family.generatedMetadata);
    const generatedPackagePath = path.join("sdks", family.root, family.generatedPackage);
    if (existsSync(path.join(workspaceRoot, generatedMetadataPath))) {
      const generatedMetadata = readJson(generatedMetadataPath);
      for (const forbiddenKey of [
        "sdkOwner",
        "apiAuthority",
        "sdkFamily",
        "generationInputSpec",
        "sdkDependencies",
        "ownerOnlyOperationCount",
        "standardProfile",
        "standardVersion",
      ]) {
        assert.equal(
          Object.hasOwn(generatedMetadata, forbiddenKey),
          false,
          `${family.root} generated metadata must not carry ownership standard key ${forbiddenKey}`,
        );
      }
    }

    if (existsSync(path.join(workspaceRoot, generatedPackagePath))) {
      const generatedPackage = readJson(generatedPackagePath);
      assert.equal(
        Object.hasOwn(generatedPackage, "sdkwork"),
        false,
        `${family.root} generated package.json must not carry SDK ownership standard metadata`,
      );
    }
  }
});

test("knowledgebase app SDK exposes composed consumer facade outside generated transport", () => {
  const manifest = readJson("sdks/sdkwork-knowledgebase-app-sdk/sdk-manifest.json");
  assert.equal(
    manifest.typescript?.composedRoot,
    "sdkwork-knowledgebase-app-sdk-typescript",
    "app SDK manifest must declare composed consumer workspace path",
  );

  const composedPackage = readJson(
    "sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/package.json",
  );
  assert.equal(
    composedPackage.name,
    "@sdkwork/knowledgebase-app-sdk",
    "composed app SDK package must own the public package name",
  );

  const generatedPackage = readJson(
    "sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi/package.json",
  );
  assert.equal(
    generatedPackage.name,
    "sdkwork-knowledgebase-app-sdk-generated-typescript",
    "generated transport package must not reuse the public app SDK package name",
  );

  assert.equal(
    Object.hasOwn(composedPackage.exports, "./generated"),
    false,
    "composed app SDK must not expose generated transport as a public subpath",
  );

  const composedSource = readFileSync(
    path.join(
      workspaceRoot,
      "sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/src/index.ts",
    ),
    "utf8",
  );
  assert.match(
    composedSource,
    /export function createKnowledgebaseAppClient/u,
    "composed app SDK entry must export createKnowledgebaseAppClient",
  );
});

test("knowledgebase backend SDK declares a generated Rust transport", () => {
  const manifest = readJson("sdks/sdkwork-knowledgebase-backend-sdk/sdk-manifest.json");
  const rustEntry = manifest.languages?.find((entry) => entry.language === "rust");

  assert.deepEqual(
    rustEntry,
    {
      language: "rust",
      workspace: "sdkwork-knowledgebase-backend-sdk-rust",
      generationState: "materialized",
      releaseState: "not_published",
      generatedPath: "sdkwork-knowledgebase-backend-sdk-rust/generated/server-openapi",
      manifestPath: "sdkwork-knowledgebase-backend-sdk-rust/generated/server-openapi/Cargo.toml",
      name: "sdkwork-knowledgebase-backend-sdk-generated-rust",
      version: "0.1.0",
      description: "Generator-owned Rust transport SDK for sdkwork-knowledgebase-backend-api.",
      consumerSurface: {
        primaryClient: "SdkworkBackendClient",
        apiPrefix: "/backend/v3/api",
      },
    },
    "backend Rust output must be declared in the family manifest",
  );

  const cargoManifestPath = path.join(
    workspaceRoot,
    "sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-rust/generated/server-openapi/Cargo.toml",
  );
  assert.ok(existsSync(cargoManifestPath), "backend Rust transport must be generated");
  assert.match(
    readFileSync(cargoManifestPath, "utf8"),
    /name = "sdkwork-knowledgebase-backend-sdk-generated-rust"/u,
    "backend Rust transport Cargo package must match the declared package",
  );
});

test("knowledgebase generated OpenAPI inputs contain only sdkwork-knowledgebase owned operations", () => {
  for (const family of families) {
    const openapi = readJson(path.join("sdks", family.root, family.input));
    assert.equal(openapi["x-sdkwork-owner"], family.owner);
    assert.equal(openapi["x-sdkwork-api-authority"], family.authority);

    for (const { pathKey, method, operation } of operationEntries(openapi)) {
      assert.equal(
        operation["x-sdkwork-owner"],
        family.owner,
        `${family.root} ${method.toUpperCase()} ${pathKey} must be knowledgebase-owned`,
      );
      assert.equal(
        operation["x-sdkwork-api-authority"],
        family.authority,
        `${family.root} ${method.toUpperCase()} ${pathKey} must use ${family.authority}`,
      );
      assert(
        !family.forbiddenPathPrefixes.some((prefix) => pathKey.startsWith(prefix)),
        `${family.root} must not copy dependency-owned route ${method.toUpperCase()} ${pathKey}`,
      );
    }
  }
});

test("knowledgebase paginated GET operations declare standard SDKWork pagination query inputs", () => {
  const forbiddenAliases = new Set(["pageSize", "limit", "page_no", "pageNo", "per_page", "size"]);

  for (const family of families) {
    const openapi = readJson(path.join("sdks", family.root, family.input));
    for (const { pathKey, method, operation } of operationEntries(openapi)) {
      if (method !== "get" || !hasPageInfoSuccessSchema(operation)) {
        continue;
      }

      const parameterNames = queryParameterNames(operation);
      const operationLabel = `${family.root} ${method.toUpperCase()} ${pathKey} (${operation.operationId})`;

      assert(
        parameterNames.includes("page_size"),
        `${operationLabel} must declare canonical page_size query input`,
      );
      assert(
        parameterNames.includes("cursor") || parameterNames.includes("page"),
        `${operationLabel} must declare cursor or page query input`,
      );
      assert.equal(
        parameterNames.some((name) => forbiddenAliases.has(name)),
        false,
        `${operationLabel} must not declare legacy pagination aliases`,
      );
    }
  }
});
