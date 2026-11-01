# SDKWork Knowledgebase PC

PC browser and optional Tauri desktop surface for SDKWork Knowledgebase.

Architecture: `../../sdkwork-specs/APP_PC_ARCHITECTURE_SPEC.md`, `../../sdkwork-specs/APP_PC_REACT_UI_SPEC.md`.

Launch acceptance: `../../docs/product/prd/PRD-mvp-launch.md`.

## Packages

| Package | Role |
| --- | --- |
| Root Vite app (`src/`) | Bootstrap, IAM shell, routing |
| `packages/sdkwork-knowledgebase-pc-core` | Session, runtime config, SDK registry, auth gate |
| `packages/sdkwork-knowledgebase-pc-commons` | Shared UI utilities, sanitizers, error boundaries |
| `packages/sdkwork-knowledgebase-pc-shell` | App shell, settings, global navigation |
| `packages/sdkwork-knowledgebase-pc-knowledgebase` | Editor, drive browser, WeChat, market |
| `packages/sdkwork-knowledgebase-pc-search` | Search and media viewer module |
| `packages/sdkwork-knowledgebase-pc-desktop` | Tauri desktop host |
| `packages/sdkwork-knowledgebase-pc-knowledge` | Generic host-managed embed components and the standalone managed-group launch page |

Import rule: use package names (`@sdkwork/sdkwork-knowledgebase-pc-*`). Capability packages import SDK contract types and clients only through `sdkwork-knowledgebase-pc-core` (`./sdk` export). Do not use `@packages/` deep paths or raw HTTP for business APIs.

## Config examples

| File | Profile |
| --- | --- |
| `config/browser/runtime-env.development.example.json` | Local browser dev |
| `config/browser/runtime-env.staging.example.json` | Staging (`enableDemoMode: false`) |
| `config/browser/runtime-env.production.example.json` | Production (`enableDemoMode: false`) |
| `config/desktop/sdkwork-knowledgebase-pc.development.toml.example` | Desktop dev host |
| `config/desktop/sdkwork-knowledgebase-pc.production.toml.example` | Desktop production host |

Repository-root `config/host/postgres.env` supplies development database and IAM signing secrets (copy from `../../config/host/postgres.env.example`).

## Prerequisites

- Node.js 22+
- pnpm 10+
- Rust toolchain (desktop builds and backend dev orchestration)

Install dependencies once from the **repository root** (`pnpm install`). The PC surface participates in the root `pnpm-workspace.yaml` per `APP_COMPOSITION_SPEC.md`.

## Local development

From the repository root:

```powershell
pnpm dev:browser
pnpm dev:desktop
```

Or from this directory (after root `pnpm install`):

```powershell
cp .env.example .env.local
pnpm dev
```

## Composition (APP_COMPOSITION_SPEC)

| Authority | Location |
| --- | --- |
| Workspace graph | Repository root `pnpm-workspace.yaml` |
| Frontend SDK inventory | `packages/sdkwork-knowledgebase-pc-core/specs/component.spec.json#contracts.sdkDependencies` |
| Runtime SDK base URLs | `packages/sdkwork-knowledgebase-pc-core/src/composition/dependency-runtime.ts` |

Gate: `pnpm check:app-composition` from repository root.

## SDK integration

- App API: `@sdkwork/knowledgebase-app-sdk` (composed facade over `../../sdks/sdkwork-knowledgebase-app-sdk/`)
- Drive: `@sdkwork/drive-app-sdk`
- Auth: `@sdkwork/auth-pc-react`, `@sdkwork/auth-runtime-pc-react` (Appbase IAM)
- String helpers: `@sdkwork/utils` (`isBlank`, `trim`) — import directly; do not add local re-export shims
- Do not call backend HTTP APIs directly from app packages.

## Generic host embedding

- Export package: `@sdkwork/knowledgebase-pc-knowledge`
- Hosts may configure generic runtime ports through `configureKnowledgebasePcRuntime({ sdkPorts })`
  for non-managed integrations.
- This generic component surface is not an IM group knowledgebase entrypoint. It must not receive a
  group id, space id, launch ticket, or session authority for managed group workspaces.
- Managed IM groups launch the complete standalone Knowledgebase browser route or the independent
  Knowledgebase Tauri process through the server-authorized group-launch flow. They never use an
  IM iframe, an IM-owned native window, or `openHostKnowledgeWindow`.

Production builds must not rely on demo/mock fallbacks (`VITE_SDKWORK_KNOWLEDGEBASE_ENABLE_DEMO_MODE=false` or unset in production).

## Verification

From repository root:

```powershell
pnpm check
pnpm check:pc-app-hygiene
pnpm check:utils-integration
pnpm lint
pnpm test:frontend
pnpm verify
pnpm test:e2e:playwright
```

From this directory:

```powershell
pnpm lint
pnpm test:e2e
```

## Hygiene

One-off migration scripts (`update_*.cjs`, `fix_*.js`, etc.) are forbidden at the app root. Use package tests, i18n tooling, and standard build scripts only. Enforced by `../../tools/check_pc_app_hygiene.mjs`.
