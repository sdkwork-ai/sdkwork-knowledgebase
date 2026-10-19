#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, "..");
const sdkgen = path.resolve(workspaceRoot, "../sdkwork-sdk-generator/bin/sdkgen.js");
const checkOnly = process.argv.includes("--check");
const familyArgumentIndex = process.argv.indexOf("--family");
const requestedFamily =
  familyArgumentIndex >= 0 ? process.argv[familyArgumentIndex + 1] : undefined;

const families = [
  {
    input: "sdks/sdkwork-knowledgebase-sdk/openapi/knowledgebase-open-api.openapi.json",
    name: "sdkwork-knowledgebase-sdk",
    type: "custom",
    apiPrefix: "/knowledge/v3/api",
    targets: [
      {
        language: "typescript",
        output: "sdks/sdkwork-knowledgebase-sdk/sdkwork-knowledgebase-sdk-typescript/generated/server-openapi",
        packageName: "@sdkwork/knowledgebase-sdk",
        clientName: "SdkworkKnowledgebaseOpenClient",
      },
    ],
  },
  {
    input: "sdks/sdkwork-knowledgebase-app-sdk/openapi/knowledgebase-app-api.openapi.json",
    name: "sdkwork-knowledgebase-app-sdk",
    type: "app",
    apiPrefix: "/app/v3/api",
    targets: [
      {
        language: "typescript",
        output: "sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-typescript/generated/server-openapi",
        packageName: "@sdkwork/knowledgebase-app-sdk",
        clientName: "SdkworkKnowledgebaseAppClient",
      },
      {
        language: "rust",
        output:
          "sdks/sdkwork-knowledgebase-app-sdk/sdkwork-knowledgebase-app-sdk-rust/generated/server-openapi",
        packageName: "sdkwork-knowledgebase-app-sdk-generated-rust",
      },
    ],
  },
  {
    input: "sdks/sdkwork-knowledgebase-backend-sdk/openapi/knowledgebase-backend-api.openapi.json",
    name: "sdkwork-knowledgebase-backend-sdk",
    type: "backend",
    apiPrefix: "/backend/v3/api",
    targets: [
      {
        language: "typescript",
        output: "sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-typescript/generated/server-openapi",
        packageName: "@sdkwork/knowledgebase-backend-sdk",
        clientName: "SdkworkKnowledgebaseBackendClient",
      },
      {
        language: "rust",
        output: "sdks/sdkwork-knowledgebase-backend-sdk/sdkwork-knowledgebase-backend-sdk-rust/generated/server-openapi",
        packageName: "sdkwork-knowledgebase-backend-sdk-generated-rust",
      },
    ],
  },
  {
    input:
      "sdks/sdkwork-knowledgebase-internal-sdk/openapi/sdkwork-knowledgebase-internal-api.sdkgen.yaml",
    authoritySource:
      "apis/internal-api/knowledgebase/sdkwork-knowledgebase-internal-api.openapi.yaml",
    authorityMirror:
      "sdks/sdkwork-knowledgebase-internal-sdk/openapi/sdkwork-knowledgebase-internal-api.openapi.yaml",
    name: "sdkwork-knowledgebase-internal-sdk",
    type: "custom",
    apiPrefix: "/internal/v3/api",
    targets: [
      {
        language: "typescript",
        output:
          "sdks/sdkwork-knowledgebase-internal-sdk/sdkwork-knowledgebase-internal-sdk-typescript/generated/server-openapi",
        packageName: "sdkwork-knowledgebase-internal-sdk-generated-typescript",
        clientName: "SdkworkKnowledgebaseInternalClient",
      },
      {
        language: "rust",
        output:
          "sdks/sdkwork-knowledgebase-internal-sdk/sdkwork-knowledgebase-internal-sdk-rust/generated/server-openapi",
        packageName: "sdkwork-knowledgebase-internal-sdk-generated-rust",
      },
    ],
  },
];

function materializeFamilyInput(family) {
  if (!family.authoritySource || !family.authorityMirror) {
    return;
  }
  const sourcePath = path.join(workspaceRoot, family.authoritySource);
  const mirrorPath = path.join(workspaceRoot, family.authorityMirror);
  const generationInputPath = path.join(workspaceRoot, family.input);
  const source = readFileSync(sourcePath);

  if (checkOnly) {
    for (const candidate of [mirrorPath, generationInputPath]) {
      const candidateContent = readFileSync(candidate);
      if (!source.equals(candidateContent)) {
        console.error(
          `Knowledgebase SDK authority drift: ${path.relative(workspaceRoot, candidate)} != ${family.authoritySource}`,
        );
        process.exit(1);
      }
    }
    return;
  }

  for (const candidate of [mirrorPath, generationInputPath]) {
    mkdirSync(path.dirname(candidate), { recursive: true });
    writeFileSync(candidate, source);
  }
}

function runGenerate(family, target) {
  const args = [
    sdkgen,
    "generate",
    "-i",
    path.join(workspaceRoot, family.input),
    "-o",
    path.join(workspaceRoot, target.output),
    "-n",
    family.name,
    "-t",
    family.type,
    "-l",
    target.language,
    "--package-name",
    target.packageName,
    "--api-prefix",
    family.apiPrefix,
    "--standard-profile",
    "sdkwork-v3",
    "--fixed-sdk-version",
    "0.1.0",
  ];

  if (target.clientName) {
    args.push("--client-name", target.clientName);
  }
  if (checkOnly) {
    args.push("--dry-run");
  }

  const result = spawnSync("node", args, { stdio: "inherit", cwd: workspaceRoot });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const selectedFamilies = requestedFamily
  ? families.filter((family) => family.name === requestedFamily)
  : families;

if (requestedFamily && selectedFamilies.length !== 1) {
  console.error(`Unknown Knowledgebase SDK family: ${requestedFamily}`);
  process.exit(1);
}

for (const family of selectedFamilies) {
  materializeFamilyInput(family);
  for (const target of family.targets) {
    console.log(`Generating ${target.language} SDK for ${family.name}`);
    runGenerate(family, target);
  }
}

console.log("Knowledgebase SDK generation completed.");
