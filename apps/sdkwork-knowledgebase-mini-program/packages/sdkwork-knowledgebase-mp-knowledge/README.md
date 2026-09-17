# @sdkwork/knowledgebase-mp-knowledge

Knowledgebase capability package for the SDKWork Knowledgebase mini program root.

## Public Exports

- `.`: models, services, state, and screen-state resolvers.
- `./routes`: route contributions projected into `pages` / `subPackages`.
- `./i18n`: thin locale fragment aggregator.

## Boundaries

Services receive SDK ports from `@sdkwork/knowledgebase-mp-core`; raw `wx.request`
transport, manual credential headers, and local SDK forks are forbidden.
