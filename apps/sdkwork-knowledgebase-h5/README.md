# SDKWork Knowledgebase H5

H5 mobile browser application root for SDKWork Knowledgebase, plus the
Capacitor iOS/Android host profile.

## Role

- Architecture: `h5` (`APP_H5_ARCHITECTURE_SPEC.md`).
- Package segment: `h5`; packages use the `@sdkwork/knowledgebase-h5-*` npm scope.
- Runnable surface: browser SPA mounted at the shared-origin `h5` slot
  (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` 2.1), built into
  `dist/<profile>/<envAlias>/`.

## Capability Surface

Aligned with the PC workbench (`apps/sdkwork-knowledgebase-pc`) by route
identity, service contract, and SDK boundary, not by identical physical paths:

| Route id | Screen |
| --- | --- |
| `app.intelligence.knowledgebase.list` | Knowledge space list and market |
| `app.intelligence.knowledgebase.detail` | Document tree and document body |
| `app.intelligence.knowledgebase.search` | Retrieval |
| `app.intelligence.knowledgebase.settings` | Knowledge space settings |
| `app.intelligence.knowledgebase.launch` | Group knowledgebase launch ticket |

## Packages

| Package | Role |
| --- | --- |
| `@sdkwork/knowledgebase-h5-core` | Runtime config, SDK client factories, token manager, session store, route registry, host adapter contracts |
| `@sdkwork/knowledgebase-h5-commons` | Domain-neutral mobile UI primitives and locale helpers |
| `@sdkwork/knowledgebase-h5-shell` | Mobile app shell, tab/stack navigation, AuthGate integration |
| `@sdkwork/knowledgebase-h5-knowledge` | Knowledgebase capability: pages, services, state, routes, i18n |

## Commands

```bash
pnpm --dir apps/sdkwork-knowledgebase-h5 dev
pnpm --dir apps/sdkwork-knowledgebase-h5 typecheck
pnpm --dir apps/sdkwork-knowledgebase-h5 build:dev
```

## Related Specs

- `../../../sdkwork-specs/APP_H5_ARCHITECTURE_SPEC.md`
- `../../../sdkwork-specs/APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`
- `../../../sdkwork-specs/APP_MOBILE_REACT_UI_SPEC.md`
- `../../../sdkwork-specs/APP_SDK_INTEGRATION_SPEC.md`
