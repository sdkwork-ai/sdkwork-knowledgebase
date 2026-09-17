import * as esbuild from "esbuild";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";

const scriptsRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptsRoot, "..");
const PROFILE_ENVIRONMENTS = ["development", "test", "staging", "production", "demo"];
const DEPLOYMENT_PROFILES = ["standalone", "cloud"];

const { values } = parseArgs({
  args: process.argv.slice(2),
  options: {
    "deployment-profile": { type: "string", default: "standalone" },
    environment: { type: "string", default: "development" },
  },
  strict: true,
});
const deploymentProfile = values["deployment-profile"];
const environment = values.environment;
if (!DEPLOYMENT_PROFILES.includes(deploymentProfile)) {
  throw new Error("--deployment-profile must be standalone or cloud");
}
if (!PROFILE_ENVIRONMENTS.includes(environment)) {
  throw new Error("--environment must be development, test, staging, production, or demo");
}
const profileId = `${deploymentProfile}.${environment}`;
const runtimeConfigPath = path.join(appRoot, "config", "mini-program", `runtime-env.${profileId}.json`);
if (!existsSync(runtimeConfigPath)) {
  throw new Error(`Mini program runtime profile does not exist: ${runtimeConfigPath}`);
}
const runtimeConfig = JSON.parse(readFileSync(runtimeConfigPath, "utf8"));
for (const [key, expected] of Object.entries({
  SDKWORK_DEPLOYMENT_PROFILE: deploymentProfile,
  SDKWORK_ENVIRONMENT: environment,
  SDKWORK_PROFILE_ID: profileId,
  SDKWORK_RUNTIME_TARGET: "mini-program",
})) {
  if (runtimeConfig[key] !== expected) {
    throw new Error(`${profileId} must declare ${key}=${expected}`);
  }
}

const runtimeDir = path.join(appRoot, "src", "runtime");
mkdirSync(runtimeDir, { recursive: true });

await esbuild.build({
  entryPoints: [path.join(appRoot, "src/bootstrap/runtimeBundle.ts")],
  bundle: true,
  outfile: path.join(runtimeDir, "knowledgebase-app.js"),
  platform: "browser",
  format: "cjs",
  target: "es2019",
  minifySyntax: true,
  minifyWhitespace: true,
  minifyIdentifiers: false,
  legalComments: "none",
  logLevel: "info",
});

writeFileSync(
  path.join(runtimeDir, "runtime-env.js"),
  `module.exports = ${JSON.stringify(runtimeConfig, null, 2)};\n`,
  "utf8",
);
writeFileSync(
  path.join(runtimeDir, "build-manifest.json"),
  `${JSON.stringify({
    deploymentProfile,
    environment,
    profileId,
    runtimeTarget: "mini-program",
    platform: "MP_WEIXIN",
  }, null, 2)}\n`,
  "utf8",
);
