# Repository Guidelines

<!-- SDKWORK-AGENTS-GENERATED: v2 -->

## SDKWORK Soul

Read `../../../sdkwork-specs/SOUL.md` before executing tasks in this root. Follow specs before memory, dictionary before context, stop on ambiguity, and evidence before completion.

## SDKWORK Standards

<!-- SDKWORK-PROGRESSIVE-LOADING: v1 -->
Resolve this standards root once and use it as the global authority for the current task:

- `../../../sdkwork-specs/README.md`
- `../../../sdkwork-specs/SOUL.md`
- `../../../sdkwork-specs/AGENTS_SPEC.md`

Read only the relevant README task-matrix row or navigation heading, then load the selected authority sections.
<!-- /SDKWORK-PROGRESSIVE-LOADING: v1 -->

Do not copy root standard text into this application root. If these relative paths do not resolve, stop and report the broken workspace layout.

## Application Identity

Read `sdkwork.app.config.json` only when the task touches H5 application behavior, runtime config, SDK wiring, release metadata, app-owned capabilities, packaging, or deployment. For unrelated package-local work, do not expand into the full app manifest unless evidence requires it.

## Local Dictionary Structure

- `AGENTS.md`: H5 application root agent entrypoint and relative SDKWork spec index.
- `CLAUDE.md`, `GEMINI.md`, `CODEX.md`: compatibility shims that point to `AGENTS.md` and must not duplicate rules.
- `sdkwork.app.config.json`: H5 application identity, runtime, release, and capability metadata.
- `etc/`: surface-owned source configuration that delegates to the repository deployment authority.
- `config/browser/`: browser-visible public runtime config templates per deployment profile.
- `config/host/`: Capacitor host packaging metadata and permission references.
- `specs/`: local application contracts and narrowing rules.
- `src/`: thin bootstrap, providers, route assembly, and shell registration only.
- `packages/`: core, commons, shell, and capability packages.
- `public/`: static runtime assets such as the materialized `runtime-env.json`.
- `tests/`: application-level architecture, config, and route verification.
- `package.json`, `vite.config.ts`, `tsconfig.json`: build manifests.

## Spec Resolution Order

<!-- SDKWORK-PROGRESSIVE-LOADING: v1 -->
Use dynamic progressive loading for the current task: resolve the selected root and task category before reading broad source context.

1. Read this `AGENTS.md` routing material and classify the owned surface.
2. Read `sdkwork.app.config.json`, local `specs/`, and `.sdkwork/` only when the task reaches the contract each item governs.
3. Locate only the relevant task-matrix row or navigation heading in `../../../sdkwork-specs/README.md`; do not load the full catalog.
4. Read only the task-specific global spec sections selected by that route, then inspect implementation files.
<!-- /SDKWORK-PROGRESSIVE-LOADING: v1 -->

## Required Specs By Task Type

- Agent/workflow changes: `../../../sdkwork-specs/SOUL.md`, `../../../sdkwork-specs/AGENTS_SPEC.md`, `../../../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`, `../../../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`, and `../../../sdkwork-specs/TEST_SPEC.md`.
- Package script changes: `../../../sdkwork-specs/PNPM_SCRIPT_SPEC.md`, `../../../sdkwork-specs/APP_RUNTIME_TOPOLOGY_SPEC.md`, `../../../sdkwork-specs/CONFIG_SPEC.md`, and `../../../sdkwork-specs/TEST_SPEC.md`.
- Any code change: `../../../sdkwork-specs/CODE_STYLE_SPEC.md`, `../../../sdkwork-specs/NAMING_SPEC.md`, plus only the touched language/framework spec.
- TypeScript/Node code: `../../../sdkwork-specs/TYPESCRIPT_CODE_SPEC.md`.
- Frontend/UI code: `../../../sdkwork-specs/FRONTEND_CODE_SPEC.md`, `../../../sdkwork-specs/FRONTEND_SPEC.md`, `../../../sdkwork-specs/UI_ARCHITECTURE_SPEC.md`, and `../../../sdkwork-specs/APP_MOBILE_REACT_UI_SPEC.md`.
- API/SDK changes: `../../../sdkwork-specs/API_SPEC.md`, `../../../sdkwork-specs/WEB_FRAMEWORK_SPEC.md`, `../../../sdkwork-specs/WEB_BACKEND_SPEC.md`, `../../../sdkwork-specs/SDK_SPEC.md`, `../../../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md`, and `../../../sdkwork-specs/TEST_SPEC.md`.
- Runtime/deployment/release changes: `../../../sdkwork-specs/CONFIG_SPEC.md`, `../../../sdkwork-specs/ENVIRONMENT_SPEC.md`, `../../../sdkwork-specs/DEPLOYMENT_SPEC.md`, `../../../sdkwork-specs/RELEASE_SPEC.md`, `../../../sdkwork-specs/SUPPLY_CHAIN_SECURITY_SPEC.md`, and `../../../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`.
- Security/auth changes: `../../../sdkwork-specs/IAM_SPEC.md`, `../../../sdkwork-specs/IAM_LOGIN_INTEGRATION_SPEC.md`, `../../../sdkwork-specs/SECURITY_SPEC.md`, and `../../../sdkwork-specs/PRIVACY_SPEC.md`.

H5 architecture and cross-client alignment specs: `../../../sdkwork-specs/APP_H5_ARCHITECTURE_SPEC.md` and `../../../sdkwork-specs/APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`.

Language-specific specs load on-demand; do not load Dart, ArkTS, or backend specs for an H5-only task.

## Code Style Rules

- `src/` stays thin: bootstrap, providers, route assembly, shell registration, AuthGate wiring, environment selection, SDK client construction, IAM runtime wiring, and host adapter registration only.
- Business screens, services, state, routes, and i18n live in `packages/`.
- Capability packages import SDK clients and ports through `@sdkwork/knowledgebase-h5-core` public exports; raw transport construction is forbidden outside bootstrap/core.
- Cross-package imports use package root exports, never private source paths.

## Build, Test, and Verification

```bash
pnpm --dir apps/sdkwork-knowledgebase-h5 typecheck
pnpm --dir apps/sdkwork-knowledgebase-h5 build:dev
pnpm --dir apps/sdkwork-knowledgebase-h5 test
```

Repository-wide gates: `node ../../sdkwork-specs/tools/verify-repo.mjs --root .` from the repository root.

Packaging workflow work loads `../../../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`; source configuration work loads `../../../sdkwork-specs/SOURCE_CONFIG_SPEC.md`; list/search pagination work loads `../../../sdkwork-specs/PAGINATION_SPEC.md` and runs `check-pagination.mjs`. Language-specific specs are on-demand only.

## Agent Execution Rules

- Evidence before completion: paste the command and its exit code for every claim.
- Stop on ambiguity instead of inventing a contract.
- Never hand-edit generated SDK transport output.

## Human Review Rules

- Human review is required before changing `sdkwork.app.config.json`, `etc/`, or any deployment profile.
- Human review is required before widening the capability surface beyond the PC-aligned route identity set.
