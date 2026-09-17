/// Named-route registry assembly for the Flutter mobile shell.
///
/// Route ids follow `<surface>.<domain>.<capability>.<screen>` and stay
/// aligned with the PC, H5, mini program, and HarmonyOS roots. Physical
/// Flutter route names may differ while route ids stay stable.
class SdkworkKnowledgebaseRouteRegistration {
  const SdkworkKnowledgebaseRouteRegistration({
    required this.id,
    required this.routeName,
    required this.titleKey,
    required this.authRequired,
  });

  final String id;
  final String routeName;
  final String titleKey;
  final bool authRequired;
}

List<SdkworkKnowledgebaseRouteRegistration> createSdkworkKnowledgebaseRouteRegistry(
  List<SdkworkKnowledgebaseRouteRegistration> routes,
) {
  return routes
      .where((route) => route.routeName.isNotEmpty)
      .toList(growable: false);
}
