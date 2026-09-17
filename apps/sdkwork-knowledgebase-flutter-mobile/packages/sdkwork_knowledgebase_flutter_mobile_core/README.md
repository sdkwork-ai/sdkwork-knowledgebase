# sdkwork_knowledgebase_flutter_mobile_core

SDKWork Knowledgebase Flutter mobile core: runtime config, SDK port factories,
credential boundary, session stores, host adapter contracts, and composition
metadata.

The generated Dart app SDK targets of `sdkwork-drive-app-sdk`,
`sdkwork-iam-app-sdk`, and `sdkwork-knowledgebase-app-sdk` are not emitted by
the SDK generation chain yet, so this package declares the injected port
contracts and the adapter seams instead of vendoring transport copies
(`APP_SDK_INTEGRATION_SPEC.md`).

Authority: `../../../../../sdkwork-specs/FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md`.
