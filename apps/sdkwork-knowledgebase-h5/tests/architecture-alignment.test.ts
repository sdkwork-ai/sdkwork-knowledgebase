import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS } from "@sdkwork/knowledgebase-h5-knowledge/routes";

const appConfig = JSON.parse(
  readFileSync(new URL("../sdkwork.app.config.json", import.meta.url), "utf8"),
) as { app: { key: string }; runtime: { family: string } };

test("H5 root declares the h5 application family", () => {
  assert.equal(appConfig.app.key, "sdkwork-knowledgebase-h5");
  assert.equal(appConfig.runtime.family, "mobile");
});

test("H5 route identity matches the PC workbench capability set", () => {
  assert.deepEqual(KNOWLEDGEBASE_CAPABILITY_ROUTE_IDS, [
    "app.intelligence.knowledgebase.list",
    "app.intelligence.knowledgebase.detail",
    "app.intelligence.knowledgebase.search",
    "app.intelligence.knowledgebase.settings",
    "app.intelligence.knowledgebase.launch",
  ]);
});
