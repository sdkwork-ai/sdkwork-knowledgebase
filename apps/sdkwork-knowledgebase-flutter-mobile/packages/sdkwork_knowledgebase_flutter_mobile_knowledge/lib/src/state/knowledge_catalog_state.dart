import '../models/knowledge_models.dart';

/// Package-local state slice for the knowledgebase catalog.
///
/// Sensitive state must clear on logout and account/tenant switch.
class KnowledgebaseCatalogState {
  const KnowledgebaseCatalogState({
    required this.page,
    required this.items,
    required this.hasMore,
    required this.loading,
    required this.errorMessage,
  });

  final int page;
  final List<KnowledgebaseListItem> items;
  final bool hasMore;
  final bool loading;
  final String errorMessage;

  KnowledgebaseCatalogState copyWith({
    int? page,
    List<KnowledgebaseListItem>? items,
    bool? hasMore,
    bool? loading,
    String? errorMessage,
  }) {
    return KnowledgebaseCatalogState(
      page: page ?? this.page,
      items: items ?? this.items,
      hasMore: hasMore ?? this.hasMore,
      loading: loading ?? this.loading,
      errorMessage: errorMessage ?? this.errorMessage,
    );
  }
}

KnowledgebaseCatalogState initialKnowledgebaseCatalogState() {
  return const KnowledgebaseCatalogState(
    page: 1,
    items: <KnowledgebaseListItem>[],
    hasMore: false,
    loading: true,
    errorMessage: '',
  );
}
