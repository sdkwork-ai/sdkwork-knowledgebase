# sdkwork-knowledgebase-drive

Domain: drive
Capability: knowledgebase-drive
Package type: rust-crate
Status: standard

This README is the SDKWork module entrypoint for `sdkwork-knowledgebase-drive`. The machine-readable component contract is `specs/component.spec.json`; canonical standards are under `../../../sdkwork-specs/`.

## Public API

- `.`

## Required In-Process Surface

- Drive service crates (`sdkwork-drive-workspace-service`, `sdkwork-drive-object-runtime`) for root-scoped metadata and bounded immutable bytes.
- The adapters open the shared authoritative PostgreSQL pool and call Drive's typed services directly; there is no Drive Internal API client and no service credential (MODULE_SPEC.md §4.2).

## Configuration

Configuration keys, runtime entrypoints, and integration contracts are declared in `specs/component.spec.json`. Shared modules must receive configuration through typed bootstrap or service boundaries rather than reading host-local environment state directly.

## SaaS/Private/Local Behavior

This component follows the deployment and runtime rules referenced by its `canonicalSpecs` entries. SaaS, private, and local behavior must stay compatible with the relevant SDKWork specs before implementation changes are made.

## Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module. Protected API and SDK access must use the generated SDK or approved service boundary declared in the component contract.

## Extension Points

Extension points are limited to public exports, runtime entrypoints, SDK clients, events, and config keys declared in `specs/component.spec.json`.

## Verification

- `cargo test -p sdkwork-knowledgebase-drive`
- `node ../sdkwork-specs/tools/check-component-port-bindings.mjs --root .`

## Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`. Update that contract before changing public integration behavior.
