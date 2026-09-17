import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Static architecture contract for the SDKWork Knowledgebase HarmonyOS mobile
 * root.
 *
 * Authority: `HARMONY_APP_MOBILE_ARCHITECTURE_SPEC.md` section 11
 * (root layout / package naming / root thinness / SDK boundary) and
 * `APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`.
 */

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function mustExist(relativePath) {
  const absolute = path.join(root, relativePath);
  assert.ok(fs.existsSync(absolute), relativePath + " must exist");
  return fs.readFileSync(absolute, "utf8");
}

function listFiles(relativePath) {
  const absolute = path.join(root, relativePath);
  if (!fs.existsSync(absolute)) return [];
  return fs.readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const childRelative = path.join(relativePath, entry.name);
    if (entry.isDirectory()) {
      if (["node_modules", "build", "oh_modules", ".hvigor"].includes(entry.name)) return [];
      return listFiles(childRelative);
    }
    return [childRelative];
  });
}

// --- Root layout -----------------------------------------------------------

for (const requiredPath of [
  "AGENTS.md",
  "README.md",
  "sdkwork.app.config.json",
  "specs/component.spec.json",
  "oh-package.json5",
  "build-profile.json5",
  "hvigorfile.ts",
  ".sdkwork/README.md",
  ".sdkwork/skills/README.md",
  ".sdkwork/plugins/README.md",
  "etc/README.md",
  "etc/sdkwork.deployment.config.json",
  "AppScope/app.json5",
  "entry/oh-package.json5",
  "entry/build-profile.json5",
  "entry/src/main/module.json5",
  "entry/src/main/ets/entryability/EntryAbility.ets",
  "sdks/README.md",
  "scripts/README.md",
]) {
  mustExist(requiredPath);
}

// --- Package family --------------------------------------------------------

const PREFIX = "sdkwork-knowledgebase-harmony-mobile";
const expectedPackages = [
  PREFIX + "-core",
  PREFIX + "-commons",
  PREFIX + "-shell",
  PREFIX + "-host",
  PREFIX + "-knowledge",
];

for (const packageName of expectedPackages) {
  const packageRoot = "packages/" + packageName;
  mustExist(packageRoot + "/oh-package.json5");
  mustExist(packageRoot + "/build-profile.json5");
  mustExist(packageRoot + "/src/main/module.json5");
  mustExist(packageRoot + "/src/main/ets/Index.ets");
  const componentSpec = JSON.parse(mustExist(packageRoot + "/specs/component.spec.json"));
  assert.equal(
    componentSpec.component?.root,
    "apps/sdkwork-knowledgebase-harmony-mobile/packages/" + packageName,
    packageName + " component spec root must use the canonical HarmonyOS package path",
  );
}

// --- Component spec layer roles -------------------------------------------

const allowedLayerRoles = new Set([
  "contract",
  "frontend-core",
  "frontend-shell",
  "frontend-feature",
  "frontend-commons",
  "frontend-host",
  "backend-route",
  "backend-service",
  "backend-domain",
  "backend-repository",
  "backend-provider",
  "runtime-api-server",
  "runtime-service-host",
  "runtime-composition",
  "runtime-gateway",
  "runtime-native-host",
  "sdk-facade",
  "sdk-generated",
  "tooling",
]);

for (const packageName of expectedPackages) {
  const componentSpec = JSON.parse(
    mustExist("packages/" + packageName + "/specs/component.spec.json"),
  );
  const layerRole = componentSpec.contracts?.layerRole;
  if (layerRole !== undefined) {
    assert.ok(
      allowedLayerRoles.has(layerRole),
      packageName + " contracts.layerRole " + JSON.stringify(layerRole) + " is not an allowed composable layer role",
    );
  }
}

// --- Root thinness ---------------------------------------------------------

const entryFiles = listFiles("entry/src/main/ets");
const businessOwnedEntryFiles = entryFiles.filter((filePath) => {
  const normalized = filePath.replaceAll("\\", "/");
  return /\/pages\/(?!Index\.ets|__generated__)/u.test(normalized);
});
assert.deepEqual(
  businessOwnedEntryFiles,
  [],
  "root entry/ must stay thin: business pages belong in capability packages",
);

// --- Config -----------------------------------------------------------------

for (const deploymentProfile of ["standalone", "cloud"]) {
  for (const environment of ["development", "test", "staging", "production"]) {
    const profileId = deploymentProfile + "." + environment;
    const runtimeConfig = JSON.parse(
      mustExist("config/app/runtime-env." + profileId + ".json"),
    );
    assert.equal(runtimeConfig.profileId, profileId, profileId + " must declare its profileId");
    assert.equal(
      runtimeConfig.deploymentProfile,
      deploymentProfile,
      profileId + " must declare deploymentProfile=" + deploymentProfile,
    );
    assert.equal(
      runtimeConfig.environment,
      environment,
      profileId + " must declare environment=" + environment,
    );
    assert.equal(
      runtimeConfig.runtimeTarget,
      "harmony-native",
      profileId + " must declare runtimeTarget=harmony-native",
    );
  }
}

// --- Host config must stay secret-free -------------------------------------

const hostConfigFiles = listFiles("config/host");
assert.ok(hostConfigFiles.length > 0, "config/host must contain checked-in templates");
const secretPattern = /(signingPrivateKey|privateKey|refreshToken|apiKey|databaseUrl|password)\s*[:=]\s*["'][^"'<]/iu;
for (const hostFile of hostConfigFiles) {
  const content = mustExist(hostFile);
  assert.doesNotMatch(content, secretPattern, hostFile + " must not contain secrets");
}

// --- App manifest ----------------------------------------------------------

const manifest = JSON.parse(mustExist("sdkwork.app.config.json"));
assert.equal(manifest.app?.appType, "APP_HARMONY", "Harmony root must declare appType APP_HARMONY");
assert.equal(manifest.runtime?.family, "mobile", "Harmony root must declare runtime.family mobile");
assert.equal(
  manifest.runtime?.framework,
  "harmony-native",
  "Harmony root must declare runtime.framework harmony-native",
);
assert.ok(
  manifest.publish?.platforms?.includes("APP_HARMONY"),
  "Harmony root must publish for APP_HARMONY",
);

// --- SDK boundary ----------------------------------------------------------

const featureSource = listFiles("packages/" + PREFIX + "-knowledge/src")
  .filter((filePath) => filePath.endsWith(".ets"))
  .map((filePath) => mustExist(filePath))
  .join("\n");
assert.doesNotMatch(
  featureSource,
  /@ohos\.net\.http|http\.createHttp|fetch\(/u,
  "capability packages must not perform raw HTTP transport",
);

console.log("harmony surface contract passed.");
