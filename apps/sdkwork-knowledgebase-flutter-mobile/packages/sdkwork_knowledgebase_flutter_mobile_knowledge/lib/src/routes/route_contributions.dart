import 'package:sdkwork_knowledgebase_flutter_mobile_shell/sdkwork_knowledgebase_flutter_mobile_shell.dart';

/// Route contributions for the knowledge capability.
///
/// Route ids follow `<surface>.<domain>.<capability>.<screen>` and stay aligned
/// with the PC, H5, mini program, and HarmonyOS roots
/// (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7). Physical Dart route
/// names may differ while route ids stay stable. Route metadata must not
/// declare HTTP API paths, SDK methods, or transport details.
const List<SdkworkKnowledgebaseRouteRegistration> knowledgebaseRouteContributions =
    <SdkworkKnowledgebaseRouteRegistration>[
  SdkworkKnowledgebaseRouteRegistration(
    id: 'app.intelligence.knowledgebase.list',
    routeName: '/knowledgebase',
    titleKey: 'knowledgebase.list.title',
    authRequired: true,
  ),
  SdkworkKnowledgebaseRouteRegistration(
    id: 'app.intelligence.knowledgebase.detail',
    routeName: '/knowledgebase/:documentId',
    titleKey: 'knowledgebase.detail.title',
    authRequired: true,
  ),
  SdkworkKnowledgebaseRouteRegistration(
    id: 'app.intelligence.knowledgebase.search',
    routeName: '/knowledgebase/search',
    titleKey: 'knowledgebase.search.title',
    authRequired: true,
  ),
  SdkworkKnowledgebaseRouteRegistration(
    id: 'app.intelligence.knowledgebase.settings',
    routeName: '/knowledgebase/settings',
    titleKey: 'knowledgebase.settings.title',
    authRequired: true,
  ),
  SdkworkKnowledgebaseRouteRegistration(
    id: 'app.intelligence.knowledgebase.launch',
    routeName: '/knowledgebase/group-launch/:ticket',
    titleKey: 'knowledgebase.launch.title',
    authRequired: true,
  ),
];
