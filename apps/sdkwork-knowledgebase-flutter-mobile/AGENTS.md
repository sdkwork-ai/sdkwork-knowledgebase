# Repository Guidelines

## SDKWORK Soul

Read `../../../sdkwork-specs/SOUL.md` before executing application tasks. Start with the sections that route the current task; related-spec references are not a startup bundle.

## SDKWORK Standards

The canonical standards index is `../../../sdkwork-specs/README.md`, and `../../../sdkwork-specs/AGENTS_SPEC.md` governs this entrypoint. Read the relevant task-matrix row first and do not copy global normative bodies locally.

## Application Identity

Read `sdkwork.app.config.json` only for application identity, SDK/API inventory, release metadata, packaging, or app-owned capabilities. Runtime values belong to source configuration, not to the application declaration.

- Application code: `knowledgebase`
- Application key: `sdkwork-knowledgebase-flutter-mobile`
- Package name: `com.sdkwork.knowledgebase.mobile`
- Client architecture: `flutter-mobile` (runtime target `flutter-android`/`flutter-ios`)

## Local Dictionary Structure

Use `AGENTS.md` as the application routing entrypoint. Read `.sdkwork/`, `specs/`, application source, tests, and documentation only when the current task reaches the contract each location governs.

- `lib/` is the Flutter application root: entrypoint, app shell scope, auth gate, and `lib/bootstrap/` composition.
- `packages/` owns the reusable Dart package family (`core`, `commons`, `shell`, `knowledge`).
- `config/` holds safe checked-in runtime templates; `env/` holds the materialized per-profile Dart-define payloads; `etc/` is this deployable root's source configuration and parent topology delegation authority.

## Spec Resolution Order

Use dynamic progressive loading: read this file and `../../AGENTS.md`, then applicable local contracts, then the relevant task route in `../../../sdkwork-specs/README.md`, and only afterward inspect implementation files. Language-specific specs load on demand only.

## Required Specs By Task Type

Code changes load `../../../sdkwork-specs/CODE_STYLE_SPEC.md`, `../../../sdkwork-specs/NAMING_SPEC.md`, and only the touched frontend authority such as `../../../sdkwork-specs/FRONTEND_CODE_SPEC.md`. Flutter client work loads `../../../sdkwork-specs/APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`, `../../../sdkwork-specs/FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md`, and `../../../sdkwork-specs/APP_FLUTTER_UI_SPEC.md`. Package-command work loads `../../../sdkwork-specs/PNPM_SCRIPT_SPEC.md`; source configuration work loads `../../../sdkwork-specs/SOURCE_CONFIG_SPEC.md`; locale resource work loads `../../../sdkwork-specs/I18N_SPEC.md`.

## Code Style Rules

Consume remote capabilities through composed SDK ports and application-owned bootstrap adapters. Do not introduce raw HTTP, manual authentication headers, generated transport imports, local SDK forks, or duplicated shared utilities. Capability packages must not construct SDK clients or read runtime environment values directly.

## Build, Test, and Verification

Flutter builds require the Flutter SDK, which is not part of the repository workspace; run the analyzer for Dart changes and keep static repository verification green.

```bash
flutter analyze
flutter test
```

Static repository verification (runs without the Flutter toolchain):

```bash
node ../../sdkwork-specs/tools/verify-repo.mjs --root ../..
node ../../sdkwork-specs/tools/check-frontend-composition.mjs --root ../..
node ../../sdkwork-specs/tools/check-component-port-bindings.mjs --root ../..
```

Packaging workflow work loads `../../../sdkwork-specs/GITHUB_WORKFLOW_SPEC.md`; source configuration work loads `../../../sdkwork-specs/SOURCE_CONFIG_SPEC.md`; list/search pagination work loads `../../../sdkwork-specs/PAGINATION_SPEC.md` and runs `check-pagination.mjs`. Language-specific specs are on-demand only.

## Agent Execution Rules

Follow specifications before memory and evidence before completion. Keep SDK construction, authentication, environment selection, and host capabilities in their owning composition layers.

## Task-Specific Standards

SDK consumer work loads `../../../sdkwork-specs/APP_SDK_INTEGRATION_SPEC.md` and runs `check-app-sdk-consumer-imports.mjs`. API work loads `../../../sdkwork-specs/API_SPEC.md` and its validators. List/search work loads `../../../sdkwork-specs/PAGINATION_SPEC.md`.

## Human Review Rules

Human review is required for public API changes, security exceptions, database migrations, generated SDK ownership changes, destructive operations, and cross-application standards changes.
