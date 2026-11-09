#!/usr/bin/env node
// RETIRED — superseded by PERMISSION_STANDARD_SPEC §Surface Authorization Tiers.
//
// First-party app-api is a consumer surface (tier 0–2): per-route RBAC scopes are a
// contract violation there, and access is enforced by the service layer's space/document
// ownership and member ACL checks. The canonical pipeline now strips every
// `x-sdkwork-permission`/`x-sdkwork-auth-tier` from the app-api contract
// (`tools/apply-knowledgebase-openapi-permissions.mjs`) and the Rust route manifest
// (`crates/sdkwork-routes-knowledgebase-app-api/src/http_route_manifest.rs`) carries no
// `required_permission`.
//
// This entrypoint is kept only so stale automation fails with guidance instead of
// silently reintroducing permission metadata.
console.error(
  'patch-app-route-permissions.mjs is retired: app-api carries no per-route permissions ' +
    '(tier 0-2 policy). Regenerate contract metadata with `pnpm api:materialize`.',
);
process.exitCode = 0;
