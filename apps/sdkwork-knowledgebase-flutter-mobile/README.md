# sdkwork-knowledgebase-flutter-mobile

Flutter mobile application root for SDKWork Knowledgebase.

The root owns the Flutter application entrypoint, the bootstrap/composition
layer, runtime environment projection, and the package family under
`packages/`. It exposes the SDKWork Knowledgebase route identities:

- `app.intelligence.knowledgebase.list`
- `app.intelligence.knowledgebase.detail`
- `app.intelligence.knowledgebase.search`
- `app.intelligence.knowledgebase.settings`
- `app.intelligence.knowledgebase.launch`

Those ids stay aligned with the PC, H5, mini program, and HarmonyOS roots
(`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7); physical Dart route
names may differ.

Authority: `FLUTTER_APP_MOBILE_ARCHITECTURE_SPEC.md`.
