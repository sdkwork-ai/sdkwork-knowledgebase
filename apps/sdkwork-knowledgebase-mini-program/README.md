# SDKWork Knowledgebase Mini Program

WeChat mini program application root for SDKWork Knowledgebase.

## Role

- Architecture: `mini-program` (`MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md`).
- Package segment: `mp`; packages use the `@sdkwork/knowledgebase-mp-*` npm scope.
- Platform: `MP_WEIXIN`. Device-class surface: packaging consumes the platform
  store and keeps the framework-default output directory.

## Capability Surface

Aligned with the PC workbench by route identity and service contract:

| Route id | Page |
| --- | --- |
| `app.intelligence.knowledgebase.list` | `pages/knowledgebase/index` |
| `app.intelligence.knowledgebase.detail` | `pages/knowledgebase-detail/index` |
| `app.intelligence.knowledgebase.search` | `pages/knowledgebase-search/index` |
| `app.intelligence.knowledgebase.settings` | `pages/knowledgebase-detail/index` |
| `app.intelligence.knowledgebase.launch` | `pages/group-launch/index` |

Physical `pages`/`subPackages` entries are projected from route
contributions by `@sdkwork/knowledgebase-mp-shell`; the contributions stay the source.

## Packages

| Package | Role |
| --- | --- |
| `@sdkwork/knowledgebase-mp-core` | Runtime config, SDK client factory, session store, composition entry |
| `@sdkwork/knowledgebase-mp-commons` | Domain-neutral design tokens, screen-state primitives, locale helpers |
| `@sdkwork/knowledgebase-mp-shell` | Navigation container, AuthGate decisions, page projection |
| `@sdkwork/knowledgebase-mp-host` | wx platform adapter contracts |
| `@sdkwork/knowledgebase-mp-knowledge` | Knowledgebase capability: services, state, routes, i18n |

## Commands

```bash
pnpm --dir apps/sdkwork-knowledgebase-mini-program typecheck
pnpm --dir apps/sdkwork-knowledgebase-mini-program build:mini-program
```

## Related Specs

- `../../../sdkwork-specs/MINI_PROGRAM_APP_ARCHITECTURE_SPEC.md`
- `../../../sdkwork-specs/APP_MINI_PROGRAM_UI_SPEC.md`
- `../../../sdkwork-specs/APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md`
