# apps/

Application: sdkwork-knowledgebase
Status: active
Owner: SDKWork maintainers
Specs: APPLICATION_SPEC.md, SDKWORK_WORKSPACE_SPEC.md

## Primary App Surface

The repository root is the primary runnable app surface.
The repository root `sdkwork.app.config.json` governs the primary application manifest.

## Directory Index

| Directory | Surface role | Runnable | Purpose | Entry |
| --- | --- | --- | --- | --- |
| sdkwork-knowledgebase-common | common | no | sdkwork-knowledgebase-common common application root. | `sdkwork-knowledgebase-common/` |
| sdkwork-knowledgebase-flutter-mobile | flutter-mobile | yes | SDKWork Knowledgebase Mobile flutter-mobile application root. | [README](sdkwork-knowledgebase-flutter-mobile/README.md) |
| sdkwork-knowledgebase-h5 | h5 | yes | SDKWork Knowledgebase H5 h5 application root. | [README](sdkwork-knowledgebase-h5/README.md) |
| sdkwork-knowledgebase-harmony-mobile | harmony-mobile | yes | SDKWork Knowledgebase HarmonyOS Mobile harmony-mobile application root. | [README](sdkwork-knowledgebase-harmony-mobile/README.md) |
| sdkwork-knowledgebase-mini-program | mini-program | yes | SDKWork Knowledgebase Mini Program mini-program application root. | [README](sdkwork-knowledgebase-mini-program/README.md) |
| sdkwork-knowledgebase-pc | pc | yes | SDKWork Knowledgebase PC pc application root. | [README](sdkwork-knowledgebase-pc/README.md) |

## Allowed Content

- Selected language/architecture application roots with `README.md`, `AGENTS.md`, `.sdkwork/`, and `specs/` when authored packages exist.
- Architecture-local `packages/`, `config/`, `src/`, `lib/`, `App/`, or `entry/` directories required by the owning architecture standard.

## Forbidden Content

- Repository-root API contracts, generated SDK workspaces, Rust crates, or deployment descriptors moved under `apps/`.
- Runtime secrets, user-private state, generated SDK transport output, or cross-application copied business logic.

## Related Specs

- `../sdkwork-specs/APPLICATION_SPEC.md`
- `../sdkwork-specs/SDKWORK_WORKSPACE_SPEC.md`
- `../sdkwork-specs/APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`

## Verification

```bash
node ../sdkwork-specs/tools/check-apps-directory-index.mjs --root .
```
