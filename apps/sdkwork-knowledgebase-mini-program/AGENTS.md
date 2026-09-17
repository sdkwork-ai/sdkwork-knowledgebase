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

Read `sdkwork.app.config.json` only when the task touches mini program behavior, runtime config, SDK wiring, release metadata, packaging, or deployment. For unrelated package-local work, do not expand into the full app manifest unless evidence requires it.

## Local Dictionary Structure

- `AGENTS.md`: mini program root agent entrypoint and relative SDKWork spec index.
- `CLAUDE.md`, `GEMINI.md`, `CODEX.md`: compatibility shims that point to `AGENTS.md` and must not duplicate rules.
- `sdkwork.app.config.json`: mini program identity, runtime, release, and capability metadata.
- `project.config.json`: WeChat developer tool project descriptor (`miniprogramRoot: src/`).
- `etc/`: surface-owned source configuration that delegates to the repository deployment authority.
- `config/mini-program/`: runtime env materialization targets per deployment profile.
- `config/host/`: platform host packaging metadata and permission references.
- `specs/`: local application contracts and narrowing rules.
- `src/`: thin bootstrap, runtime bundle entry, and platform page shells.
- `src/runtime/`: generated runtime bundle output (`build-runtime.mjs`).
- `packages/`: core, commons, shell, host, and capability packages.
- `scripts/`: surface-local build helpers.
- `tests/`: application-level architecture and route verification.

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
- Frontend/UI code: `../../../sdkwork-specs/FRONTEND_CODE_SPEC.md`, `../../../sdkwork-specs/FRONTEND_SPEC.md`, `../../../sdkwork-specs/UI_ARCHITECTURE_SPEC.md`, and `../../../sdkwork-specs/APP_MINI_PROGRAM_UI_SPEC.md`.
- API/SDK changes: `../../../sdkwork-specs/API_SPEC.md`, `../../../sdkwork-specs/WEB_FRAMEWORK_SPEC.md`, `../../../sdkwork-specs/WEB_BACKEND_SPEC.md`, `../../../sdkwork-specs/SDK_SPEC.md`, `../../../sdkwork-specs/SDK_WORKSPACE_GENERATION_SPEC.md`, and `../../../sdkwork-specs/TEST_SPEC.md`.
- Runtime/deployment/release changes: `../../../sdkwork-specs/CONFIG_SPEC.md`, `../../../sdkwork-specs/ENVIRONMENT_SPEC.md`, `../../../sdkwork-specs/DEPLOYMENT_SPEC.md`, `../../../sdkwork-specs/RELEASE_SPEC.md`, `../../../sdkwork-specs/SUPPLY_CHAIN_SECURITY_SPEC.md`, and `../../../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`.
- Security/auth changes: `../../../sdkwork-specs/IAM_SPEC.md`, `../../../sdkwork-specs/IAM_LOGIN_INTEGRATION_SPEC.md`, `../../../sdkwork-specs/SECURITY_SPEC.md`, and `../../../sdkwork-specs/PRIVACY_SPEC.md`.

Mini program architecture spec: `../../../sdkwork-specs/MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md`.

Language-specific specs load on-demand; do not load Dart, ArkTS, or backend specs for a mini-program-only task.

## Code Style Rules

- `src/` stays thin: app registration, runtime bundle entry, and platform page shells only.
- Business pages, services, state, routes, and i18n live in `packages/`.
- Capability packages import SDK clients through `@sdkwork/knowledgebase-mp-core` public exports; raw `wx.request` transport is forbidden.
- Cross-package imports use package root exports, never private source paths.

## Build, Test, and Verification

```bash
pnpm --dir apps/sdkwork-knowledgebase-mini-program typecheck
pnpm --dir apps/sdkwork-knowledgebase-mini-program build:mini-program
pnpm --dir apps/sdkwork-knowledgebase-mini-program test
```

Repository-wide gates: `node ../../sdkwork-specs/tools/verify-repo.mjs --root .` from the repository root.

Packaging workflow work loads `../../../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`; source configuration work loads `../../../sdkwork-specs/SOURCE_CONFIG_SPEC.md`; list/search pagination work loads `../../../sdkwork-specs/PAGINATION_SPEC.md` and runs `check-pagination.mjs`. Language-specific specs are on-demand only.

## Agent Execution Rules

- Evidence before completion: paste the command and its exit code for every claim.
- Stop on ambiguity instead of inventing a contract.
- Never hand-edit generated runtime bundles.

## Human Review Rules

- Human review is required before changing `sdkwork.app.config.json`, `etc/`, or any deployment profile.
- Human review is required before changing the projected `pages`/`subPackages` binding.
