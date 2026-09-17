# sdkwork-knowledgebase-harmony-mobile

HarmonyOS native (ArkTS/ArkUI) mobile application root for SDKWork
Knowledgebase.

The root owns the installable HAP entry module, the bootstrap/composition
layer, runtime environment projection, and the HAR package family under
`packages/`. It exposes the SDKWork Knowledgebase route identities:

- `app.intelligence.knowledgebase.list`
- `app.intelligence.knowledgebase.detail`
- `app.intelligence.knowledgebase.search`
- `app.intelligence.knowledgebase.settings`
- `app.intelligence.knowledgebase.launch`

Those ids stay aligned with the PC, H5, mini program, and Flutter roots
(`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7); physical Harmony page
paths may differ.

HarmonyOS builds require DevEco Studio or a compatible HarmonyOS SDK,
`hvigor`, and `ohpm` plus a documented signing profile. Those toolchains are
not part of the repository workspace and are tracked as pending integration;
static repository verification runs without them.

Authority: `HARMONY_APP_MOBILE_ARCHITECTURE_SPEC.md`.
