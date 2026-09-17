/// Module registry.
///
/// The root bootstrap reads this registry instead of importing capability
/// packages directly.
class KnowledgebaseModuleRegistration {
  const KnowledgebaseModuleRegistration({
    required this.id,
    required this.routeIdPrefix,
  });

  final String id;
  final String routeIdPrefix;
}

List<KnowledgebaseModuleRegistration> listKnowledgebaseModules() {
  return const <KnowledgebaseModuleRegistration>[
    KnowledgebaseModuleRegistration(
      id: 'knowledge',
      routeIdPrefix: 'app.intelligence.knowledgebase.',
    ),
  ];
}
