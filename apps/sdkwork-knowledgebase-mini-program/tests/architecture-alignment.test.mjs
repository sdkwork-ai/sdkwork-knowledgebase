import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const readJson = (relative) =>
  JSON.parse(readFileSync(new URL(`../${relative}`, import.meta.url), "utf8"));

const appConfig = readJson("sdkwork.app.config.json");
const appJson = readJson("src/app.json");
const componentSpec = readJson("specs/component.spec.json");

test("mini program root declares the MP_WEIXIN runtime family", () => {
  assert.equal(appConfig.app.key, "sdkwork-knowledgebase-mini-program");
  assert.equal(appConfig.runtime.family, "mini-program");
  assert.deepEqual(appConfig.runtime.runtimes, ["MP_WEIXIN"]);
  assert.equal(componentSpec.component.type, "mini-program-app-root");
});

test("every declared page exists on disk", async () => {
  for (const page of appJson.pages) {
    const url = new URL(`../src/${page}.js`, import.meta.url);
    const { statSync } = await import("node:fs");
    assert.ok(statSync(url).isFile(), `missing page implementation: ${page}.js`);
  }
});

test("route identity matches the PC workbench capability set", () => {
  assert.deepEqual(appJson.pages, [
    "pages/home/index",
    "pages/knowledgebase/index",
    "pages/knowledgebase-detail/index",
    "pages/knowledgebase-search/index",
    "pages/group-launch/index",
  ]);
});
