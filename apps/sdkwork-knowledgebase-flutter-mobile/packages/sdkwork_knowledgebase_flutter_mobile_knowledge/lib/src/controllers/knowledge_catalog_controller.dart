import 'package:flutter/foundation.dart';

import '../models/knowledge_models.dart';
import '../services/knowledge_catalog_service.dart';
import '../state/knowledge_catalog_state.dart';

/// Presentation controller for the knowledgebase catalog.
///
/// Owns UI state mapping and calls services only.
class KnowledgebaseCatalogController extends ChangeNotifier {
  KnowledgebaseCatalogController({required this.service});

  final KnowledgebaseCatalogService service;
  KnowledgebaseCatalogState _state = initialKnowledgebaseCatalogState();

  KnowledgebaseCatalogState get state => _state;

  Future<void> load({int page = 1}) async {
    _state = _state.copyWith(loading: true, errorMessage: '');
    notifyListeners();
    try {
      final result = await service.loadPage(page: page);
      _state = _state.copyWith(
        items: result.items,
        page: result.page,
        hasMore: result.hasMore,
        loading: false,
        errorMessage: '',
      );
    } catch (error) {
      _state = _state.copyWith(
        items: const <KnowledgebaseListItem>[],
        loading: false,
        errorMessage: error.toString(),
      );
    }
    notifyListeners();
  }
}
