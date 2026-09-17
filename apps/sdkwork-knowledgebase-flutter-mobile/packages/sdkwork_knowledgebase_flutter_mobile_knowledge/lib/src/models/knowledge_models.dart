/// View models, screen models, and route params for the knowledge capability.
///
/// The generated Dart app SDK target is not emitted yet, so these stay view
/// models mapped structurally from the app-api envelope. They are never
/// transport DTOs.
class KnowledgebaseListItem {
  const KnowledgebaseListItem({
    required this.id,
    required this.name,
    required this.description,
    required this.documentCount,
  });

  final String id;
  final String name;
  final String description;
  final int documentCount;
}

class KnowledgebasePage {
  const KnowledgebasePage({
    required this.items,
    required this.page,
    required this.hasMore,
  });

  final List<KnowledgebaseListItem> items;
  final int page;
  final bool hasMore;
}

class KnowledgebaseDocument {
  const KnowledgebaseDocument({
    required this.id,
    required this.knowledgebaseId,
    required this.title,
    required this.preview,
  });

  final String id;
  final String knowledgebaseId;
  final String title;
  final String preview;
}

class KnowledgebaseSearchHit {
  const KnowledgebaseSearchHit({
    required this.documentId,
    required this.knowledgebaseId,
    required this.title,
    required this.snippet,
    required this.score,
  });

  final String documentId;
  final String knowledgebaseId;
  final String title;
  final String snippet;
  final double score;
}

class KnowledgebaseRouteParams {
  const KnowledgebaseRouteParams({this.documentId, this.ticket});

  final String? documentId;
  final String? ticket;
}

/// Group-space launch request accepted from IM.
///
/// Authority: the group-knowledgebase boundary in
/// `../sdkwork-im/AGENTS.md`. Only the opaque single-use ticket crosses the
/// boundary; space identifiers, destinations, session tokens, and caller
/// context are never accepted from IM URLs.
class GroupKnowledgebaseLaunchRequest {
  const GroupKnowledgebaseLaunchRequest({required this.ticket});

  final String ticket;
}
