import 'package:sdkwork_knowledgebase_flutter_mobile_knowledge/sdkwork_knowledgebase_flutter_mobile_knowledge.dart';

/// Route registry assembly.
///
/// Route ids follow `<surface>.<domain>.<capability>.<screen>` and stay aligned
/// with the PC, H5, mini program, and HarmonyOS roots
/// (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7). Physical Dart route
/// names may differ from other platforms while route ids stay stable.
List<KnowledgebaseRouteRegistration> createRoutes() {
  return createKnowledgebaseRouteRegistry(knowledgebaseRouteContributions);
}
